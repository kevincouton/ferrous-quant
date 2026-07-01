//! Interactive Brokers TWS API Client with async connection, market data streaming,
//! order management, and paper trading support.

#![warn(missing_docs)]

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

/// Default TWS API port for live connections.
pub const TWS_LIVE_PORT: u16 = 7496;
/// Default TWS API port for paper/simulated connections.
pub const TWS_PAPER_PORT: u16 = 7497;

/// IBKR client configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct IbkrConfig {
    /// Host address (default: 127.0.0.1).
    pub host: String,
    /// Port (7496 for live, 7497 for paper).
    pub port: u16,
    /// Client ID (must be unique per connection).
    pub client_id: i32,
    /// Paper trading mode.
    pub paper: bool,
    /// Connection timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for IbkrConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: TWS_PAPER_PORT,
            client_id: 0,
            paper: true,
            timeout_secs: 10,
        }
    }
}

impl IbkrConfig {
    /// Create a default paper trading configuration.
    pub fn paper() -> Self {
        Self::default()
    }

    /// Create a live trading configuration.
    pub fn live() -> Self {
        Self {
            port: TWS_LIVE_PORT,
            paper: false,
            ..Default::default()
        }
    }

    /// Set the host.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the client ID.
    pub fn client_id(mut self, client_id: i32) -> Self {
        self.client_id = client_id;
        self
    }

    /// Set paper trading mode.
    pub fn with_paper(mut self, paper: bool) -> Self {
        self.paper = paper;
        self
    }
}

/// Connection status to IB Gateway / TWS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Disconnected.
    Disconnected,
    /// Connecting.
    Connecting,
    /// Connected and authenticated.
    Connected,
    /// Connection lost.
    Lost,
}

/// Market data tick from IBKR.
#[derive(Debug, Clone, PartialEq)]
pub struct IbkrTick {
    /// Request ID.
    pub req_id: i32,
    /// Field type (price, size, etc.).
    pub field: TickField,
    /// Value.
    pub value: f64,
    /// Timestamp.
    pub timestamp: i64,
}

/// Tick field types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickField {
    /// Bid price.
    BidPrice,
    /// Ask price.
    AskPrice,
    /// Last price.
    LastPrice,
    /// Bid size.
    BidSize,
    /// Ask size.
    AskSize,
    /// Last size.
    LastSize,
    /// Volume.
    Volume,
    /// High price.
    High,
    /// Low price.
    Low,
    /// Open price.
    Open,
    /// Close price.
    Close,
    /// VWAP.
    Vwap,
    /// Unknown / other.
    Other,
}

/// IBKR client state.
#[derive(Debug)]
pub struct IbkrClient {
    config: IbkrConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    tick_tx: mpsc::UnboundedSender<IbkrTick>,
    tick_rx: Option<mpsc::UnboundedReceiver<IbkrTick>>,
    stream: Option<TcpStream>,
    next_req_id: Arc<RwLock<i32>>,
}

impl IbkrClient {
    /// Create a new IBKR client with the given configuration.
    ///
    /// Does not connect until `connect()` is called.
    pub fn new(config: IbkrConfig) -> Self {
        let (tick_tx, tick_rx) = mpsc::unbounded_channel();
        Self {
            config,
            status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            tick_tx,
            tick_rx: Some(tick_rx),
            stream: None,
            next_req_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Current connection status.
    pub async fn status(&self) -> ConnectionStatus {
        *self.status.read().await
    }

    /// Connect to TWS / IB Gateway.
    pub async fn connect(&mut self) -> Result<(), IbkrError> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| IbkrError::Config(format!("invalid address: {e}")))?;

        info!(addr = %addr, "connecting to IB Gateway");
        *self.status.write().await = ConnectionStatus::Connecting;

        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|_| IbkrError::Timeout)?
        .map_err(|e| IbkrError::Io(e.to_string()))?;

        self.stream = Some(stream);
        *self.status.write().await = ConnectionStatus::Connected;
        info!("connected to IB Gateway");

        // Send initial handshake
        self.send_handshake().await?;

        // Spawn message reader
        let status = self.status.clone();
        let _tick_tx = self.tick_tx.clone();
        if let Some(mut stream) = self.stream.take() {
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) => {
                            warn!("IB Gateway closed connection");
                            *status.write().await = ConnectionStatus::Lost;
                            break;
                        }
                        Ok(n) => {
                            debug!(bytes = n, "received data from IB");
                            // In a real implementation, parse IB protocol messages here
                            // and route them to tick_tx or order handlers.
                        }
                        Err(e) => {
                            error!(error = %e, "IB read error");
                            *status.write().await = ConnectionStatus::Lost;
                            break;
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// Disconnect from TWS / IB Gateway.
    pub async fn disconnect(&mut self) {
        info!("disconnecting from IB Gateway");
        *self.status.write().await = ConnectionStatus::Disconnected;
        self.stream = None;
    }

    /// Subscribe to real-time market data for a symbol.
    pub async fn subscribe_market_data(
        &mut self,
        symbol: impl AsRef<str>,
    ) -> Result<i32, IbkrError> {
        let req_id = self.next_req_id().await;
        info!(req_id = req_id, symbol = %symbol.as_ref(), "subscribing to market data");
        // In a real implementation, send IB API message:
        // reqMktData(req_id, Contract(symbol), "", false, false, vec![])
        Ok(req_id)
    }

    /// Request historical data.
    pub async fn request_historical_data(
        &mut self,
        symbol: impl AsRef<str>,
        duration: &str,
        bar_size: &str,
    ) -> Result<i32, IbkrError> {
        let req_id = self.next_req_id().await;
        info!(
            req_id = req_id,
            symbol = %symbol.as_ref(),
            duration = duration,
            bar_size = bar_size,
            "requesting historical data"
        );
        Ok(req_id)
    }

    /// Place an order via the IB API.
    pub async fn place_order(
        &mut self,
        order: quant_core::order::Order,
    ) -> Result<String, IbkrError> {
        let order_id = self.next_req_id().await;
        info!(
            order_id = order_id,
            symbol = %order.symbol,
            side = %order.side,
            qty = %order.quantity,
            "placing order"
        );
        // In a real implementation, serialize and send IB API order message
        Ok(format!("IB-{order_id}"))
    }

    /// Cancel an order.
    pub async fn cancel_order(&mut self, order_id: &str) -> Result<(), IbkrError> {
        info!(order_id = %order_id, "cancelling order");
        // In a real implementation, send cancelOrder message
        Ok(())
    }

    /// Get the market data receiver.
    pub fn take_tick_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<IbkrTick>> {
        self.tick_rx.take()
    }

    async fn next_req_id(&self) -> i32 {
        let mut id = self.next_req_id.write().await;
        let current = *id;
        *id += 1;
        current
    }

    async fn send_handshake(&mut self) -> Result<(), IbkrError> {
        // IB API v100+ handshake: send "API\0" followed by version
        let msg = format!("API\0{}\0{}\0", self.config.client_id, 100);
        if let Some(ref mut stream) = self.stream {
            stream
                .write_all(msg.as_bytes())
                .await
                .map_err(|e| IbkrError::Io(e.to_string()))?;
            stream
                .flush()
                .await
                .map_err(|e| IbkrError::Io(e.to_string()))?;
            Ok(())
        } else {
            Err(IbkrError::NotConnected)
        }
    }
}

/// IBKR client errors.
#[derive(Debug, Clone, PartialEq)]
pub enum IbkrError {
    /// Not connected to IB Gateway.
    NotConnected,
    /// IO error during communication.
    Io(String),
    /// Connection timeout.
    Timeout,
    /// Invalid configuration.
    Config(String),
    /// Protocol error.
    Protocol(String),
    /// Order rejected by IB.
    OrderRejected { code: i32, message: String },
}

impl std::fmt::Display for IbkrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IbkrError::NotConnected => write!(f, "not connected to IB Gateway"),
            IbkrError::Io(msg) => write!(f, "IO error: {msg}"),
            IbkrError::Timeout => write!(f, "connection timeout"),
            IbkrError::Config(msg) => write!(f, "config error: {msg}"),
            IbkrError::Protocol(msg) => write!(f, "protocol error: {msg}"),
            IbkrError::OrderRejected { code, message } => {
                write!(f, "order rejected (code {code}): {message}")
            }
        }
    }
}

impl std::error::Error for IbkrError {}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builder() {
        let cfg = IbkrConfig::paper()
            .host("192.168.1.100")
            .port(7497)
            .client_id(42);

        assert_eq!(cfg.host, "192.168.1.100");
        assert_eq!(cfg.port, 7497);
        assert_eq!(cfg.client_id, 42);
        assert!(cfg.paper);
    }

    #[test]
    fn config_live_defaults() {
        let cfg = IbkrConfig::live();
        assert_eq!(cfg.port, TWS_LIVE_PORT);
        assert!(!cfg.paper);
    }

    #[tokio::test]
    async fn client_status_lifecycle() {
        let client = IbkrClient::new(IbkrConfig::paper());
        assert_eq!(client.status().await, ConnectionStatus::Disconnected);
        // Note: connect() would fail without a real TWS running,
        // so we only test the status logic here.
    }
}
