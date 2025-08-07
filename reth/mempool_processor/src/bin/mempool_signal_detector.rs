/// Mempool Signal Detector Service
/// 
/// Production service that implements the complete signal detection pipeline:
/// 1. Receives transactions from Reth IPC
/// 2. Detects function signatures
/// 3. Routes transactions by category (contract creation, creator actions)
/// 4. Simulates relevant transactions
/// 5. Detects signals (trading enabled, liquidity removal, honeypots, etc.)
/// 6. Publishes signals via ZMQ and logs
///
/// Performance targets:
/// - Function detection: <10μs per transaction
/// - TX routing: <5μs per transaction  
/// - Simulation: <50ms per transaction (Reth bottleneck)
/// - Signal detection: <1ms per result
/// - End-to-end: <100ms for critical signals

use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::path::PathBuf;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error};
use tokio::time;
use tokio::signal;
use tokio::sync::Mutex;
use tracing_subscriber::Layer;

// Mempool processor imports
use mempool_processor::{
    mempool_fetcher::NonBlockingIpcClient,
    function_detector::FunctionDetector,
    tx_router::{TransactionRouter, TransactionCategory, CreatorFunctionType},
    simulator::{SimulationManager, SimulationRequest, SimulationType, UnifiedSimulator, BuySellSimulatorConfig},
    signal_detector::SignalManagerConfig,
    token_tracking::TokenTrackingSubscriber,
    signal_publisher::{SignalPublisher, SignalPublisherConfig},
    database::{MempoolTimestampTracker, TrackerConfig},
};
use ethers::types::H256;
use hex;
use sqlx::postgres::PgPoolOptions;

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// Reth database path for simulations
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_db_path: String,
    
    /// Log directory base path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool")]
    log_dir: String,
    
    /// Batch size for transaction processing
    #[arg(long, default_value = "100")]
    batch_size: usize,
    
    /// Simulation worker threads
    #[arg(long, default_value = "10")]
    sim_workers: usize,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Performance report interval in seconds
    #[arg(long, default_value = "60")]
    report_interval: u64,
    
    /// Database URL for mempool timestamp tracking (optional)
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
}

/// Performance metrics tracker
struct ServiceMetrics {
    // Transaction counters
    total_processed: AtomicU64,
    contract_creations: AtomicU64,
    creator_actions: AtomicU64,
    dex_interactions: AtomicU64,
    regular_txs: AtomicU64,
    
    // Simulation metrics
    simulations_submitted: AtomicU64,
    simulations_completed: AtomicU64,
    simulation_errors: AtomicU64,
    
    // Signal counts
    trading_enabled_signals: Arc<AtomicU64>,
    liquidity_removal_signals: Arc<AtomicU64>,
    honeypot_signals: Arc<AtomicU64>,
    tax_change_signals: Arc<AtomicU64>,
    
    // Timing metrics (using Mutex for simplicity with vectors)
    detection_latencies: Arc<Mutex<Vec<Duration>>>,
    routing_latencies: Arc<Mutex<Vec<Duration>>>,
    simulation_times: Arc<Mutex<Vec<Duration>>>,
}

impl ServiceMetrics {
    fn new() -> Self {
        Self {
            total_processed: AtomicU64::new(0),
            contract_creations: AtomicU64::new(0),
            creator_actions: AtomicU64::new(0),
            dex_interactions: AtomicU64::new(0),
            regular_txs: AtomicU64::new(0),
            simulations_submitted: AtomicU64::new(0),
            simulations_completed: AtomicU64::new(0),
            simulation_errors: AtomicU64::new(0),
            trading_enabled_signals: Arc::new(AtomicU64::new(0)),
            liquidity_removal_signals: Arc::new(AtomicU64::new(0)),
            honeypot_signals: Arc::new(AtomicU64::new(0)),
            tax_change_signals: Arc::new(AtomicU64::new(0)),
            detection_latencies: Arc::new(Mutex::new(Vec::with_capacity(10000))),
            routing_latencies: Arc::new(Mutex::new(Vec::with_capacity(10000))),
            simulation_times: Arc::new(Mutex::new(Vec::with_capacity(1000))),
        }
    }
    
    async fn add_detection_latency(&self, latency: Duration) {
        let mut latencies = self.detection_latencies.lock().await;
        if latencies.len() >= 10000 {
            latencies.drain(0..5000); // Keep last 5000
        }
        latencies.push(latency);
    }
    
    async fn add_routing_latency(&self, latency: Duration) {
        let mut latencies = self.routing_latencies.lock().await;
        if latencies.len() >= 10000 {
            latencies.drain(0..5000);
        }
        latencies.push(latency);
    }
    
    async fn add_simulation_time(&self, time: Duration) {
        let mut times = self.simulation_times.lock().await;
        if times.len() >= 1000 {
            times.drain(0..500);
        }
        times.push(time);
    }
    
    async fn calculate_latency_stats(&self, latencies: &[Duration]) -> (Duration, Duration, Duration) {
        if latencies.is_empty() {
            return (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        }
        
        let mut sorted = latencies.to_vec();
        sorted.sort();
        
        let sum: Duration = sorted.iter().sum();
        let avg = sum / sorted.len() as u32;
        let max = sorted.last().cloned().unwrap_or(Duration::ZERO);
        let p99 = sorted.get(sorted.len() * 99 / 100).cloned().unwrap_or(max);
        
        (avg, max, p99)
    }
    
    async fn report(&self, elapsed: Duration) -> String {
        let detection_latencies = self.detection_latencies.lock().await;
        let (avg_detect, max_detect, _p99_detect) = self.calculate_latency_stats(&detection_latencies).await;
        drop(detection_latencies);
        
        let routing_latencies = self.routing_latencies.lock().await;
        let (avg_route, max_route, _p99_route) = self.calculate_latency_stats(&routing_latencies).await;
        drop(routing_latencies);
        
        let simulation_times = self.simulation_times.lock().await;
        let (avg_sim, max_sim, _p99_sim) = self.calculate_latency_stats(&simulation_times).await;
        drop(simulation_times);
        
        let total = self.total_processed.load(Ordering::Relaxed);
        let creations = self.contract_creations.load(Ordering::Relaxed);
        let creator_actions = self.creator_actions.load(Ordering::Relaxed);
        let _dex = self.dex_interactions.load(Ordering::Relaxed);
        let _regular = self.regular_txs.load(Ordering::Relaxed);
        
        let _sims_submitted = self.simulations_submitted.load(Ordering::Relaxed);
        let sims_completed = self.simulations_completed.load(Ordering::Relaxed);
        let sim_errors = self.simulation_errors.load(Ordering::Relaxed);
        
        let rate = total as f64 / elapsed.as_secs_f64();
        
        format!(
            "TX: {} ({:.1}/s) | Detect: {}μs/{}μs | Route: {}μs/{}μs | Sim: {:.1}ms/{:.1}ms | CC:{} CA:{} | Sims:{}/{} | Signals: TE:{} LR:{} HP:{} TC:{}",
            total, rate,
            avg_detect.as_micros(), max_detect.as_micros(),
            avg_route.as_micros(), max_route.as_micros(),
            avg_sim.as_secs_f64() * 1000.0, max_sim.as_secs_f64() * 1000.0,
            creations, creator_actions,
            sims_completed, sim_errors,
            self.trading_enabled_signals.load(Ordering::Relaxed),
            self.liquidity_removal_signals.load(Ordering::Relaxed),
            self.honeypot_signals.load(Ordering::Relaxed),
            self.tax_change_signals.load(Ordering::Relaxed),
        )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Set up shutdown signal handler
    let shutdown = setup_shutdown_handler();
    
    // Create timestamped run directory
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let run_dir = PathBuf::from(&args.log_dir).join(format!("signal_detector_{}", timestamp));
    std::fs::create_dir_all(&run_dir)?;
    
    // Initialize logging to run directory
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    
    // Main log file for general logs
    let main_file_appender = RollingFileAppender::builder()
        .rotation(Rotation::NEVER)  // Single file per run
        .filename_prefix("signal_detector")
        .filename_suffix("log")
        .build(&run_dir)?;
    
    // Simulation log file for simulation-specific logs
    let sim_file_appender = RollingFileAppender::builder()
        .rotation(Rotation::NEVER)
        .filename_prefix("simulation")
        .filename_suffix("log")
        .build(&run_dir)?;
    
    // Create the main layer with filtering to exclude simulation logs
    let main_filter = if args.verbose {
        "debug,mempool_processor::simulator=warn,reth=warn,reth_provider=warn,reth_db=warn"
    } else {
        "info,mempool_processor::simulator=warn,reth=warn,reth_provider=warn,reth_db=warn"
    };
    
    let main_layer = fmt::layer()
        .with_target(false)
        .with_writer(main_file_appender)
        .with_filter(EnvFilter::new(main_filter));
    
    // Create the simulation layer that only captures simulation logs
    let sim_layer = fmt::layer()
        .with_target(false)
        .with_writer(sim_file_appender)
        .with_filter(EnvFilter::new("mempool_processor::simulator=info"));
    
    // Combine layers
    tracing_subscriber::registry()
        .with(main_layer)
        .with(sim_layer)
        .init();
    
    info!("🚀 Starting Mempool Signal Detection Service");
    info!("================================");
    info!("Configuration:");
    info!("  IPC Path: {}", args.ipc_path);
    info!("  Reth DB: {}", args.reth_db_path);
    info!("  Log Directory: {}", args.log_dir);
    info!("  Batch Size: {}", args.batch_size);
    info!("  Simulation Workers: {}", args.sim_workers);
    info!("  Report Interval: {}s", args.report_interval);
    info!("================================");
    
    // Initialize metrics
    let metrics = Arc::new(ServiceMetrics::new());
    let run_dir = Arc::new(run_dir);
    
    // Initialize components
    info!("\n🔧 Initializing pipeline components...");
    
    // 1. Token tracking subscriber
    info!("📊 Starting token tracking subscriber...");
    let mut token_subscriber = TokenTrackingSubscriber::new(0.1); // 0.1 ETH threshold
    let token_cache = token_subscriber.get_cache();
    
    // Start token subscriber in background
    let subscriber_handle = tokio::spawn(async move {
        if let Err(e) = token_subscriber.start_listening().await {
            error!("Token subscriber error: {}", e);
        }
    });
    
    // Wait for initial cache population
    info!("⏳ Waiting for token cache population...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    let initial_pools = token_cache.pools.get_pool_count().await;
    let initial_creators = token_cache.get_creator_count().await;
    info!("✅ Token cache initialized: {} pools, {} creators", initial_pools, initial_creators);
    
    // 2. IPC client
    info!("\n🔌 Connecting to Reth IPC...");
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");
    
    // 3. Mempool timestamp tracker (optional)
    let mempool_tracker = if let Some(db_url) = &args.database_url {
        info!("📝 Initializing mempool timestamp tracker...");
        
        // Create database connection pool
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await?;
        
        // Create tracker with default config
        let tracker_config = TrackerConfig::default();
        let tracker = MempoolTimestampTracker::new(db_pool, tracker_config).await?;
        
        info!("✅ Mempool timestamp tracker ready");
        Some(tracker)
    } else {
        info!("⚠️  Mempool timestamp tracking disabled (no DATABASE_URL)");
        None
    };
    
    // 4. Function detector
    info!("🔍 Initializing function detector...");
    // Create function detector with custom log directory
    let detector_log_dir = run_dir.join("function_detector");
    std::fs::create_dir_all(&detector_log_dir)?;
    std::env::set_var("FUNCTION_DETECTOR_LOG_DIR", detector_log_dir.to_str().unwrap());
    let function_detector = FunctionDetector::new_with_cache(Some(token_cache.clone()));
    info!("✅ Function detector ready");
    
    // 4. Transaction router
    info!("🚦 Initializing transaction router...");
    let tx_router = TransactionRouter::new(Some(token_cache.clone()));
    info!("✅ Transaction router ready");
    
    // 5. Unified Simulator (single database connection)
    info!("🧪 Initializing unified simulator...");
    let buy_sell_config = BuySellSimulatorConfig::default();
    let unified_simulator = Arc::new(UnifiedSimulator::with_config(&args.reth_db_path, buy_sell_config)?);
    info!("✅ Unified simulator initialized");
    
    // 6. Signal publisher (moved before simulation manager)
    info!("📡 Initializing signal publisher...");
    let signals_dir = run_dir.join("signals");
    std::fs::create_dir_all(&signals_dir)?;
    let publisher_config = SignalPublisherConfig::with_log_dir(signals_dir.to_str().unwrap());
    let signal_publisher = Arc::new(Mutex::new(SignalPublisher::new(publisher_config).await?));
    info!("✅ Signal publisher ready");
    
    // 7. Simulation manager (now includes signal detection and publishing)
    info!("📦 Starting simulation manager with integrated signal detection and publishing...");
    let signal_config = SignalManagerConfig {
        log_dir: signals_dir.clone(),
    };
    let simulation_manager = SimulationManager::new(
        unified_simulator,
        token_cache.clone(),
        signal_config.clone(),
        signal_publisher.clone(),
        args.sim_workers
    );
    
    // Note: set_metric_counters has been removed from SimulationManager
    
    info!("✅ Simulation manager ready with {} workers, signal detection and publishing", args.sim_workers);
    
    info!("\n🏃 Starting main processing loop...\n");
    info!("📁 Run directory: {}", run_dir.display());
    
    // Create a summary log file for high-level metrics
    let summary_log_path = run_dir.join("summary.log");
    let mut summary_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&summary_log_path)?;
    use std::io::Write;
    writeln!(summary_file, "Mempool Signal Detector Run Summary")?;
    writeln!(summary_file, "===================================")?;
    writeln!(summary_file, "Started: {}", chrono::Local::now())?;
    writeln!(summary_file, "Configuration:")?;
    writeln!(summary_file, "  IPC Path: {}", args.ipc_path)?;
    writeln!(summary_file, "  Batch Size: {}", args.batch_size)?;
    writeln!(summary_file, "  Simulation Workers: {}", args.sim_workers)?;
    writeln!(summary_file, "  Report Interval: {}s", args.report_interval)?;
    writeln!(summary_file, "\nPerformance Targets:")?;
    writeln!(summary_file, "  Function Detection: <10μs")?;
    writeln!(summary_file, "  TX Routing: <5μs")?;
    writeln!(summary_file, "  Simulation: <50ms")?;
    writeln!(summary_file, "  End-to-end: <100ms")?;
    writeln!(summary_file, "\n===================================")?;
    writeln!(summary_file, "Real-time Metrics:\n")?;
    
    let summary_log_path = Arc::new(summary_log_path);
    
    // Initialize performance log with header
    let perf_log_path = run_dir.join("performance.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&perf_log_path)
    {
        writeln!(file, "# Performance metrics logged every {} seconds", args.report_interval).ok();
        writeln!(file, "# Format: TX: total (rate/s) | Detect: avg/max μs | Route: avg/max μs | Sim: avg/max ms | CC:contract_creations CA:creator_actions | Sims:ok/err | Signals: TE:trading_enabled LR:liquidity_removal HP:honeypot TC:tax_change").ok();
        writeln!(file, "#").ok();
    }
    
    // Create simulation log path for direct simulation result logging
    let simulation_log_path = Arc::new(run_dir.join("simulation_results.log"));
    
    let mut last_report = Instant::now();
    let mut consecutive_empty = 0u64;
    let start_time = Instant::now();
    
    // Main processing loop
    loop {
        // Check for shutdown signal
        if shutdown.load(Ordering::Relaxed) {
            info!("🛑 Shutdown signal received, stopping gracefully...");
            break;
        }
        
        // Get new transactions
        let new_txs = ipc_client.get_transactions_instant(args.batch_size).await;
        
        if new_txs.is_empty() {
            consecutive_empty += 1;
            
            // Adaptive backoff
            let sleep_time = match consecutive_empty {
                1..=10 => Duration::from_micros(100),
                11..=100 => Duration::from_millis(1), 
                _ => Duration::from_millis(10),
            };
            time::sleep(sleep_time).await;
            continue;
        }
        
        consecutive_empty = 0;
        
        // Record mempool timestamps for all transactions (non-blocking)
        if let Some(ref tracker) = mempool_tracker {
            for tx in &new_txs {
                tracker.record_transaction(tx.hash.clone()).await;
            }
        }
        
        // Process batch through pipeline
        let batch_start = Instant::now();
        
        // Step 1: Function detection
        let detection_start = Instant::now();
        let transactions_with_functions = function_detector.detect_batch(new_txs);
        let detection_time = detection_start.elapsed();
        
        // Step 2: Process each transaction
        for tx in transactions_with_functions {
            let tx_start = Instant::now();
            metrics.total_processed.fetch_add(1, Ordering::Relaxed);
            
            // Record detection latency
            let detection_latency_ns = tx.detection_ns;
            metrics.add_detection_latency(Duration::from_nanos(detection_latency_ns as u64)).await;
            
            // Transaction routing
            let routing_start = Instant::now();
            let classification = tx_router.classify(&tx).await;
            let routing_time = routing_start.elapsed();
            metrics.add_routing_latency(routing_time).await;
            
            // Update category metrics
            match &classification.category {
                TransactionCategory::ContractCreation { .. } => {
                    metrics.contract_creations.fetch_add(1, Ordering::Relaxed);
                }
                TransactionCategory::CreatorTransaction { .. } => {
                    metrics.creator_actions.fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    metrics.regular_txs.fetch_add(1, Ordering::Relaxed);
                    continue; // Skip non-relevant transactions
                }
            }
            
            // Handle non-simulated transactions (like LP approvals)
            if !classification.requires_simulation {
                // Check if this is an LP approval that needs direct signal detection
                if let TransactionCategory::CreatorTransaction { 
                    function_type: CreatorFunctionType::LiquidityManagement, 
                    .. 
                } = &classification.category {
                    // Route LP approval through simulation manager (no simulation, just detection)
                    simulation_manager.detect_lp_approval(&tx, &classification.category).await;
                }
                continue;
            }
            
            // Create simulation request
            let sim_request = SimulationRequest {
                tx: tx.clone(),
                category: classification.category.clone(),
                priority: classification.priority,
                simulation_type: match &classification.category {
                    TransactionCategory::ContractCreation { .. } => SimulationType::TransactionWithBuySell,
                    TransactionCategory::CreatorTransaction { .. } => SimulationType::TransactionWithBuySell,
                    _ => SimulationType::TransactionOnly,
                },
                tx_hash: H256::from_slice(
                    hex::decode(&tx.hash.trim_start_matches("0x"))
                        .unwrap_or_default()
                        .as_slice()
                ),
            };
            
            // Submit for simulation (signal detection happens internally)
            metrics.simulations_submitted.fetch_add(1, Ordering::Relaxed);
            
            let sim_start = Instant::now();
            match simulation_manager.submit(sim_request).await {
                Ok(()) => {
                    metrics.simulations_completed.fetch_add(1, Ordering::Relaxed);
                    let sim_time = sim_start.elapsed();
                    metrics.add_simulation_time(sim_time).await;
                }
                Err(e) => {
                    metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
                    warn!("Simulation submission error for {}: {}", tx.hash, e);
                }
            }
        }
        
        // Process simulation queue
        let simulation_results = simulation_manager.process_queue().await;
        for result in simulation_results {
            // Log simulation result to dedicated file
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(false)
                .append(true)
                .open(simulation_log_path.as_ref())
            {
                let timestamp = chrono::Local::now();
                let tx_hash = format!("{:?}", result.request.tx_hash);
                let category = match &result.request.category {
                    TransactionCategory::ContractCreation { .. } => "ContractCreation",
                    TransactionCategory::CreatorTransaction { .. } => "CreatorTransaction",
                    _ => "Other",
                };
                
                if let Some(ref error) = result.error {
                    writeln!(file, "[{}] ERROR | {} | {} | {}", 
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        tx_hash,
                        category,
                        error
                    ).ok();
                } else {
                    // Log successful simulation with key results
                    let buy_sell_info = if let Some(ref bs) = result.buy_sell_result {
                        format!("CanBuy: {}, CanSell: {}", 
                            bs.can_buy,
                            bs.can_sell
                        )
                    } else {
                        "No buy/sell data".to_string()
                    };
                    
                    writeln!(file, "[{}] SUCCESS | {} | {} | SimTime: {:.1}ms | {}", 
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        tx_hash,
                        category,
                        result.simulation_time_ms,
                        buy_sell_info
                    ).ok();
                }
            }
            
            if let Some(ref error) = result.error {
                metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
                warn!("Simulation error: {}", error);
            } else {
                metrics.simulations_completed.fetch_add(1, Ordering::Relaxed);
                if result.simulation_time_ms > 0.0 {
                    let sim_duration = Duration::from_secs_f64(result.simulation_time_ms / 1000.0);
                    metrics.add_simulation_time(sim_duration).await;
                }
            }
        }
        
        // Periodic reporting
        if last_report.elapsed() > Duration::from_secs(args.report_interval) {
            let elapsed = start_time.elapsed();
            let report = metrics.report(elapsed).await;
            
            // Log to performance file with cleaner format
            let perf_log_path = run_dir.join("performance.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&perf_log_path)
            {
                let timestamp = chrono::Local::now();
                writeln!(file, "[{}] {}", timestamp.format("%H:%M:%S"), report).ok();
            }
            
            // Update summary file with latest high-level metrics
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(false)
                .append(true)
                .open(summary_log_path.as_ref())
            {
                let timestamp = chrono::Local::now();
                let elapsed = start_time.elapsed();
                let total = metrics.total_processed.load(Ordering::Relaxed);
                writeln!(file, "[{}] Runtime: {:.1}min, Total TX: {}, Rate: {:.1} tx/sec",
                    timestamp.format("%H:%M:%S"),
                    elapsed.as_secs_f64() / 60.0,
                    total,
                    total as f64 / elapsed.as_secs_f64()
                ).ok();
            }
            
            // Update cache statistics
            let current_pools = token_cache.pools.get_pool_count().await;
            let current_creators = token_cache.get_creator_count().await;
            if current_pools != initial_pools || current_creators != initial_creators {
                info!("📊 Token cache updated: {} pools (+{}), {} creators (+{})", 
                    current_pools, current_pools.saturating_sub(initial_pools),
                    current_creators, current_creators.saturating_sub(initial_creators));
            }
            
            last_report = Instant::now();
        }
    }
    
    // Graceful shutdown
    let total_runtime = start_time.elapsed();
    info!("\n🛑 Shutting down Mempool Signal Detection Service...");
    
    // Final statistics
    let final_report = metrics.report(total_runtime).await;
    info!("{}", final_report);
    info!("\n📊 Service Statistics:");
    info!("  Total Runtime: {:.1} minutes", total_runtime.as_secs_f64() / 60.0);
    info!("  Average Throughput: {:.1} tx/sec", 
        metrics.total_processed.load(Ordering::Relaxed) as f64 / total_runtime.as_secs_f64());
    
    // Write final summary
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(false)
        .append(true)
        .open(summary_log_path.as_ref())
    {
        writeln!(file, "\n===================================")?;
        writeln!(file, "Run Completed: {}", chrono::Local::now())?;
        writeln!(file, "Total Runtime: {:.1} minutes", total_runtime.as_secs_f64() / 60.0)?;
        writeln!(file, "Total Transactions: {}", metrics.total_processed.load(Ordering::Relaxed))?;
        writeln!(file, "Average Throughput: {:.1} tx/sec", 
            metrics.total_processed.load(Ordering::Relaxed) as f64 / total_runtime.as_secs_f64())?;
        writeln!(file, "\nFinal Metrics: {}", final_report)?;
    }
    
    // Shutdown components
    drop(signal_publisher);
    drop(simulation_manager);
    subscriber_handle.abort();
    
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