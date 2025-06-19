/// Test DevP2P connection to local Reth node
/// This tests the experimental devp2p implementation

use std::time::Instant;
use tracing::{info, error, debug};
use tokio::signal;

// Import from experimental module
#[path = "../experimental/devp2p/reth_implementation.rs"]
mod reth_implementation;
use reth_implementation::{RethDevP2pClient, create_reth_devp2p_client};

// Import types
#[path = "../src/mempool_fetcher/types.rs"]
mod types;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info,mempool_processor=debug,reth=debug")
        .init();
    
    info!("🔗 Testing DevP2P connection to local Reth node...");
    
    // Create DevP2P client
    let client = match create_reth_devp2p_client().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create DevP2P client: {}", e);
            return Err(e);
        }
    };
    
    // Check initial stats
    let stats = client.get_stats();
    info!("📊 Initial stats: {:?}", stats);
    
    // Wait for connections
    info!("⏳ Waiting for peer connections...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    
    // Check stats again
    let stats = client.get_stats();
    info!("📊 Stats after 5s: {:?}", stats);
    
    if !stats.connected {
        error!("❌ Failed to connect to any peers");
        info!("Make sure:");
        info!("1. Reth node is running on localhost");
        info!("2. Port 30303 is available for Reth");
        info!("3. Port 30304 is available for our client");
        info!("4. No firewall blocking connections");
        
        // Try to ping Reth RPC to verify it's running
        match check_reth_rpc().await {
            Ok(_) => info!("✅ Reth RPC is responding at http://127.0.0.1:8545"),
            Err(e) => error!("❌ Reth RPC not responding: {}", e),
        }
        
        return Ok(());
    }
    
    info!("✅ Connected to {} peers", stats.peers);
    
    // Monitor transactions
    info!("📡 Monitoring for transactions... (Press Ctrl+C to stop)");
    
    let mut total_fetched = 0u64;
    let start = Instant::now();
    let mut last_report = Instant::now();
    
    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Fetch new transactions
                match client.fetch_new_transactions().await {
                    Ok(txs) => {
                        if !txs.is_empty() {
                            total_fetched += txs.len() as u64;
                            let fetch_time = Instant::now();
                            
                            info!("📦 Fetched {} transactions (total: {})", txs.len(), total_fetched);
                            
                            // Log first few transaction hashes
                            for (i, tx) in txs.iter().take(3).enumerate() {
                                info!("  [{}] hash: 0x{}", i, hex::encode(&tx.hash));
                            }
                            
                            // Measure latency (rough estimate)
                            debug!("Fetch completed in {:?}", fetch_time.elapsed());
                        }
                    }
                    Err(e) => {
                        error!("Error fetching transactions: {}", e);
                    }
                }
                
                // Periodic stats every 30 seconds
                if last_report.elapsed().as_secs() >= 30 {
                    let stats = client.get_stats();
                    let elapsed = start.elapsed().as_secs();
                    info!("📊 Stats: {} peers, {} txs total, {:.2} tx/s avg", 
                          stats.peers, stats.total_transactions, 
                          stats.total_transactions as f64 / elapsed as f64);
                    last_report = Instant::now();
                }
            }
        }
    }
    
    // Final stats
    let stats = client.get_stats();
    info!("📊 Final stats: {:?}", stats);
    
    // Calculate average latency
    let elapsed = start.elapsed().as_secs();
    if total_fetched > 0 && elapsed > 0 {
        info!("📈 Average rate: {:.2} tx/s", total_fetched as f64 / elapsed as f64);
    }
    
    // Shutdown
    client.shutdown().await?;
    info!("✅ Test completed");
    
    Ok(())
}

/// Check if Reth RPC is responding
async fn check_reth_rpc() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8545")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("RPC returned status: {}", response.status()).into())
    }
}