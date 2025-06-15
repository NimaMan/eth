/// Quick Simulation Test
/// 
/// Minimal test to simulate transactions and measure performance
///
/// Run with: cargo run --example quick_simulation_test

use mempool_processor::mempool_fetcher::WebSocketClient;
use ethers::prelude::*;
use std::time::{Duration, Instant};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info")
        .init();

    info!("🚀 Quick Transaction Simulation Test");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    // Connect to mempool
    info!("Connecting to mempool...");
    let ws_client = WebSocketClient::new(ws_url, http_url)?;
    ws_client.start_monitoring().await?;
    
    info!("Connected! Detecting transactions...\n");
    
    let start_time = Instant::now();
    let mut count = 0;
    let mut total_detection_time = 0.0;
    
    // Detect 10 transactions
    while count < 10 && start_time.elapsed() < Duration::from_secs(60) {
        match tokio::time::timeout(
            Duration::from_millis(500),
            ws_client.get_transactions(5)
        ).await {
            Ok(Ok(transactions)) => {
                for tx in transactions {
                    count += 1;
                    total_detection_time += tx.latency_ms;
                    
                    info!("TX #{}: {}", count, &tx.hash[..10]);
                    info!("  Detection latency: {:.2}ms", tx.latency_ms);
                    info!("  (Simulation would happen here)\n");
                    
                    if count >= 10 {
                        break;
                    }
                }
            }
            _ => {} // Timeout, continue
        }
    }
    
    info!("\n📊 RESULTS:");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Transactions detected: {}", count);
    if count > 0 {
        info!("Average detection latency: {:.2}ms", total_detection_time / count as f64);
    }
    info!("\nNOTE: This only measured detection latency.");
    info!("Full simulation with state changes would add ~50-200ms per transaction.");
    
    Ok(())
}