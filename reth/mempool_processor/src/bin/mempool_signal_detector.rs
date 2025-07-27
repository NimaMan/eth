use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use clap::Parser;
use eyre::Result;
use tracing::{info, warn};
use tokio::time;
use tokio::signal;

// Mempool processor imports
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use mempool_processor::signal_engine::FunctionDetector;
use mempool_processor::token_tracking::TokenTrackingSubscriber;

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
    
    // Set up shutdown signal handler
    let shutdown = setup_shutdown_handler();
    
    // Initialize logging with daily rotation
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    let log_dir = std::path::Path::new("logs/mempool_signal_detector");
    std::fs::create_dir_all(log_dir)?;
    
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("mempool_signal_detector")
        .filename_suffix("log")
        .build(log_dir)?;
    
    tracing_subscriber::fmt()
        .with_target(false)
        .with_writer(file_appender)
        .init();
    
    info!("🚀 Starting Mempool Signal Detection Service");
    info!("   ⚡ Using non-blocking IPC for sub-millisecond latency");
    
    // Initialize non-blocking IPC client
    info!("🚀 Initializing IPC client...");
    info!("   Socket path: {}", args.ipc_path);
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("⚡ IPC subscription active!");
    
    // Initialize token tracker and cache first
    info!("📊 Initializing token tracker...");
    let mut token_tracker = TokenTrackingSubscriber::new(0.1); // 0.1 ETH threshold
    let token_cache = token_tracker.get_cache();
    
    // Initialize function detector with token cache
    info!("🔍 Initializing function detector...");
    let function_detector = FunctionDetector::new_with_cache(Some(token_cache.clone()));
    
    // Get the log directory from function detector to share with other components
    let log_dir = function_detector.get_log_dir().to_path_buf();
    let base_log_dir = log_dir.parent().unwrap().to_str().unwrap();
    
    // Initialize signal processor (disabled for now due to compilation issues)
    // info!("🧠 Initializing signal processor...");
    // let signal_processor = SignalProcessor::new(token_cache.clone())?;
    
    // Signal publisher disabled for now due to compilation issues
    // info!("📡 Initializing signal publisher...");
    // let publisher_config = SignalPublisherConfig::with_timestamped_logs(base_log_dir);
    // let mut _signal_publisher = SignalPublisher::new(publisher_config).await?;
    
    
    // Start listening for token updates in background (includes initial data request)
    tokio::spawn(async move {
        if let Err(e) = token_tracker.start_listening().await {
            warn!("Token tracker error: {}", e);
        }
    });
    
    // Give it a moment to load initial data
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("✅ Token cache initialized with {} pools, {} creators", 
          token_cache.pools.get_pool_count().await, 
          token_cache.creators.get_creator_count().await);
    
    // Performance metrics
    info!("🎯 Starting main processing loop...");
    let mut total_processed = 0u64;
    let mut last_report = Instant::now();
    
    // Timing vectors - only keep last 1000 transactions for reporting
    let mut detection_latencies_ms: Vec<f64> = Vec::with_capacity(1000);
    let mut function_detection_times_ms: Vec<f64> = Vec::with_capacity(1000);
    
    let mut consecutive_empty = 0u64;
    
    // Main processing loop with shutdown handling
    loop {
        // Check for shutdown signal
        if shutdown.load(Ordering::Relaxed) {
            info!("🛑 Shutdown signal received, stopping gracefully...");
            break;
        }
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
            let detection_latency_ms = tx_with_func.detection_ns as f64 / 1_000_000.0;
            detection_latencies_ms.push(detection_latency_ms);
        }
        
        // Signal processing disabled for now due to compilation issues
        // TODO: Re-enable once signal_processor compilation issues are fixed
        /*
        match signal_processor.process_batch(transactions_with_functions.clone()).await {
            Ok(signals) => {
                // Publish each detected signal
                for signal in signals {
                    if let Err(e) = signal_publisher.publish(signal).await {
                        warn!("Failed to publish signal: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Signal processor error: {}", e);
            }
        }
        */
        
        
        total_processed += transactions_with_functions.len() as u64;
        
        // Periodic reporting every 60 seconds
        if last_report.elapsed() > Duration::from_secs(60) {
            let avg_detection = if !detection_latencies_ms.is_empty() {
                detection_latencies_ms.iter().sum::<f64>() / detection_latencies_ms.len() as f64
            } else { 0.0 };
            
            let max_detection = detection_latencies_ms.iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .copied()
                .unwrap_or(0.0);
            
            let avg_function_detection = if !function_detection_times_ms.is_empty() {
                function_detection_times_ms.iter().sum::<f64>() / function_detection_times_ms.len() as f64
            } else { 0.0 };
            
            let max_function_detection = function_detection_times_ms.iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .copied()
                .unwrap_or(0.0);
            
            info!("📊 PERFORMANCE REPORT:");
            info!("   Total transactions processed: {}", total_processed);
            info!("   IPC detection latency - Avg: {:.3}ms, Max: {:.3}ms", avg_detection, max_detection);
            info!("   Function detection time - Avg: {:.3}ms, Max: {:.3}ms", avg_function_detection, max_function_detection);
            
            // Signal publisher statistics disabled for now
            // let publisher_stats = signal_publisher.get_stats();
            // info!("   Signal publisher - Published: {}, ZMQ: {}, Logs: {}, DB: {}, Errors: {}", 
            //       publisher_stats.total_published, publisher_stats.zmq_published, 
            //       publisher_stats.logs_written, publisher_stats.db_written, publisher_stats.errors);
            
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
    
    // Graceful shutdown
    info!("📊 Final statistics before shutdown:");
    info!("   Total transactions processed: {}", total_processed);
    
    // Signal publisher statistics disabled for now
    // let final_publisher_stats = signal_publisher.get_stats();
    // info!("   Final signal publisher stats - Published: {}, ZMQ: {}, Logs: {}, DB: {}, Errors: {}", 
    //       final_publisher_stats.total_published, final_publisher_stats.zmq_published, 
    //       final_publisher_stats.logs_written, final_publisher_stats.db_written, final_publisher_stats.errors);
    
    
    info!("✅ Mempool signal detector shutdown complete");
    Ok(())
}

/// Sets up signal handlers for graceful shutdown
fn setup_shutdown_handler() -> Arc<AtomicBool> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    
    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .unwrap_or_else(|e| {
                    eprintln!("Failed to install Ctrl+C handler: {}", e);
                    std::process::exit(1);
                });
        };
        
        #[cfg(unix)]
        let terminate = async {
            match signal::unix::signal(signal::unix::SignalKind::terminate()) {
                Ok(mut stream) => stream.recv().await,
                Err(e) => {
                    eprintln!("Failed to install SIGTERM handler: {}", e);
                    std::future::pending().await
                }
            };
        };
        
        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();
        
        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C signal");
            }
            _ = terminate => {
                info!("Received SIGTERM signal");
            }
        }
        
        shutdown_clone.store(true, Ordering::Relaxed);
    });
    
    shutdown
}