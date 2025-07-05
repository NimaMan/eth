/// NonBlocking IPC Client 1K Transaction Demo
/// 
/// Demonstrates the NonBlockingIpcClient by collecting 1,000 transactions
/// and logging their timestamps to /home/nima/code/crypto/logs/mempool
///
/// Run with: cargo run --example nonblocking_ipc_1k_demo --release

use std::time::Instant;
use std::fs::File;
use std::io::Write;
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use tracing::info;
use tracing_subscriber;
use chrono::Local;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("Starting NonBlocking IPC Client 1K Demo");

    // Create log file
    let log_path = "/home/nima/code/crypto/logs/mempool/nonblocking_ipc_1k_demo.log";
    let mut log_file = File::create(log_path)?;
    writeln!(log_file, "timestamp,tx_hash,detection_ns,detection_us")?;

    // Initialize client
    let client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    client.start().await?;

    info!("Client started, collecting 1,000 transactions...");

    let start_time = Instant::now();
    let mut tx_count = 0;

    // Collect 1,000 transactions
    while tx_count < 1000 {
        // Get batch of transactions
        let transactions = client.get_transactions(100).await?;
        let is_empty = transactions.is_empty();
        
        for tx in transactions {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.6f");
            
            // Log to file
            writeln!(
                log_file,
                "{},{},{},{}",
                timestamp,
                tx.hash,
                tx.detection_ns,
                tx.detection_ns / 1000
            )?;
            
            // Log to console every 100 transactions
            if tx_count % 100 == 0 {
                info!(
                    "Progress: {}/1000 - Latest: {} in {}μs",
                    tx_count,
                    &tx.hash[..10],
                    tx.detection_ns / 1000
                );
            }
            
            tx_count += 1;
            if tx_count >= 1000 {
                break;
            }
        }
        
        // Brief pause if no transactions
        if is_empty {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    let total_time = start_time.elapsed();
    
    // Get final stats
    let stats = client.get_stats().await;
    
    // Write summary
    writeln!(log_file, "\n# Summary")?;
    writeln!(log_file, "Total transactions: {}", stats.total)?;
    writeln!(log_file, "Sub-10μs: {} ({:.1}%)", stats.sub_10us, (stats.sub_10us as f64 / stats.total as f64) * 100.0)?;
    writeln!(log_file, "Sub-100μs: {} ({:.1}%)", stats.sub_100us, (stats.sub_100us as f64 / stats.total as f64) * 100.0)?;
    writeln!(log_file, "Sub-1ms: {} ({:.1}%)", stats.sub_1ms, (stats.sub_1ms as f64 / stats.total as f64) * 100.0)?;
    writeln!(log_file, "Total time: {:.2}s", total_time.as_secs_f64())?;
    writeln!(log_file, "Throughput: {:.1} tx/s", 1000.0 / total_time.as_secs_f64())?;

    info!("Completed! Collected 1,000 transactions in {:.2}s", total_time.as_secs_f64());
    info!("Log written to: {}", log_path);
    info!("Stats: sub-10μs: {}, sub-100μs: {}, sub-1ms: {}", 
          stats.sub_10us, stats.sub_100us, stats.sub_1ms);

    Ok(())
}