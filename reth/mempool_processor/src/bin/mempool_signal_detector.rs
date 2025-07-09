use std::time::{Duration, Instant};
use clap::Parser;
use eyre::Result;
use tracing::{info, warn};
use tokio::time;

// Mempool processor imports
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use mempool_processor::signal_engine::FunctionDetector;
use mempool_processor::tx_simulator::SimulatorProcessor;
use mempool_processor::pool_subscriber::PoolSubscriber;

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Reth data directory
    #[arg(long, env = "RETH_DATADIR", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_datadir: String,
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
    
    // Get the log directory from function detector to share with simulator
    let log_dir = function_detector.get_log_dir().to_path_buf();
    
    // Initialize simulator processor with same log directory
    info!("🔄 Initializing simulator processor...");
    let mut simulator_processor = SimulatorProcessor::new(&args.reth_datadir, log_dir)?;
    
    // Initialize pool subscriber and cache
    info!("📊 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::new(0.1); // 0.1 ETH threshold
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Clone pool cache for simulator processor
    let pool_cache_for_simulator = (*pool_cache).clone();
    simulator_processor.set_pool_cache(pool_cache_for_simulator);
    
    // Start listening for pool updates in background (includes initial pool request)
    tokio::spawn(async move {
        if let Err(e) = pool_subscriber.start_listening().await {
            warn!("Pool subscriber error: {}", e);
        }
    });
    
    // Give it a moment to load initial pools
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("✅ Pool cache initialized with {} pools", pool_cache.get_pool_count());
    
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
        
        // Process batch with new pipeline
        let batch_start = Instant::now();
        
        // Function detection on entire batch
        let transactions_with_functions = function_detector.detect_batch(new_txs);
        
        // Track function detection time for the batch
        let function_detection_elapsed = batch_start.elapsed().as_secs_f64() * 1000.0;
        function_detection_times_ms.push(function_detection_elapsed);
        
        // Track IPC detection latency for each transaction
        for tx_with_func in &transactions_with_functions {
            let detection_latency_ms = tx_with_func.tx.detection_ns as f64 / 1_000_000.0;
            detection_latencies_ms.push(detection_latency_ms);
        }
        
        // Send batch to simulator processor (non-blocking)
        if let Err(e) = simulator_processor.process_batch(transactions_with_functions.clone()).await {
            warn!("Simulator processor error: {}", e);
        }
        
        total_processed += transactions_with_functions.len() as u64;
        
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
            
            // Log simulator processor statistics
            simulator_processor.log_performance_summary();
            
            // Clear timing vectors
            detection_latencies_ms.clear();
            function_detection_times_ms.clear();
            last_report = Instant::now();
        }
    }
}