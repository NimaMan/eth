/// Mempool Queue Monitor
/// 
/// Monitors the mempool as a queuing system, tracking:
/// - Mempool size over time
/// - Arrival rates
/// - Processing rates
/// - Queue dynamics
/// - Detection latencies
///
/// Run with: cargo run --example mempool_queue_monitor --release

use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use std::fs::File;
use std::io::Write;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use ethers::providers::{Provider, Http};
use tracing::{info, warn};
use chrono::Local;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct MempoolSnapshot {
    timestamp: u64,
    pending_count: u64,
    queued_count: u64,
    total_count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct QueueMetrics {
    timestamp: u64,
    mempool_size: u64,
    arrivals_per_sec: f64,
    detections_per_sec: f64,
    avg_latency_us: f64,
    max_latency_us: u64,
    min_latency_us: u64,
    p99_latency_us: u64,
    p95_latency_us: u64,
    p50_latency_us: u64,
}

struct QueueMonitor {
    ipc_client: NonBlockingIpcClient,
    http_provider: Arc<Provider<Http>>,
    mempool_snapshots: Arc<Mutex<VecDeque<MempoolSnapshot>>>,
    detection_latencies: Arc<Mutex<VecDeque<u64>>>,
    detection_timestamps: Arc<Mutex<VecDeque<Instant>>>,
    metrics_log: Arc<Mutex<File>>,
    detailed_log: Arc<Mutex<File>>,
}

impl QueueMonitor {
    async fn new(ipc_path: &str, rpc_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create log files
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let log_dir = "/home/nima/code/crypto/logs/mempool";
        
        let metrics_path = format!("{}/queue_metrics_{}.csv", log_dir, timestamp);
        let detailed_path = format!("{}/queue_detailed_{}.csv", log_dir, timestamp);
        
        let mut metrics_log = File::create(&metrics_path)?;
        let mut detailed_log = File::create(&detailed_path)?;
        
        // Write CSV headers
        writeln!(metrics_log, "timestamp,mempool_size,arrivals_per_sec,detections_per_sec,avg_latency_us,max_latency_us,min_latency_us,p99_latency_us,p95_latency_us,p50_latency_us")?;
        writeln!(detailed_log, "timestamp,tx_hash,latency_us,mempool_pending,mempool_queued")?;
        
        info!("Queue metrics log: {}", metrics_path);
        info!("Detailed log: {}", detailed_path);

        // Initialize IPC client
        let ipc_client = NonBlockingIpcClient::new(Some(ipc_path))?;
        ipc_client.start().await?;
        
        // Initialize HTTP provider
        let http_provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        
        Ok(Self {
            ipc_client,
            http_provider,
            mempool_snapshots: Arc::new(Mutex::new(VecDeque::with_capacity(120))), // 1 minute of data at 0.5s intervals
            detection_latencies: Arc::new(Mutex::new(VecDeque::with_capacity(10000))), // Last 10k detections
            detection_timestamps: Arc::new(Mutex::new(VecDeque::with_capacity(10000))), // Timestamps of detections
            metrics_log: Arc::new(Mutex::new(metrics_log)),
            detailed_log: Arc::new(Mutex::new(detailed_log)),
        })
    }
    
    async fn get_mempool_status(&self) -> Result<MempoolSnapshot, Box<dyn std::error::Error>> {
        // Get txpool status
        let response: serde_json::Value = self.http_provider
            .request("txpool_status", ())
            .await?;
        
        let pending = response["pending"]
            .as_str()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);
            
        let queued = response["queued"]
            .as_str()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);
        
        Ok(MempoolSnapshot {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            pending_count: pending,
            queued_count: queued,
            total_count: pending + queued,
        })
    }
    
    fn calculate_percentile(values: &[u64], percentile: f64) -> u64 {
        if values.is_empty() {
            return 0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        let index = ((percentile / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[index]
    }
    
    async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Spawn mempool monitor
        let _mempool_monitor = self.spawn_mempool_monitor();
        
        // Spawn metrics calculator
        let _metrics_calculator = self.spawn_metrics_calculator();
        
        // Main detection loop
        info!("Starting transaction detection loop...");
        
        let mut total_detected = 0u64;
        let mut last_detection_count = 0u64;
        let mut last_rate_calc = Instant::now();
        
        loop {
            // Get current mempool status for logging
            let mempool_status = self.get_mempool_status().await.unwrap_or(MempoolSnapshot {
                timestamp: 0,
                pending_count: 0,
                queued_count: 0,
                total_count: 0,
            });
            
            // Get transactions
            match self.ipc_client.get_transactions(100).await {
                Ok(transactions) => {
                    let timestamp = Local::now();
                    
                    for tx in transactions {
                        total_detected += 1;
                        let latency_us = tx.detection_ns / 1000;
                        
                        // Store latency and timestamp
                        {
                            let mut latencies = self.detection_latencies.lock().await;
                            latencies.push_back(latency_us);
                            if latencies.len() > 10000 {
                                latencies.pop_front();
                            }
                            
                            let mut timestamps = self.detection_timestamps.lock().await;
                            timestamps.push_back(Instant::now());
                            if timestamps.len() > 10000 {
                                timestamps.pop_front();
                            }
                        }
                        
                        // Log to detailed CSV
                        {
                            let mut log = self.detailed_log.lock().await;
                            writeln!(
                                log,
                                "{},{},{},{},{}",
                                timestamp.format("%Y-%m-%d %H:%M:%S%.6f"),
                                tx.hash,
                                latency_us,
                                mempool_status.pending_count,
                                mempool_status.queued_count
                            )?;
                        }
                        
                        // Log every 1000th transaction
                        if total_detected % 1000 == 0 {
                            let elapsed = last_rate_calc.elapsed();
                            let rate = (total_detected - last_detection_count) as f64 / elapsed.as_secs_f64();
                            
                            info!(
                                "Detected {} txs | Rate: {:.1} tx/s | Mempool: {} pending, {} queued | Latest: {}μs",
                                total_detected,
                                rate,
                                mempool_status.pending_count,
                                mempool_status.queued_count,
                                latency_us
                            );
                            
                            last_detection_count = total_detected;
                            last_rate_calc = Instant::now();
                        }
                    }
                }
                Err(e) => {
                    warn!("Error getting transactions: {}", e);
                }
            }
            
            // Small sleep
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    fn spawn_mempool_monitor(&self) -> tokio::task::JoinHandle<()> {
        let provider = self.http_provider.clone();
        let snapshots = self.mempool_snapshots.clone();
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(500));
            
            loop {
                ticker.tick().await;
                
                // Get mempool status
                let response: Result<serde_json::Value, _> = provider
                    .request("txpool_status", ())
                    .await;
                    
                if let Ok(status) = response {
                    let pending = status["pending"]
                        .as_str()
                        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                        .unwrap_or(0);
                        
                    let queued = status["queued"]
                        .as_str()
                        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                        .unwrap_or(0);
                    
                    let snapshot = MempoolSnapshot {
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        pending_count: pending,
                        queued_count: queued,
                        total_count: pending + queued,
                    };
                    
                    let mut snapshots_guard = snapshots.lock().await;
                    snapshots_guard.push_back(snapshot);
                    if snapshots_guard.len() > 120 {
                        snapshots_guard.pop_front();
                    }
                }
            }
        })
    }
    
    fn spawn_metrics_calculator(&self) -> tokio::task::JoinHandle<()> {
        let snapshots = self.mempool_snapshots.clone();
        let latencies = self.detection_latencies.clone();
        let timestamps = self.detection_timestamps.clone();
        let metrics_log = self.metrics_log.clone();
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                ticker.tick().await;
                
                // Calculate metrics
                let snapshots_guard = snapshots.lock().await;
                let latencies_guard = latencies.lock().await;
                let timestamps_guard = timestamps.lock().await;
                
                if !snapshots_guard.is_empty() && !latencies_guard.is_empty() {
                    // Get current mempool size
                    let current_mempool = snapshots_guard.back()
                        .map(|s| s.total_count)
                        .unwrap_or(0);
                    
                    // Calculate arrival rate (mempool growth)
                    let arrival_rate = if snapshots_guard.len() >= 10 {
                        let old = &snapshots_guard[snapshots_guard.len() - 10];
                        let new = snapshots_guard.back().unwrap();
                        let time_diff = (new.timestamp - old.timestamp) as f64;
                        if time_diff > 0.0 {
                            ((new.total_count as i64 - old.total_count as i64).max(0) as f64) / time_diff
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    
                    // Calculate detection rate (actual transactions in last 5 seconds)
                    let now = Instant::now();
                    let five_secs_ago = now - Duration::from_secs(5);
                    let recent_detections = timestamps_guard.iter()
                        .filter(|&&t| t > five_secs_ago)
                        .count();
                    let detection_rate = recent_detections as f64 / 5.0;
                    
                    // Calculate latency statistics
                    let latencies_vec: Vec<u64> = latencies_guard.iter().cloned().collect();
                    let avg_latency = latencies_vec.iter().sum::<u64>() as f64 / latencies_vec.len() as f64;
                    let max_latency = *latencies_vec.iter().max().unwrap_or(&0);
                    let min_latency = *latencies_vec.iter().min().unwrap_or(&0);
                    
                    let p99 = Self::calculate_percentile(&latencies_vec, 99.0);
                    let p95 = Self::calculate_percentile(&latencies_vec, 95.0);
                    let p50 = Self::calculate_percentile(&latencies_vec, 50.0);
                    
                    let metrics = QueueMetrics {
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        mempool_size: current_mempool,
                        arrivals_per_sec: arrival_rate,
                        detections_per_sec: detection_rate,
                        avg_latency_us: avg_latency,
                        max_latency_us: max_latency,
                        min_latency_us: min_latency,
                        p99_latency_us: p99,
                        p95_latency_us: p95,
                        p50_latency_us: p50,
                    };
                    
                    // Log metrics
                    info!(
                        "Queue Metrics | Mempool: {} | Arrivals: {:.1}/s | Detections: {:.1}/s | Latency: avg={:.1}μs p99={}μs",
                        metrics.mempool_size,
                        metrics.arrivals_per_sec,
                        metrics.detections_per_sec,
                        metrics.avg_latency_us,
                        metrics.p99_latency_us
                    );
                    
                    // Write to CSV
                    if let Ok(mut log) = metrics_log.try_lock() {
                        let _ = writeln!(
                            log,
                            "{},{},{:.2},{:.2},{:.2},{},{},{},{},{}",
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            metrics.mempool_size,
                            metrics.arrivals_per_sec,
                            metrics.detections_per_sec,
                            metrics.avg_latency_us,
                            metrics.max_latency_us,
                            metrics.min_latency_us,
                            metrics.p99_latency_us,
                            metrics.p95_latency_us,
                            metrics.p50_latency_us
                        );
                        let _ = log.flush();
                    }
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
    
    info!("Starting Mempool Queue Monitor");
    info!("This treats the mempool as a queuing system and monitors:");
    info!("- Mempool size every 0.5 seconds");
    info!("- Transaction arrival rates");
    info!("- Our detection rates and latencies");
    info!("- Queue dynamics over time");
    
    let monitor = QueueMonitor::new(
        "/tmp/reth.ipc",
        "http://localhost:8545"
    ).await?;
    
    monitor.run().await?;
    
    Ok(())
}