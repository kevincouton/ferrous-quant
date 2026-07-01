//! Order execution and paper trading engine with venue abstractions and fill simulation.

#![warn(missing_docs)]

use quant_core::order::{Order, OrderStatus};
use quant_core::price::{Price, Quantity};
use quant_core::symbol::Symbol;
use std::collections::HashMap;

/// A fill event from an execution venue.
#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    /// Order ID.
    pub order_id: String,
    /// Filled quantity.
    pub quantity: Quantity,
    /// Fill price.
    pub price: Price,
    /// Timestamp.
    pub timestamp: i64,
    /// Exchange or venue.
    pub venue: String,
}

/// An execution venue (broker, exchange, or simulator).
#[async_trait::async_trait]
pub trait ExecutionVenue: Send + Sync {
    /// Submit an order to the venue.
    async fn submit(&self, order: &Order) -> Result<String, ExecutionError>;

    /// Cancel an order.
    async fn cancel(&self, order_id: &str) -> Result<(), ExecutionError>;

    /// Query order status.
    async fn status(&self, order_id: &str) -> Result<OrderStatus, ExecutionError>;
}

/// Execution errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionError {
    /// Connection lost.
    NotConnected,
    /// Order rejected.
    Rejected { reason: String },
    /// Order not found.
    NotFound,
    /// Internal error.
    Internal(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::NotConnected => write!(f, "not connected"),
            ExecutionError::Rejected { reason } => write!(f, "rejected: {reason}"),
            ExecutionError::NotFound => write!(f, "order not found"),
            ExecutionError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// Paper trading venue that simulates fills at current market price.
#[derive(Debug, Default)]
pub struct PaperVenue {
    orders: std::sync::Mutex<HashMap<String, Order>>,
    fills: std::sync::Mutex<Vec<Fill>>,
}

impl PaperVenue {
    /// Create a new paper venue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Simulate a market fill for an order at a given price.
    pub fn simulate_fill(&self, order_id: &str, price: Price) -> Option<Fill> {
        let mut orders = self.orders.lock().unwrap();
        let order = orders.get_mut(order_id)?;
        if order.is_filled() || order.is_terminal() {
            return None;
        }

        let fill = Fill {
            order_id: order_id.to_string(),
            quantity: order.remaining_quantity(),
            price,
            timestamp: chrono::Utc::now().timestamp_millis(),
            venue: "PAPER".into(),
        };

        order.filled_quantity = order.quantity;
        order.status = OrderStatus::Filled;
        order.avg_fill_price = Some(price);

        drop(orders);
        self.fills.lock().unwrap().push(fill.clone());
        Some(fill)
    }
}

#[async_trait::async_trait]
impl ExecutionVenue for PaperVenue {
    async fn submit(&self, order: &Order) -> Result<String, ExecutionError> {
        let mut orders = self.orders.lock().unwrap();
        let id = order.client_order_id.clone();
        orders.insert(id.clone(), order.clone());
        Ok(id)
    }

    async fn cancel(&self, order_id: &str) -> Result<(), ExecutionError> {
        let mut orders = self.orders.lock().unwrap();
        let order = orders.get_mut(order_id).ok_or(ExecutionError::NotFound)?;
        if order.is_terminal() {
            return Err(ExecutionError::Rejected {
                reason: "order already in terminal state".into(),
            });
        }
        order.status = OrderStatus::Cancelled;
        Ok(())
    }

    async fn status(&self, order_id: &str) -> Result<OrderStatus, ExecutionError> {
        let orders = self.orders.lock().unwrap();
        let order = orders.get(order_id).ok_or(ExecutionError::NotFound)?;
        Ok(order.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn paper_venue_submits_and_fills() {
        let venue = PaperVenue::new();
        let order = Order::market(
            "ORD-1",
            Symbol::new("AAPL", quant_core::symbol::InstrumentType::Stock),
            quant_core::order::OrderSide::Buy,
            Quantity::from_i64(100),
        );

        let id = venue.submit(&order).await.unwrap();
        assert_eq!(id, "ORD-1");

        let fill = venue.simulate_fill(&id, Price::new(150.0)).unwrap();
        assert_eq!(fill.quantity.to_i64(), 100);
        assert_eq!(fill.price, Price::new(150.0));

        let status = venue.status(&id).await.unwrap();
        assert_eq!(status, OrderStatus::Filled);
    }
}
