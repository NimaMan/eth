use alloy_primitives::B256;
use clap::Parser;
use eyre::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
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
use tokio::sync::{mpsc, Mutex};
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

fn parse_tx_hash_or_zero(hash: &str) -> B256 {
    let trimmed = hash.trim();
    let normalized = if trimmed.starts_with("0x") {
        trimmed.to_string()
    } else {
        format!("0x{trimmed}")
    };
    B256::from_str(&normalized).unwrap_or(B256::ZERO)
}

// Mempool processor imports
use mempool_processor::{
    arrival_recorder::{ArrivalRecorderConfig, MempoolArrivalRecorder},
    config::MempoolProcessorConfig,
    function_detector::CreatorFunctionType,
    function_detector::FunctionDetector,
    mempool_fetcher::{IngressStatsSnapshot, MempoolFetcherIPCClient, MempoolIngressObserver},
    signal_detector::SignalManagerConfig,
    signal_publisher::{SignalPublisher, SignalPublisherConfig},
    simulator::{
        MempoolSimulator, SimulationManager, SimulationResult, SimulationType, TxSimulationJob,
    },
    token_tracking::{
        hydrate_cache_from_live_token_server, start_live_token_server_cache_sync,
        TokenTrackingSubscriber,
    },
    tx_router::{TransactionCategory, TransactionRouter},
};
use tx_simulator::LiveChainCache;

struct ArrivalRecordingIngressObserver {
    arrival_recorder: Option<Arc<MempoolArrivalRecorder>>,
}

impl ArrivalRecordingIngressObserver {
    fn new(arrival_recorder: Option<Arc<MempoolArrivalRecorder>>) -> Self {
        Self { arrival_recorder }
    }
}

impl MempoolIngressObserver for ArrivalRecordingIngressObserver {
    fn tx_received(&self, hash: &str, first_seen_ms: u64, _stats: IngressStatsSnapshot) {
        if let Some(recorder) = self.arrival_recorder.as_ref() {
            recorder.record_hash_hex_ms_at(hash, first_seen_ms);
        }
    }

    fn tx_dropped_queue_full(&self, hash: &str, first_seen_ms: u64, _stats: IngressStatsSnapshot) {
        if let Some(recorder) = self.arrival_recorder.as_ref() {
            recorder.record_hash_hex_ms_at(hash, first_seen_ms);
        }
    }

    fn queue_depth_updated(&self, _stats: IngressStatsSnapshot) {}
}

#[derive(Parser, Debug)]
struct Args {
    /// Optional config file path (TOML)
    #[arg(long, env = "MEMPOOL_CONFIG_PATH")]
    config: Option<String>,
    /// IPC socket path
    #[arg(long, env = "MEMPOOL_IPC_PATH")]
    ipc_path: Option<String>,

    /// Reth database path for simulations
    #[arg(long, env = "MEMPOOL_RETH_DATADIR")]
    reth_db_path: Option<String>,

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

    /// Maximum wall-clock time for one pending tx simulation
    #[arg(long, default_value = "5000")]
    simulation_timeout_ms: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Performance report interval in seconds
    #[arg(long, default_value = "60")]
    report_interval: u64,

    /// Deprecated: arrival index dir now defaults to `<RETH_DATADIR>/reth_index`.
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
    let cfg_ipc_path = args
        .ipc_path
        .clone()
        .unwrap_or_else(|| base_config.ipc.socket_path.clone());
    let cfg_reth_db_path = args
        .reth_db_path
        .clone()
        .unwrap_or_else(|| base_config.simulation.reth_datadir.clone());
    let cfg_log_dir = base_config.logging.log_dir.clone();
    let cfg_report_interval = args.report_interval.max(1);
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
    info!("  Simulation Timeout: {}ms", args.simulation_timeout_ms);

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
        &source_cfg.redis_url,
        &source_cfg.redis_token_prefix,
    );
    let token_cache = token_subscriber.get_cache();

    let mut live_token_server_hydrate_ok = false;
    let live_token_server_sync_handle = if let Some(base_url) = source_cfg
        .live_token_server_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
    {
        info!(
            "📡 Hydrating token cache from Rust live token tracker at {}",
            base_url
        );
        match hydrate_cache_from_live_token_server(token_cache.as_ref(), base_url).await {
            Ok(report) => {
                info!(
                    "✅ Live token tracker cache hydrate complete: status={:?}, block={}, tokens={}, pools={}",
                    report.status, report.block_number, report.tokens, report.pools
                );
                live_token_server_hydrate_ok = report.tokens > 0;
            }
            Err(err) => {
                warn!(
                    "Live token tracker cache hydrate failed for {}: {}",
                    base_url, err
                );
            }
        }

        Some(start_live_token_server_cache_sync(
            token_cache.clone(),
            base_url.to_string(),
            Duration::from_secs(source_cfg.live_token_server_sync_interval_secs.max(1)),
        ))
    } else {
        info!("📡 Rust live token tracker cache source disabled");
        None
    };
    token_subscriber.skip_initial_redis_warmup(live_token_server_hydrate_ok);

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

    let initial_cache_stats = token_cache.stats().await;
    info!(
        "✅ Token cache initialized: {} tokens, {} pools, {} creators",
        initial_cache_stats.total_tokens,
        initial_cache_stats.total_pools,
        initial_cache_stats.total_creators
    );

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

    // Initialize arrival recorder only after simulator (to reuse provider).
    // This index is useful for first-seen latency analytics, but the live
    // mempool signal path should still run if the sidecar index is absent.
    let arrival_recorder: Option<Arc<MempoolArrivalRecorder>> = {
        let index_dir = Path::new(&cfg_reth_db_path).join("reth_index");
        let recorder_result = (|| -> Result<MempoolArrivalRecorder> {
            std::fs::create_dir_all(&index_dir)?;
            let db = std::sync::Arc::new(
                reth_chain_query::reth_index::database::RethIndexDB::open(&index_dir)?,
            );
            let provider_factory = mempool_simulator
                .get_tx_simulator()
                .provider_factory()
                .clone();
            let writer = std::sync::Arc::new(
                reth_chain_query::reth_index::writers::mempool_arrival_writer::MempoolArrivalWriter::new_with_fallbacks(
                    db.clone(),
                    std::sync::Arc::new(provider_factory),
                    cfg_reth_db_path.clone(),
                    mempool_processor::config::eth_rpc_url_from_env(),
                ),
            );
            let cfg = ArrivalRecorderConfig {
                flush_interval: Duration::from_secs(5),
                batch_size: 1000,
                max_entry_age: Duration::from_secs(2 * 24 * 60 * 60),
            };
            Ok(MempoolArrivalRecorder::new(db, writer, cfg))
        })();

        match recorder_result {
            Ok(recorder) => {
                info!(
                    "✅ Arrival recorder initialized at {} (ms precision)",
                    index_dir.display()
                );
                Some(Arc::new(recorder))
            }
            Err(err) => {
                warn!(
                    "Mempool arrival recorder disabled; live mempool processing will continue without first-seen persistence: {}",
                    err
                );
                None
            }
        }
    };

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
    publisher_config.enable_database = base_config.database.enabled;
    publisher_config.database_url = base_config.database.url.clone();
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

    let simulation_timeout = Duration::from_millis(args.simulation_timeout_ms.max(1));
    let worker_count = sim_workers.max(1);
    let (simulation_result_tx, mut simulation_result_rx) =
        mpsc::channel::<SimulationResult>(worker_count * 256);
    let mut simulation_worker_handles = Vec::with_capacity(worker_count);
    for worker_id in 0..worker_count {
        let manager = simulation_manager.clone();
        let result_tx = simulation_result_tx.clone();
        let shutdown_flag = shutdown.clone();
        simulation_worker_handles.push(tokio::spawn(async move {
            loop {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }

                match manager.next_request().await {
                    Some(request) => {
                        let result = manager
                            .simulate_with_timeout(request, simulation_timeout)
                            .await;
                        if result_tx.send(result).await.is_err() {
                            warn!("Simulation worker {} result channel closed", worker_id);
                            break;
                        }
                    }
                    None => time::sleep(Duration::from_millis(2)).await,
                }
            }
            info!("Simulation worker {} stopped", worker_id);
        }));
    }
    drop(simulation_result_tx);
    info!(
        "✅ Started {} background simulation workers with {}ms timeout",
        worker_count,
        simulation_timeout.as_millis()
    );

    // Start IPC after simulator, arrival recorder, signal publisher, and
    // simulation workers are ready so ingress timestamps are captured and the
    // detector can drain immediately.
    info!("\n🔌 Connecting to Reth IPC...");
    let ingress_observer: Arc<dyn MempoolIngressObserver> = Arc::new(
        ArrivalRecordingIngressObserver::new(arrival_recorder.clone()),
    );
    let ipc_client =
        MempoolFetcherIPCClient::new_with_observer(Some(&cfg_ipc_path), Some(ingress_observer))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");

    info!("\n🏃 Starting main processing loop...\n");
    info!("📁 Run directory: {}", run_dir.display());

    // Create simulation log path for direct simulation result logging
    let simulation_log_path = Arc::new(run_dir.join("simulation_results.log"));
    let simulation_error_log_path = Arc::new(run_dir.join("simulation_errors.log"));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(simulation_log_path.as_ref())?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(simulation_error_log_path.as_ref())?;

    let mut last_report = Instant::now();
    let mut consecutive_empty = 0u64;
    let mut last_report_total = 0u64;
    let start_time = Instant::now();

    // Main processing loop
    loop {
        if shutdown.load(Ordering::Relaxed) {
            info!("🛑 Shutdown signal received, stopping gracefully...");
            break;
        }

        drain_simulation_results(
            &mut simulation_result_rx,
            metrics.as_ref(),
            mempool_simulator.as_ref(),
            simulation_log_path.as_ref(),
            simulation_error_log_path.as_ref(),
        )
        .await;

        let new_txs = ipc_client.get_transactions_instant(args.batch_size).await;
        let mut idle_sleep = None;

        if new_txs.is_empty() {
            consecutive_empty += 1;
            idle_sleep = Some(match consecutive_empty {
                1..=10 => Duration::from_micros(100),
                11..=100 => Duration::from_millis(1),
                _ => Duration::from_millis(10),
            });
        } else {
            consecutive_empty = 0;
            let transactions_with_functions = function_detector.detect_batch(new_txs);

            for tx in transactions_with_functions {
                metrics.total_processed.fetch_add(1, Ordering::Relaxed);
                metrics
                    .add_detection_latency(Duration::from_nanos(tx.detection_ns))
                    .await;

                let classification = tx_router.classify(&tx).await;
                match &classification.category {
                    TransactionCategory::ContractCreation { .. }
                    | TransactionCategory::CreatorTransaction { .. } => {}
                    _ => continue,
                }

                if !classification.requires_simulation {
                    if let TransactionCategory::CreatorTransaction {
                        function_type: CreatorFunctionType::LiquidityPoolApproval,
                        ..
                    } = &classification.category
                    {
                        let _published = simulation_manager
                            .detect_lp_approval(&tx, &classification.category)
                            .await;
                    }
                    continue;
                }

                let sim_request = TxSimulationJob {
                    tx: tx.clone(),
                    category: classification.category.clone(),
                    priority: classification.priority,
                    simulation_type: match &classification.category {
                        TransactionCategory::ContractCreation { .. }
                        | TransactionCategory::CreatorTransaction { .. } => {
                            SimulationType::TransactionWithBuySell
                        }
                        _ => SimulationType::TransactionOnly,
                    },
                    tx_hash: parse_tx_hash_or_zero(&tx.hash),
                };

                match simulation_manager.submit(sim_request).await {
                    Ok(()) => {
                        metrics
                            .simulations_submitted
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
                        warn!("Simulation submission error for {}: {}", tx.hash, e);
                    }
                }
            }
        }

        drain_simulation_results(
            &mut simulation_result_rx,
            metrics.as_ref(),
            mempool_simulator.as_ref(),
            simulation_log_path.as_ref(),
            simulation_error_log_path.as_ref(),
        )
        .await;

        if last_report.elapsed() > Duration::from_secs(cfg_report_interval) {
            let interval_secs = last_report.elapsed().as_secs_f64().max(0.001);
            let total = metrics.total_processed.load(Ordering::Relaxed);
            let delta = total.saturating_sub(last_report_total);
            let rate = delta as f64 / interval_secs;
            let sims = metrics.simulations_completed.load(Ordering::Relaxed);
            let sim_errs = metrics.simulation_errors.load(Ordering::Relaxed);
            let publisher_stats = {
                let publisher = signal_publisher.lock().await;
                publisher.get_stats()
            };
            let ipc_stats = ipc_client.get_stats().await;
            let manager_stats = simulation_manager.stats().await;

            info!(
                "📊 Interval stats: {} tx (+{}), {:.1}/s | Sims submitted/ok/err: {}/{}/{} | Signals TE:{} LR:{} LP:{} TAX:{} SCAM:{} | Published:{} ZMQ:{} DB:{} Err:{}",
                total,
                delta,
                rate,
                metrics.simulations_submitted.load(Ordering::Relaxed),
                sims,
                sim_errs,
                publisher_stats.trading_enabled,
                publisher_stats.liquidity_removals,
                publisher_stats.lp_approvals,
                publisher_stats.tax_signals,
                publisher_stats.scam_detections,
                publisher_stats.total_published,
                publisher_stats.zmq_published,
                publisher_stats.db_written,
                publisher_stats.errors
            );
            info!(
                "📊 Mempool ingress: received={} dropped={} ipc_queue={} | simulation_queue current={} enqueued={} processed={} dropped={}",
                ipc_stats.total,
                ipc_stats.total_dropped,
                ipc_stats.queue_size,
                manager_stats.queue_current_size,
                manager_stats.queue_total_enqueued,
                manager_stats.queue_total_processed,
                manager_stats.queue_total_dropped
            );
            if let Some(ref recorder) = arrival_recorder {
                let arrival_stats = recorder.stats();
                info!(
                    "📊 Arrival recorder: seen={} pending={} resolved={} written={} unresolved={} expired={} flush_errors={}",
                    arrival_stats.seen,
                    arrival_stats.pending,
                    arrival_stats.resolved,
                    arrival_stats.written,
                    arrival_stats.unresolved,
                    arrival_stats.expired,
                    arrival_stats.flush_errors
                );
            }
            let lp_approval_stats = tx_router.lp_approval_stats();
            info!(
                "📊 LP approval path: router_approvals={} tracked_pool_approvals={} pool_cache_misses={} published={} db_errors={}",
                lp_approval_stats.router_approvals_seen,
                lp_approval_stats.tracked_pool_approvals,
                lp_approval_stats.pool_cache_misses,
                publisher_stats.lp_approvals,
                publisher_stats.db_errors
            );
            let cache_stats = token_cache.stats().await;
            info!(
                "📊 Token cache stats: {} tokens, {} pools, {} creators",
                cache_stats.total_tokens, cache_stats.total_pools, cache_stats.total_creators
            );
            last_report_total = total;

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
            last_report = Instant::now();
        }

        if let Some(sleep_time) = idle_sleep {
            time::sleep(sleep_time).await;
        }
    }

    let total_runtime = start_time.elapsed();
    info!("\n🛑 Shutting down Mempool Signal Detection Service...");
    for handle in simulation_worker_handles {
        handle.abort();
    }

    let final_report = metrics.report(total_runtime).await;
    info!("{}", final_report);
    let publisher_stats = {
        let publisher = signal_publisher.lock().await;
        publisher.get_stats()
    };
    info!(
        "Publisher signals: TE:{} LR:{} LP:{} TAX:{} SCAM:{} | Published:{} ZMQ:{} Logs:{} DB:{} Err:{}",
        publisher_stats.trading_enabled,
        publisher_stats.liquidity_removals,
        publisher_stats.lp_approvals,
        publisher_stats.tax_signals,
        publisher_stats.scam_detections,
        publisher_stats.total_published,
        publisher_stats.zmq_published,
        publisher_stats.logs_written,
        publisher_stats.db_written,
        publisher_stats.errors
    );
    info!("\n📊 Service Statistics:");
    info!(
        "  Total Runtime: {:.1} minutes",
        total_runtime.as_secs_f64() / 60.0
    );
    info!(
        "  Average Throughput: {:.1} tx/sec",
        metrics.total_processed.load(Ordering::Relaxed) as f64 / total_runtime.as_secs_f64()
    );

    drop(signal_publisher);
    drop(simulation_manager);
    if let Some(handle) = live_token_server_sync_handle {
        handle.abort();
    }
    subscriber_handle.abort();

    info!("✅ Mempool signal detector shutdown complete");
    Ok(())
}

async fn drain_simulation_results(
    receiver: &mut mpsc::Receiver<SimulationResult>,
    metrics: &ServiceMetrics,
    mempool_simulator: &MempoolSimulator,
    simulation_log_path: &Path,
    simulation_error_log_path: &Path,
) -> usize {
    let mut drained = 0usize;
    while let Ok(result) = receiver.try_recv() {
        drained += 1;
        let tx_hash = format!("{:?}", result.request.tx_hash);
        let category = match &result.request.category {
            TransactionCategory::ContractCreation { .. } => "ContractCreation",
            TransactionCategory::CreatorTransaction { .. } => "CreatorTransaction",
            _ => "Other",
        };

        if let Ok(mut file) = OpenOptions::new()
            .create(false)
            .append(true)
            .open(simulation_log_path)
        {
            let timestamp = chrono::Local::now();

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
                let buy_sell_info = if let Some(ref pool_result) = result.pool_viability_result {
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
                let block_str = match mempool_simulator.latest_simulation_block().await {
                    Ok(b) => b.to_string(),
                    Err(_) => "unknown".to_string(),
                };
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(simulation_error_log_path)
                {
                    let timestamp = chrono::Local::now();
                    writeln!(
                        file,
                        "[{}] ERROR | block={} | {} | {} | {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        block_str,
                        tx_hash,
                        category,
                        error
                    )
                    .ok();
                }
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
    drained
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
