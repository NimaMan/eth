/// Simple test of DevP2P connection to local Reth node
/// Run with: cargo run --example test_devp2p_simple --features reth_integration

use std::time::Instant;
use tracing::{info, error};

// Since we're in examples/, we need to include the experimental module directly
#[path = "../experimental/devp2p/reth_implementation.rs"]
mod devp2p;
use devp2p::create_reth_devp2p_client;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info")
        .init();
    
    info!("🚀 Starting DevP2P connection test...");
    
    // Create DevP2P client
    let client = match create_reth_devp2p_client().await {
        Ok(c) => {
            info!("✅ DevP2P client created successfully");
            c
        },
        Err(e) => {
            error!("❌ Failed to create DevP2P client: {}", e);
            return Err(e);
        }
    };
    
    // Wait a bit for connections
    info!("⏳ Waiting for peer connections...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    
    // Check connection status
    let stats = client.get_stats();
    info!("📊 Connection stats:");
    info!("   Connected: {}", stats.connected);
    info!("   Peers: {}", stats.peers);
    info!("   Transactions: {}", stats.total_transactions);
    
    if !stats.connected {
        error!("❌ No peers connected!");
        info!("Please ensure:");
        info!("  1. Reth is running on localhost:30303");
        info!("  2. Reth has the admin and debug APIs enabled");
        info!("  3. No firewall is blocking the connection");
        return Ok(());
    }
    
    info!("✅ Connected to {} peer(s)", stats.peers);
    
    // Monitor for transactions for 30 seconds
    info!("📡 Monitoring for transactions for 30 seconds...");
    let start = Instant::now();
    let mut tx_count = 0;
    
    while start.elapsed().as_secs() < 30 {
        match client.fetch_new_transactions().await {
            Ok(txs) => {
                if !txs.is_empty() {
                    tx_count += txs.len();
                    info!("📦 Received {} transactions (total: {})", txs.len(), tx_count);
                    
                    // Show first transaction
                    if let Some(tx) = txs.first() {
                        info!("   First tx hash: 0x{}", hex::encode(&tx.hash));
                    }
                }
            }
            Err(e) => {
                error!("Error fetching transactions: {}", e);
            }
        }
        
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    // Final stats
    let final_stats = client.get_stats();
    info!("📊 Final statistics:");
    info!("   Total transactions: {}", final_stats.total_transactions);
    info!("   Transactions per second: {:.2}", final_stats.transactions_per_second);
    info!("   Uptime: {} seconds", final_stats.uptime_seconds);
    
    // Shutdown
    client.shutdown().await?;
    info!("✅ Test completed successfully");
    
    Ok(())
}