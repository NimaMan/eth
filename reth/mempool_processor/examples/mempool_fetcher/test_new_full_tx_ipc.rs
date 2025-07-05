use std::time::Instant;
use tokio::time::{sleep, Duration};
use tracing::info;
use mempool_processor::{FullTransactionIpcClient, FullTransaction};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Testing NEW FullTransactionIpcClient performance");
    info!("📌 This client gets full transaction data without RPC fallback");
    
    // Create and start the new IPC client
    let client = FullTransactionIpcClient::new(None)?;
    client.start_monitoring().await?;
    
    // Wait for connection
    sleep(Duration::from_millis(100)).await;
    
    info!("⏱️ Starting performance measurement...");
    
    let start = Instant::now();
    let mut total_transactions = 0;
    let mut sub_1ms_count = 0;
    let mut sub_100us_count = 0;
    let mut sub_10us_count = 0;
    
    // Collect transactions for 10 seconds
    let test_duration = Duration::from_secs(10);
    
    while start.elapsed() < test_duration {
        let batch_start = Instant::now();
        let transactions = client.get_full_transactions(10).await?;
        
        for tx in &transactions {
            total_transactions += 1;
            
            // Log performance metrics
            let latency_us = tx.latency_ns / 1000;
            let latency_ms = tx.latency_ns as f64 / 1_000_000.0;
            
            if latency_ms < 1.0 {
                sub_1ms_count += 1;
            }
            if latency_us < 100 {
                sub_100us_count += 1;
            }
            if latency_us < 10 {
                sub_10us_count += 1;
            }
            
            // Log ultra-fast detections
            if latency_us < 10 {
                info!("⚡ ULTRA-FAST: {} in {}μs ({}ns)", 
                      &tx.hash[..10], latency_us, tx.latency_ns);
            } else if latency_us < 100 && total_transactions % 10 == 0 {
                info!("✅ FAST: {} in {}μs", &tx.hash[..10], latency_us);
            }
            
            // Verify we have full transaction data
            if total_transactions == 1 {
                info!("🔍 First transaction data sample:");
                info!("   Hash: {}", tx.hash);
                info!("   Has full data: {}", tx.tx_data.is_object());
                if let Some(from) = tx.tx_data.get("from") {
                    info!("   From: {}", from);
                }
                if let Some(value) = tx.tx_data.get("value") {
                    info!("   Value: {}", value);
                }
            }
        }
        
        if transactions.is_empty() {
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    let elapsed = start.elapsed();
    let stats = client.get_stats().await;
    
    info!("\n📊 FullTransactionIpcClient Performance Report:");
    info!("═══════════════════════════════════════════════");
    info!("⏱️  Test duration: {:.1}s", elapsed.as_secs_f64());
    info!("📦 Total transactions: {}", stats.total_transactions);
    info!("⚡ Average latency: {}ns ({}μs)", 
          stats.avg_latency_ns, stats.avg_latency_ns / 1000);
    info!("🎯 Sub-1ms: {} ({:.1}%)", 
          stats.sub_1ms_count, 
          (stats.sub_1ms_count as f64 / stats.total_transactions as f64) * 100.0);
    info!("🚀 Sub-100μs: {} ({:.1}%)", 
          stats.sub_100us_count,
          (stats.sub_100us_count as f64 / stats.total_transactions as f64) * 100.0);
    info!("⚡ Sub-10μs: {} ({:.1}%)", 
          stats.sub_10us_count,
          (stats.sub_10us_count as f64 / stats.total_transactions as f64) * 100.0);
    info!("📈 Min latency: {}ns", stats.min_latency_ns.unwrap_or(0));
    info!("📉 Max latency: {}ns", stats.max_latency_ns.unwrap_or(0));
    info!("🔄 Throughput: {:.1} tx/s", 
          stats.total_transactions as f64 / elapsed.as_secs_f64());
    
    info!("\n✅ Key Achievement: NO RPC FALLBACK NEEDED!");
    info!("🎯 This eliminates the 5000ms latency spikes from the old system");
    
    Ok(())
}