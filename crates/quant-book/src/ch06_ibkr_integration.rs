//! Chapter 6: Connecting to Interactive Brokers
//!
//! Demonstrates the IBKR client configuration and connection lifecycle.

use anyhow::Result;
use quant_ibkr::{IbkrClient, IbkrConfig};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Chapter 6: Connecting to Interactive Brokers ===\n");

    // 1. Paper trading configuration
    println!("1. Paper Trading Configuration");
    let paper_config = IbkrConfig::paper()
        .host("127.0.0.1")
        .port(7497)
        .client_id(1);
    println!("   Host: {}", paper_config.host);
    println!("   Port: {} (paper)", paper_config.port);
    println!("   Client ID: {}\n", paper_config.client_id);

    // 2. Live trading configuration
    println!("2. Live Trading Configuration");
    let live_config = IbkrConfig::live();
    println!("   Port: {} (live)\n", live_config.port);

    // 3. Client creation (does not connect yet)
    println!("3. IBKR Client");
    let client = IbkrClient::new(paper_config);
    println!("   Status: {:?}", client.status().await);
    println!("   (Run with TWS/IB Gateway running to connect)\n");

    println!("Chapter 6 complete!");
    println!("To test connection, run:");
    println!("  podman-compose -f deploy/compose.yml up -d ib-gateway");
    println!("  cargo run --bin ch06_ibkr_integration");
    Ok(())
}
