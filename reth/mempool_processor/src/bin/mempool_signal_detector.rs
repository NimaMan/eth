use clap::Parser;
use eyre::Result;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
/// Mempool Signal Detector Service
///
/// Production service that implements the complete signal detection pipeline:
/// 1. Receives transactions from Reth IPC
/// 2. Detects function signatures
/// 3. Routes transactions by category (contract creation, creator actions)
/// 4. Simulates relevant transactions
/// 5. Detects signals (trading enabled, liquidity removal, honeypots, etc.)
/// 6. Publishes signals via ZMQ and logs and writes to database
///
/// Performance targets:
/// - Function detection: <10μs per transaction
/// - TX routing: <5μs per transaction  
/// - Simulation: <50ms per transaction (Reth bottleneck)
/// - Signal detection: <1ms per result
/// - End-to-end: <100ms for critical signals
use std::time::{Duration, Instant};
use tokio::signal;
use tokio::sync::Mutex;
use tokio::time;
use tracing::{error, info, warn};
use tracing_subscriber::Layer;

#[derive(Clone, Debug, Default)]
struct LocalTimeFormatter;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimeFormatter {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = chrono::Local::now();
        write!(w, "[{}]", now.format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

// Mempool processor imports
use ethers::types::H256;
use hex;
use mempool_processor::{
    arrival_recorder::{ArrivalRecorderConfig, MempoolArrivalRecorder},
    config::MempoolProcessorConfig,
    function_detector::CreatorFunctionType,
    function_detector::FunctionDetector,
    mempool_fetcher::MempoolFetcherIPCClient,
    signal_detector::SignalManagerConfig,
    signal_publisher::{SignalPublisher, SignalPublisherConfig},
    simulator::{MempoolSimulator, SimulationManager, SimulationType, TxSimulationJob},
    token_tracking::TokenTrackingSubscriber,
    tx_router::{SimulationPriority, TransactionCategory, TransactionRouter},
};
use tx_simulator::LiveChainCache;

#[derive(Parser, Debug)]
struct Args {
    /// Optional config file path (TOML)
    #[arg(long, env = "MEMPOOL_CONFIG_PATH")]
    config: Option<String>,
    /// IPC socket path
    #[arg(
        long,
        env = "IPC_PATH",
        default_value_t = mempool_processor::config::DEFAULT_RETH_IPC_PATH.to_string()
    )]
    ipc_path: String,

    /// Reth database path for simulations
    #[arg(
        long,
        env = "RETH_DB_PATH",
        default_value_t = mempool_processor::config::DEFAULT_RETH_DATA_DIR.to_string()
    )]
    reth_db_path: String,

    /// Log directory base path
    #[arg(long, default_value = mempool_processor::config::DEFAULT_LOG_DIR)]
    log_dir: String,

    /// Batch size for transaction processing
    #[arg(long, default_value = "100")]
    batch_size: usize,

    /// Simulation worker threads
    #[arg(
        long,
        default_value_t = mempool_processor::config::DEFAULT_SIM_WORKERS
    )]
    sim_workers: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Performance report interval in seconds
    #[arg(long, default_value = "60")]
    report_interval: u64,

    /// Deprecated: arrival index dir now defaults to `<RETH_DB_PATH>/reth_index`.
    /// Kept for compatibility but ignored.
    #[arg(long, env = "ARRIVAL_INDEX_DIR")]
    _arrival_index_dir: Option<String>,
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

    async fn add_simulation_time(&self, time: Duration) {
        let mut times = self.simulation_times.lock().await;
        if times.len() >= 1000 {
            times.drain(0..500);
        }
        times.push(time);
    }

    async fn calculate_latency_stats(
        &self,
        latencies: &[Duration],
    ) -> (Duration, Duration, Duration) {
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
        let (avg_detect, max_detect, _p99_detect) =
            self.calculate_latency_stats(&detection_latencies).await;
        drop(detection_latencies);

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
            "TX: {} ({:.1}/s) | Detect: {}μs/{}μs | Sim: {:.1}ms/{:.1}ms | CC:{} CA:{} | Sims:{}/{} | Signals: TE:{} LR:{} HP:{} TC:{}",
            total, rate,
            avg_detect.as_micros(), max_detect.as_micros(),
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

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up shutdown signal handler
    let shutdown = setup_shutdown_handler();

    // Load configuration (config file -> env/defaults)
    let base_config = if let Some(ref path) = args.config {
        MempoolProcessorConfig::from_file(path)
            .unwrap_or_else(|_| MempoolProcessorConfig::from_env())
    } else {
        MempoolProcessorConfig::from_env()
    };

    // Resolve key paths from config (CLI may still print separate values)
    let cfg_ipc_path = base_config.ipc.socket_path.clone();
    let cfg_reth_db_path = base_config.simulation.reth_datadir.clone();
    let cfg_log_dir = base_config.logging.log_dir.clone();
    let cfg_report_interval = base_config.logging.metrics_interval.as_secs();
    let cfg_sim_workers = base_config.simulation.worker_threads;

    let sim_workers = if args.sim_workers == mempool_processor::config::DEFAULT_SIM_WORKERS
        && cfg_sim_workers != mempool_processor::config::DEFAULT_SIM_WORKERS
    {
        cfg_sim_workers
    } else {
        args.sim_workers
    };

    // Create timestamped run directory
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let run_dir = PathBuf::from(&cfg_log_dir).join(format!("signal_detector_{}", timestamp));
    std::fs::create_dir_all(&run_dir)?;

    // Initialize logging to run directory
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // Main log file for general logs
    let main_file_appender = RollingFileAppender::builder()
        .rotation(Rotation::NEVER) // Single file per run
        .filename_prefix("signal_detector")
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
        .with_timer(LocalTimeFormatter::default())
        .with_ansi(false)
        .with_filter(EnvFilter::new(main_filter));

    let console_layer = fmt::layer()
        .with_target(false)
        .with_writer(std::io::stdout)
        .with_timer(LocalTimeFormatter::default())
        .with_filter(EnvFilter::new(main_filter));

    // Combine layers
    tracing_subscriber::registry()
        .with(main_layer)
        .with(console_layer)
        .init();

    info!("🚀 Starting Mempool Signal Detection Service");
    info!("================================");
    info!("Configuration:");
    info!("  IPC Path: {}", cfg_ipc_path);
    info!("  Reth DB: {}", cfg_reth_db_path);
    info!("  Log Directory: {}", cfg_log_dir);
    info!("  Batch Size: {}", args.batch_size);
    info!("  Simulation Workers: {}", sim_workers);

    info!("  Report Interval: {}s", cfg_report_interval);
    let live_chain_cache = match LiveChainCache::new(&base_config.simulation.live_data_redis_url) {
        Ok(cache) => {
            info!(
                "  Live data Redis: {}",
                base_config.simulation.live_data_redis_url
            );
            Some(cache)
        }
        Err(err) => {
            warn!(
                "Live chain cache unavailable ({}); simulations will use MDBX-only context",
                err
            );
            None
        }
    };
    info!("================================");

    // Initialize metrics
    let metrics = Arc::new(ServiceMetrics::new());
    let run_dir = Arc::new(run_dir);

    // Initialize components
    info!("\n🔧 Initializing pipeline components...");

    // 1. Token tracking subscriber
    info!("📊 Starting token tracking subscriber...");
    let source_cfg = &base_config.token_cache_source;
    let mut token_subscriber = TokenTrackingSubscriber::with_sources(
        source_cfg.eth_threshold,
        &source_cfg.zmq_pub_endpoint,
        &source_cfg.zmq_rep_endpoint,
        &source_cfg.redis_url,
        &source_cfg.redis_token_prefix,
    );
    let token_cache = token_subscriber.get_cache();

    // Start token subscriber in background
    let subscriber_handle = tokio::spawn(async move {
        if let Err(e) = token_subscriber.start_listening().await {
            error!("Token subscriber error: {}", e);
        }
    });

    // Wait for initial cache population
    info!("⏳ Waiting for token cache population...");
    std::thread::sleep(Duration::from_secs(3));
    info!("🔁 Warmup wait complete; reading token cache stats...");

    let initial_pools = token_cache.get_pool_count().await;
    let initial_creators = token_cache.get_creator_count().await;
    info!(
        "✅ Token cache initialized: {} pools, {} creators",
        initial_pools, initial_creators
    );

    // 2. IPC client
    info!("\n🔌 Connecting to Reth IPC...");
    let ipc_client = MempoolFetcherIPCClient::new(Some(&cfg_ipc_path))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");

    // Trading signal writer is now integrated into SignalPublisher
    // Database writing happens automatically when signals are published

    // 4. Function detector
    info!("🔍 Initializing function detector...");
    // Create function detector with custom log directory
    let detector_log_dir = run_dir.join("function_detector");
    std::fs::create_dir_all(&detector_log_dir)?;
    std::env::set_var(
        "FUNCTION_DETECTOR_LOG_DIR",
        detector_log_dir.to_str().unwrap(),
    );
    let function_detector = FunctionDetector::new_with_cache(Some(token_cache.clone()));
    info!("✅ Function detector ready");

    // 4. Transaction router
    info!("🚦 Initializing transaction router...");
    let tx_router = TransactionRouter::new(Some(token_cache.clone()));
    info!("✅ Transaction router ready");

    // 5. Mempool Simulator (single database connection)
    info!("🧪 Initializing mempool simulator...");
    let mempool_simulator = Arc::new(MempoolSimulator::new(&cfg_reth_db_path, live_chain_cache)?);
    info!("✅ Mempool simulator initialized");

    // Initialize arrival recorder only after simulator (to reuse provider)
    let index_dir = Path::new(&cfg_reth_db_path).join("reth_index");
    std::fs::create_dir_all(&index_dir)?;
    let db = std::sync::Arc::new(reth_chain_query::reth_index::database::RethIndexDB::open(
        &index_dir,
    )?);
    let provider_factory = mempool_simulator
        .get_tx_simulator()
        .provider_factory()
        .clone();
    let writer = std::sync::Arc::new(
        reth_chain_query::reth_index::writers::mempool_arrival_writer::MempoolArrivalWriter::new(
            db.clone(),
            std::sync::Arc::new(provider_factory),
        ),
    );
    let cfg = ArrivalRecorderConfig {
        flush_interval: Duration::from_secs(5),
        batch_size: 1000,
        max_entry_age: Duration::from_secs(2 * 24 * 60 * 60),
    };
    let arrival_recorder = Some(MempoolArrivalRecorder::new(db, writer, cfg));
    info!(
        "✅ Arrival recorder initialized at {} (ms precision)",
        index_dir.display()
    );

    // 6. Signal publisher (moved before simulation manager)
    info!("📡 Initializing signal publisher...");
    let signals_dir = run_dir.join("signals");
    std::fs::create_dir_all(&signals_dir)?;
    let external_data_log_path = run_dir.join("external_data_updates.log");
    token_cache
        .set_log_path(external_data_log_path.clone())
        .await;
    let mut publisher_config = SignalPublisherConfig::with_log_dir(signals_dir.to_str().unwrap());
    publisher_config.zmq_endpoint = base_config.zmq.signal_endpoint.clone();
    let db_enabled = publisher_config.enable_database;
    let signal_publisher = Arc::new(Mutex::new(SignalPublisher::new(publisher_config).await?));
    info!(
        "✅ Signal publisher ready with database writing {}",
        if db_enabled { "enabled" } else { "disabled" }
    );

    // 7. Simulation manager (now includes signal detection and publishing)
    info!("📦 Starting simulation manager with integrated signal detection and publishing...");
    // Load configuration (from file or defaults)
    let config = base_config.clone();

    let signal_config = SignalManagerConfig {
        log_dir: signals_dir.clone(),
        tax_detection: config.tax_detection.clone(),
    };
    let simulation_manager = SimulationManager::new(
        mempool_simulator.clone(),
        token_cache.clone(),
        signal_config.clone(),
        signal_publisher.clone(),
        sim_workers,
    );

    // Note: set_metric_counters has been removed from SimulationManager
    info!(
        "✅ Simulation manager ready with {} workers, signal detection and publishing",
        sim_workers
    );

    info!("\n🏃 Starting main processing loop...\n");
    info!("📁 Run directory: {}", run_dir.display());

    // Create simulation log path for direct simulation result logging
    use std::io::Write;
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

        // Record mempool arrival timestamps in ms when first seen
        if let Some(ref recorder) = arrival_recorder {
            for tx in &new_txs {
                recorder.record_hash_hex_ms(&tx.hash);
            }
        }

        // Step 1: Function detection
        let transactions_with_functions = function_detector.detect_batch(new_txs);

        // Step 2: Process each transaction
        for tx in transactions_with_functions {
            // Record earliest arrival if not already recorded
            if let Some(ref recorder) = arrival_recorder {
                recorder.record_hash_hex_ms(&tx.hash);
            }
            metrics.total_processed.fetch_add(1, Ordering::Relaxed);

            // Record detection latency
            let detection_latency_ns = tx.detection_ns;
            metrics
                .add_detection_latency(Duration::from_nanos(detection_latency_ns as u64))
                .await;

            // Transaction routing
            let classification = tx_router.classify(&tx).await;

            // Skip regular transactions we don't care about
            match &classification.category {
                TransactionCategory::ContractCreation { .. }
                | TransactionCategory::CreatorTransaction { .. } => {
                    // Process these transactions
                }
                _ => {
                    continue; // Skip non-relevant transactions
                }
            }

            // Handle non-simulated transactions (like LP approvals)
            if !classification.requires_simulation {
                // Check if this is an LP approval that needs direct signal detection
                if let TransactionCategory::CreatorTransaction {
                    function_type: CreatorFunctionType::LiquidityPoolApproval,
                    ..
                } = &classification.category
                {
                    // Route LP approval through simulation manager (no simulation, just detection)
                    simulation_manager
                        .detect_lp_approval(&tx, &classification.category)
                        .await;

                    // Schedule a follow-up buy/sell check once liquidity lands on-chain.
                    let followup_job = TxSimulationJob {
                        tx: tx.clone(),
                        category: classification.category.clone(),
                        priority: SimulationPriority::High,
                        simulation_type: SimulationType::BuySellOnly,
                        tx_hash: H256::from_slice(
                            hex::decode(&tx.hash.trim_start_matches("0x"))
                                .unwrap_or_default()
                                .as_slice(),
                        ),
                    };
                    simulation_manager.schedule_buy_sell_follow_up(followup_job);
                }
                continue;
            }

            // Create simulation request
            let sim_request = TxSimulationJob {
                tx: tx.clone(),
                category: classification.category.clone(),
                priority: classification.priority,
                simulation_type: match &classification.category {
                    TransactionCategory::ContractCreation { .. } => {
                        SimulationType::TransactionWithBuySell
                    }
                    TransactionCategory::CreatorTransaction { .. } => {
                        SimulationType::TransactionWithBuySell
                    }
                    _ => SimulationType::TransactionOnly,
                },
                tx_hash: H256::from_slice(
                    hex::decode(&tx.hash.trim_start_matches("0x"))
                        .unwrap_or_default()
                        .as_slice(),
                ),
            };

            // Submit for simulation (signal detection happens internally)
            metrics
                .simulations_submitted
                .fetch_add(1, Ordering::Relaxed);

            let sim_start = Instant::now();
            match simulation_manager.submit(sim_request).await {
                Ok(()) => {
                    metrics
                        .simulations_completed
                        .fetch_add(1, Ordering::Relaxed);
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
                    writeln!(
                        file,
                        "[{}] ERROR | {} | {} | {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        tx_hash,
                        category,
                        error
                    )
                    .ok();
                } else {
                    // Log successful simulation with key results
                    let buy_sell_info = if let Some(ref pool_result) = result.pool_viability_result
                    {
                        format!(
                            "CanBuy: {}, CanSell: {}, BuyTax: {:.2}%, SellTax: {:.2}%",
                            pool_result.can_buy,
                            pool_result.can_sell,
                            pool_result.buy_tax_percent,
                            pool_result.sell_tax_percent
                        )
                    } else {
                        "No buy/sell data".to_string()
                    };

                    writeln!(
                        file,
                        "[{}] SUCCESS | {} | {} | SimTime: {:.1}ms | {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        tx_hash,
                        category,
                        result.simulation_time_ms,
                        buy_sell_info
                    )
                    .ok();
                }
            }

            if let Some(ref error) = result.error {
                metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
                if !error.contains("No pools found for token") {
                    warn!("Simulation error: {}", error);
                }
            } else {
                metrics
                    .simulations_completed
                    .fetch_add(1, Ordering::Relaxed);
                if result.simulation_time_ms > 0.0 {
                    let sim_duration = Duration::from_secs_f64(result.simulation_time_ms / 1000.0);
                    metrics.add_simulation_time(sim_duration).await;
                }
            }
        }

        // Periodic reporting (only refresh cache stats now)
        if last_report.elapsed() > Duration::from_secs(cfg_report_interval) {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            match mempool_simulator.latest_simulation_block().await {
                Ok(latest_block) => {
                    let line = format!(
                        "[{}]  INFO 📡 Latest simulation block target: {}",
                        timestamp, latest_block
                    );
                    append_line_to_file(&external_data_log_path, &line);
                }
                Err(err) => {
                    let line = format!(
                        "[{}]  WARN 📡 Unable to determine latest simulation block: {}",
                        timestamp, err
                    );
                    append_line_to_file(&external_data_log_path, &line);
                }
            }
            // Touch cache metrics to maintain interval cadence alongside head tracking.
            let _ = token_cache.get_pool_count().await;
            let _ = token_cache.get_creator_count().await;
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
    info!(
        "  Total Runtime: {:.1} minutes",
        total_runtime.as_secs_f64() / 60.0
    );
    info!(
        "  Average Throughput: {:.1} tx/sec",
        metrics.total_processed.load(Ordering::Relaxed) as f64 / total_runtime.as_secs_f64()
    );

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
            signal::ctrl_c().await.unwrap_or_else(|e| {
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

fn append_line_to_file(path: &Path, line: &str) {
    if let Err(err) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| {
            use std::io::Write;
            writeln!(file, "{}", line)
        })
    {
        warn!(
            "Failed to write external update log at {}: {}",
            path.display(),
            err
        );
    }
}
