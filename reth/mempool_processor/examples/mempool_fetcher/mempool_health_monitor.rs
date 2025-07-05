/// Mempool Fetcher Health Monitor
/// 
/// Long-running monitor to assess the health and performance of our fetching system.
/// Tracks detection latency, mempool coverage, and timing compared to on-chain data.
///
/// Run with: cargo run --example mempool_health_monitor --release

use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;
use std::fs::File;
use std::io::Write;
use tokio::sync::Mutex;
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{H256, BlockNumber, Transaction};
use tracing::{info, warn, error};
use chrono::{Local, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionTiming {
    hash: String,
    detection_time: u64,      // Unix timestamp when we detected it
    detection_latency_us: u64, // IPC latency in microseconds
    block_number: Option<u64>,
    block_time: Option<u64>,  // Unix timestamp when mined
    time_to_block_ms: Option<i64>, // Time from our detection to block inclusion (can be negative)
    gas_price_gwei: f64,
}

#[derive(Debug, Default)]
struct HealthMetrics {
    total_detected: u64,
    total_mined: u64,
    detected_before_block: u64,
    detected_after_block: u64,
    avg_detection_latency_us: f64,
    avg_time_to_block_ms: f64,
    missed_transactions: u64, // Transactions we didn't see before they were mined
}

struct HealthMonitor {
    ipc_client: NonBlockingIpcClient,
    http_provider: Arc<Provider<Http>>,
    pending_txs: Arc<Mutex<HashMap<H256, TransactionTiming>>>,
    metrics: Arc<Mutex<HealthMetrics>>,
    log_file: Arc<Mutex<File>>,
    detailed_log: Arc<Mutex<File>>,
}

impl HealthMonitor {
    async fn new(ipc_path: &str, rpc_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create log files
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let log_dir = "/home/nima/code/crypto/logs/mempool";
        
        let log_path = format!("{}/health_monitor_{}.log", log_dir, timestamp);
        let detailed_path = format!("{}/health_monitor_detailed_{}.csv", log_dir, timestamp);
        
        let mut log_file = File::create(&log_path)?;
        let mut detailed_log = File::create(&detailed_path)?;
        
        // Write CSV header
        writeln!(detailed_log, "detection_time,hash,detection_latency_us,block_number,block_time,time_to_block_ms,gas_price_gwei,status")?;
        
        // Initialize IPC client
        let ipc_client = NonBlockingIpcClient::new(Some(ipc_path))?;
        ipc_client.start().await?;
        
        // Initialize HTTP provider
        let http_provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        
        info!("Health monitor initialized");
        info!("Main log: {}", log_path);
        info!("Detailed log: {}", detailed_path);
        
        Ok(Self {
            ipc_client,
            http_provider,
            pending_txs: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(HealthMetrics::default())),
            log_file: Arc::new(Mutex::new(log_file)),
            detailed_log: Arc::new(Mutex::new(detailed_log)),
        })
    }
    
    async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Spawn block monitor
        let block_monitor = self.spawn_block_monitor();
        
        // Spawn metrics reporter
        let metrics_reporter = self.spawn_metrics_reporter();
        
        // Main detection loop
        info!("Starting transaction detection loop...");
        
        loop {
            // Get transactions with low timeout to avoid blocking
            match tokio::time::timeout(
                Duration::from_millis(100),
                self.ipc_client.get_transactions(100)
            ).await {
                Ok(Ok(transactions)) => {
                    for tx in transactions {
                        let hash = match tx.hash.parse::<H256>() {
                            Ok(h) => h,
                            Err(e) => {
                                warn!("Invalid hash {}: {}", tx.hash, e);
                                continue;
                            }
                        };
                        
                        // Get current timestamp
                        let detection_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        
                        // Get transaction details from data
                        let gas_price_gwei = if let Some(gas_price) = tx.data.get("gasPrice") {
                            gas_price.as_str()
                                .and_then(|s| s.parse::<u128>().ok())
                                .map(|gp| gp as f64 / 1e9)
                                .unwrap_or(0.0)
                        } else {
                            0.0
                        };
                        
                        let timing = TransactionTiming {
                            hash: format!("{:?}", hash),
                            detection_time,
                            detection_latency_us: tx.detection_ns / 1000,
                            block_number: None,
                            block_time: None,
                            time_to_block_ms: None,
                            gas_price_gwei,
                        };
                        
                        // Store in pending map
                        {
                            let mut pending = self.pending_txs.lock().await;
                            pending.insert(hash, timing.clone());
                        }
                        
                        // Update metrics
                        {
                            let mut metrics = self.metrics.lock().await;
                            metrics.total_detected += 1;
                            
                            // Update average detection latency
                            let n = metrics.total_detected as f64;
                            metrics.avg_detection_latency_us = 
                                (metrics.avg_detection_latency_us * (n - 1.0) + timing.detection_latency_us as f64) / n;
                        }
                        
                        // Log detection
                        if self.metrics.lock().await.total_detected % 100 == 0 {
                            info!("Detected {} transactions, latest: {} in {}μs", 
                                  self.metrics.lock().await.total_detected, &tx.hash[..10], tx.detection_ns / 1000);
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("Error getting transactions: {}", e);
                }
                Err(_) => {
                    // Timeout, normal
                }
            }
            
            // Brief sleep to prevent CPU spinning
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    fn spawn_block_monitor(&self) -> tokio::task::JoinHandle<()> {
        let provider = self.http_provider.clone();
        let pending_txs = self.pending_txs.clone();
        let metrics = self.metrics.clone();
        let detailed_log = self.detailed_log.clone();
        
        tokio::spawn(async move {
            let mut last_block = 0u64;
            
            loop {
                // Get latest block
                match provider.get_block_number().await {
                    Ok(block_number) => {
                        let current_block = block_number.as_u64();
                        
                        // Process new blocks
                        for block_num in (last_block + 1)..=current_block {
                            if let Ok(Some(block)) = provider.get_block(block_num).await {
                                let block_time = block.timestamp.as_u64();
                                
                                // Check each transaction in the block
                                for tx_hash in &block.transactions {
                                    let mut pending = pending_txs.lock().await;
                                    
                                    if let Some(mut timing) = pending.remove(tx_hash) {
                                        // We detected this transaction!
                                        timing.block_number = Some(block_num);
                                        timing.block_time = Some(block_time);
                                        
                                        // Calculate time to block (can be negative if we detected after mining)
                                        let time_to_block_ms = (block_time as i64 - timing.detection_time as i64) * 1000;
                                        timing.time_to_block_ms = Some(time_to_block_ms);
                                        
                                        // Update metrics
                                        {
                                            let mut m = metrics.lock().await;
                                            m.total_mined += 1;
                                            
                                            if time_to_block_ms > 0 {
                                                m.detected_before_block += 1;
                                            } else {
                                                m.detected_after_block += 1;
                                                warn!("Detected tx {} AFTER it was mined! ({}ms late)", 
                                                      &timing.hash[..10], -time_to_block_ms);
                                            }
                                            
                                            // Update average time to block
                                            let n = m.total_mined as f64;
                                            m.avg_time_to_block_ms = 
                                                (m.avg_time_to_block_ms * (n - 1.0) + time_to_block_ms as f64) / n;
                                        }
                                        
                                        // Log to detailed CSV
                                        {
                                            let mut log = detailed_log.lock().await;
                                            let status = if time_to_block_ms > 0 { "before_block" } else { "after_block" };
                                            writeln!(log, "{},{},{},{},{},{},{:.3},{}",
                                                timing.detection_time,
                                                timing.hash,
                                                timing.detection_latency_us,
                                                block_num,
                                                block_time,
                                                time_to_block_ms,
                                                timing.gas_price_gwei,
                                                status
                                            ).ok();
                                        }
                                    } else {
                                        // We missed this transaction!
                                        let mut m = metrics.lock().await;
                                        m.missed_transactions += 1;
                                        
                                        if m.missed_transactions % 100 == 0 {
                                            warn!("Missed {} transactions so far", m.missed_transactions);
                                        }
                                    }
                                }
                            }
                        }
                        
                        last_block = current_block;
                    }
                    Err(e) => {
                        error!("Error getting block number: {}", e);
                    }
                }
                
                // Check for new blocks every 2 seconds
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        })
    }
    
    fn spawn_metrics_reporter(&self) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();
        let log_file = self.log_file.clone();
        let pending_txs = self.pending_txs.clone();
        
        tokio::spawn(async move {
            let mut last_report = Instant::now();
            
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                
                let elapsed = last_report.elapsed();
                last_report = Instant::now();
                
                let m = metrics.lock().await;
                let pending_count = pending_txs.lock().await.len();
                
                let report = format!(
                    "\n========== HEALTH REPORT {} ==========\n\
                    Uptime: {:.1} minutes\n\
                    Total Detected: {} transactions\n\
                    Total Mined: {} transactions\n\
                    Pending (not mined yet): {} transactions\n\
                    \n\
                    Detection Performance:\n\
                    - Average IPC latency: {:.1}μs\n\
                    - Detected before block: {} ({:.1}%)\n\
                    - Detected after block: {} ({:.1}%)\n\
                    - Missed transactions: {} ({:.1}% miss rate)\n\
                    \n\
                    Timing Analysis:\n\
                    - Average time from detection to block: {:.1}ms\n\
                    - Detection rate: {:.1} tx/sec\n\
                    =====================================\n",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    elapsed.as_secs_f64() / 60.0,
                    m.total_detected,
                    m.total_mined,
                    pending_count,
                    m.avg_detection_latency_us,
                    m.detected_before_block,
                    if m.total_mined > 0 { (m.detected_before_block as f64 / m.total_mined as f64) * 100.0 } else { 0.0 },
                    m.detected_after_block,
                    if m.total_mined > 0 { (m.detected_after_block as f64 / m.total_mined as f64) * 100.0 } else { 0.0 },
                    m.missed_transactions,
                    if m.total_mined + m.missed_transactions > 0 { 
                        (m.missed_transactions as f64 / (m.total_mined + m.missed_transactions) as f64) * 100.0 
                    } else { 0.0 },
                    m.avg_time_to_block_ms,
                    m.total_detected as f64 / elapsed.as_secs_f64()
                );
                
                info!("{}", report);
                
                // Write to log file
                if let Ok(mut file) = log_file.try_lock() {
                    let _ = file.write_all(report.as_bytes());
                    let _ = file.flush();
                }
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();
    
    info!("Starting Mempool Health Monitor");
    
    let monitor = HealthMonitor::new(
        "/tmp/reth.ipc",
        "http://localhost:8545"
    ).await?;
    
    monitor.run().await?;
    
    Ok(())
}