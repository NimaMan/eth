use std::time::{Duration, Instant};
use clap::Parser;
use eyre::Result;
use tracing::info;
use tokio::time;

// Mempool processor imports
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use mempool_processor::signal_engine::FunctionDetector;

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Starting Mempool Signal Detection Service");
    info!("   ⚡ Using non-blocking IPC for sub-millisecond latency");
    
    // Initialize non-blocking IPC client
    info!("🚀 Initializing IPC client...");
    info!("   Socket path: {}", args.ipc_path);
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("⚡ IPC subscription active!");
    
    // Initialize function detector
    info!("🔍 Initializing function detector...");
    let function_detector = FunctionDetector::new();
    
    // Performance metrics
    info!("🎯 Starting main processing loop...");
    let mut total_processed = 0u64;
    let mut last_report = Instant::now();
    
    // Timing vectors - only keep last 1000 transactions for reporting
    let mut detection_latencies_ms: Vec<f64> = Vec::with_capacity(1000);
    let mut function_detection_times_ms: Vec<f64> = Vec::with_capacity(1000);
    
    let mut consecutive_empty = 0u64;
    
    loop {
        // Get new transactions from IPC
        let new_txs = ipc_client.get_transactions_instant(100).await;
        
        if new_txs.is_empty() {
            consecutive_empty += 1;
            
            // Adaptive backoff to reduce CPU usage during empty periods
            let sleep_time = match consecutive_empty {
                1..=10 => Duration::from_micros(100),     // First 1ms: check every 100μs
                11..=100 => Duration::from_millis(1),     // Next 90ms: check every 1ms
                _ => Duration::from_millis(10),           // After 100ms: check every 10ms
            };
            time::sleep(sleep_time).await;
            continue;
        }
        
        // Got transactions! Reset counter
        consecutive_empty = 0;
        
        // Process each transaction for function detection
        for ipc_tx in new_txs {
            let function_detection_start = Instant::now();
            
            // Pass transaction directly to function detector
            function_detector.detect_from_ipc(&ipc_tx);
            
            // Track function detection time
            let function_detection_elapsed = function_detection_start.elapsed().as_secs_f64() * 1000.0;
            function_detection_times_ms.push(function_detection_elapsed);
            
            // Track IPC detection latency
            let detection_latency_ms = ipc_tx.detection_ns as f64 / 1_000_000.0;
            detection_latencies_ms.push(detection_latency_ms);
            
            total_processed += 1;
        }
        
        // Periodic reporting every 60 seconds
        if last_report.elapsed() > Duration::from_secs(60) {
            let avg_detection = if !detection_latencies_ms.is_empty() {
                detection_latencies_ms.iter().sum::<f64>() / detection_latencies_ms.len() as f64
            } else { 0.0 };
            
            let max_detection = detection_latencies_ms.iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .copied()
                .unwrap_or(0.0);
            
            let avg_function_detection = if !function_detection_times_ms.is_empty() {
                function_detection_times_ms.iter().sum::<f64>() / function_detection_times_ms.len() as f64
            } else { 0.0 };
            
            let max_function_detection = function_detection_times_ms.iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .copied()
                .unwrap_or(0.0);
            
            info!("📊 PERFORMANCE REPORT:");
            info!("   Total transactions processed: {}", total_processed);
            info!("   IPC detection latency - Avg: {:.3}ms, Max: {:.3}ms", avg_detection, max_detection);
            info!("   Function detection time - Avg: {:.3}ms, Max: {:.3}ms", avg_function_detection, max_function_detection);
            
            // Log performance metrics to file
            function_detector.log_performance_metrics(total_processed, avg_detection, max_detection,
                                                    avg_function_detection, max_function_detection);
            
            // Log function detection statistics
            function_detector.log_stats_summary();
            
            // Clear timing vectors
            detection_latencies_ms.clear();
            function_detection_times_ms.clear();
            last_report = Instant::now();
        }
    }
}