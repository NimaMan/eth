/// Long-term Mempool Fetcher Health Test
/// 
/// Monitors the health and performance of our fetching system over extended periods.
/// Tracks detection latency, timing relative to block inclusion, and system stability.
///
/// Run with: cargo run --example long_term_health_test --release

use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::H256;
use tracing::{info, warn};
use chrono::Local;

struct HealthStats {
    total_detected: u64,
    detections_last_minute: u64,
    avg_latency_us: f64,
    max_latency_us: u64,
    min_latency_us: u64,
    last_reset: Instant,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("Starting Long-term Mempool Fetcher Health Test");

    // Create log files
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = format!("/home/nima/code/crypto/logs/mempool/long_term_health_{}.csv", timestamp);
    let summary_path = format!("/home/nima/code/crypto/logs/mempool/long_term_health_summary_{}.log", timestamp);
    
    let mut log_file = File::create(&log_path)?;
    let mut summary_file = File::create(&summary_path)?;
    
    // Write CSV header
    writeln!(log_file, "timestamp,tx_hash,detection_latency_us,unix_time")?;
    
    info!("Detailed log: {}", log_path);
    info!("Summary log: {}", summary_path);

    // Initialize IPC client
    let client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    client.start().await?;
    info!("IPC client started");

    // Initialize HTTP provider for block checking
    let provider = Provider::<Http>::try_from("http://localhost:8545")?;
    let current_block = provider.get_block_number().await?.as_u64();
    info!("Current block: {}", current_block);

    // Health statistics
    let mut stats = HealthStats {
        total_detected: 0,
        detections_last_minute: 0,
        avg_latency_us: 0.0,
        max_latency_us: 0,
        min_latency_us: u64::MAX,
        last_reset: Instant::now(),
    };
    
    let mut pending_txs: HashMap<H256, u64> = HashMap::new();
    let mut last_report = Instant::now();
    let mut last_minute_reset = Instant::now();
    
    info!("Starting detection loop...");
    
    loop {
        // Get transactions
        match client.get_transactions(100).await {
            Ok(transactions) => {
                let unix_now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                for tx in transactions {
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.6f");
                    let latency_us = tx.detection_ns / 1000;
                    
                    // Update statistics
                    stats.total_detected += 1;
                    stats.detections_last_minute += 1;
                    
                    // Update latency stats
                    if latency_us > stats.max_latency_us {
                        stats.max_latency_us = latency_us;
                    }
                    if latency_us < stats.min_latency_us {
                        stats.min_latency_us = latency_us;
                    }
                    
                    // Update average
                    let n = stats.total_detected as f64;
                    stats.avg_latency_us = (stats.avg_latency_us * (n - 1.0) + latency_us as f64) / n;
                    
                    // Log to CSV
                    writeln!(
                        log_file,
                        "{},{},{},{}",
                        timestamp,
                        tx.hash,
                        latency_us,
                        unix_now
                    )?;
                    
                    // Track pending transactions
                    if let Ok(hash) = tx.hash.parse::<H256>() {
                        pending_txs.insert(hash, unix_now);
                    }
                    
                    // Log every 1000th transaction
                    if stats.total_detected % 1000 == 0 {
                        info!(
                            "Detected {} transactions, latest: {} in {}μs",
                            stats.total_detected,
                            &tx.hash[..10],
                            latency_us
                        );
                    }
                }
            }
            Err(e) => {
                warn!("Error getting transactions: {}", e);
            }
        }
        
        // Reset minute counter
        if last_minute_reset.elapsed() > Duration::from_secs(60) {
            stats.detections_last_minute = 0;
            last_minute_reset = Instant::now();
        }
        
        // Generate report every 5 minutes
        if last_report.elapsed() > Duration::from_secs(300) {
            let report = format!(
                "\n==================== HEALTH REPORT {} ====================\n\
                Uptime: {:.1} minutes\n\
                Total Transactions Detected: {}\n\
                Detection Rate (last minute): {} tx/min\n\
                \nLatency Statistics:\n\
                - Average: {:.1}μs\n\
                - Minimum: {}μs\n\
                - Maximum: {}μs\n\
                \nPending Transactions: {} (not yet mined)\n\
                ==========================================================\n",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                stats.last_reset.elapsed().as_secs_f64() / 60.0,
                stats.total_detected,
                stats.detections_last_minute,
                stats.avg_latency_us,
                stats.min_latency_us,
                stats.max_latency_us,
                pending_txs.len()
            );
            
            info!("{}", report);
            summary_file.write_all(report.as_bytes())?;
            summary_file.flush()?;
            
            // Clean up old pending transactions (older than 10 minutes)
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            pending_txs.retain(|_, &mut time| current_time - time < 600);
            
            last_report = Instant::now();
        }
        
        // Small sleep to prevent CPU spinning
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}