/// Simple Latency Test
/// 
/// Quick test to measure WebSocket detection latency
///
/// Run with: cargo run --example simple_latency_test

use mempool_processor::mempool_fetcher::WebSocketClient;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info")
        .init();

    info!("🚀 Simple WebSocket Latency Test");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    info!("Connecting to WebSocket: {}", ws_url);
    
    // Create WebSocket client
    let client = WebSocketClient::new(ws_url, http_url)?;
    
    // Start monitoring
    client.start_monitoring().await?;
    
    info!("Connected! Waiting for transactions...\n");
    
    // Monitor for 30 seconds
    let start = std::time::Instant::now();
    let mut count = 0;
    
    while start.elapsed() < Duration::from_secs(30) {
        // Get transactions
        match tokio::time::timeout(
            Duration::from_millis(100),
            client.get_transactions(10)
        ).await {
            Ok(Ok(transactions)) => {
                for tx in transactions {
                    count += 1;
                    info!("TX #{}: {} - Detection latency: {:.2}ms", 
                          count, 
                          &tx.hash[..10], 
                          tx.latency_ms);
                    
                    if count >= 10 {
                        break;
                    }
                }
                
                if count >= 10 {
                    break;
                }
            }
            _ => {} // Timeout or error, continue
        }
    }
    
    // Get final stats
    let stats = client.get_stats().await;
    info!("\n📊 RESULTS:");
    info!("  Total transactions: {}", stats.total_transactions);
    info!("  Average latency: {:.2}ms", stats.avg_latency_ms);
    info!("  Min latency: {:.2}ms", stats.min_latency_ms.unwrap_or(0.0));
    info!("  Max latency: {:.2}ms", stats.max_latency_ms.unwrap_or(0.0));
    
    Ok(())
}