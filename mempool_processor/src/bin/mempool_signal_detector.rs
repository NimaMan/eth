#[path = "../mempool_signal_detector_runtime/mod.rs"]
mod mempool_signal_detector_runtime;

use clap::Parser;
use eyre::{bail, Result};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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
use tokio::sync::{mpsc, Mutex};
use tokio::time;
use tracing::{info, warn};
use tracing_subscriber::Layer;

use mempool_signal_detector_runtime::{
    create_arrival_recording_ingress_observer, drain_completed_simulation_outcomes,
    env_flag_enabled, process_signal_path_transactions, retry_cache_waiting_unresolved_intents,
    schedule_latest_simulation_status_log, setup_shutdown_handler, spawn_critical_signal_consumer,
    spawn_detector_timing_watchdog, DetectorLane, DetectorLoopTiming, LocalLogTimeFormatter,
    ServiceMetrics,
};

// Mempool processor imports
use mempool_processor::{
    arrival_recorder::{ArrivalRecorderConfig, MempoolArrivalRecorder},
    config::MempoolProcessorConfig,
    function_detector::FunctionDetector,
    mempool_fetcher::MempoolFetcherIPCClient,
    signal_detector::SignalManagerConfig,
    signal_publisher::{SignalPublisher, SignalPublisherConfig},
    simulator::{MempoolSimulator, SimulationManager, SimulationResult},
    token_tracking::{
        hydrate_cache_from_live_token_server, start_live_token_server_cache_sync, CacheConfig,
        TokenTrackingCache,
    },
    tx_router::TransactionRouter,
    unresolved_intents::UnresolvedIntentStore,
};

const MEMPOOL_ALLOW_DATABASE_DISABLED_ENV: &str = "MEMPOOL_ALLOW_DATABASE_DISABLED";
const UNRESOLVED_INTENT_MAX_ENTRIES: usize = 20_000;
const UNRESOLVED_INTENT_MAX_LIFETIME: Duration = Duration::from_secs(2);
const UNRESOLVED_INTENT_RETRY_INTERVAL: Duration = Duration::from_millis(250);
const LIVE_TX_SIMULATOR_READY_TIMEOUT: Duration = Duration::from_secs(180);

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

    /// Allow a diagnostic run to publish only ZMQ/log signals without Postgres.
    /// Live runs should not set this.
    #[arg(long)]
    allow_database_disabled: bool,
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
        .with_timer(LocalLogTimeFormatter)
        .with_ansi(false)
        .with_filter(EnvFilter::new(main_filter));

    let console_layer = fmt::layer()
        .with_target(false)
        .with_writer(std::io::stdout)
        .with_timer(LocalLogTimeFormatter)
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
    info!(
        "  Database Persistence: {}",
        if base_config.database.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );

    info!("  Report Interval: {}s", cfg_report_interval);
    info!(
        "  Live tx simulator: {}",
        base_config
            .simulation
            .live_tx_simulator_server_url
            .as_deref()
            .unwrap_or("local Reth historical context")
    );
    info!("================================");

    let allow_database_disabled =
        args.allow_database_disabled || env_flag_enabled(MEMPOOL_ALLOW_DATABASE_DISABLED_ENV);
    if !base_config.database.enabled && !allow_database_disabled {
        bail!(
            "mempool_signal_detector live profile requires persisted signal writes. \
             Set {} in blockchains/eth/config.toml, or pass \
             --allow-database-disabled only for a diagnostic ZMQ/log-only run.",
            mempool_processor::config::MEMPOOL_DATABASE_CONFIG_KEY
        );
    }

    // Initialize metrics
    let metrics = Arc::new(ServiceMetrics::new());
    let run_dir = Arc::new(run_dir);

    // Initialize components
    info!("\n🔧 Initializing pipeline components...");

    // 1. Token context from eth_chain_server
    info!("📊 Starting token context sync from eth_chain_server...");
    let source_cfg = &base_config.token_cache_source;
    let token_cache = Arc::new(TokenTrackingCache::new(CacheConfig {
        eth_threshold: source_cfg.eth_threshold,
        ..Default::default()
    }));

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
                    "✅ Live token tracker cache hydrate complete: status={:?}, block={}, tokens={}, pools={}, accepted={}",
                    report.status, report.block_number, report.tokens, report.pools, report.accepted
                );
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
        ))
    } else {
        warn!("📡 Rust live token tracker cache source disabled; token context will remain empty");
        None
    };

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
    let function_detector = FunctionDetector::new_with_cache(Some(token_cache.clone()));
    info!("✅ Function detector ready");

    // 4. Transaction router
    info!("🚦 Initializing transaction router...");
    let tx_router = Arc::new(TransactionRouter::new(Some(token_cache.clone())));
    info!("✅ Transaction router ready");

    // 5. Mempool Simulator (single database connection)
    info!("🧪 Initializing mempool simulator...");
    let mempool_simulator = Arc::new(MempoolSimulator::new_with_chain_server_live_tx_simulator(
        &cfg_reth_db_path,
        base_config
            .simulation
            .live_tx_simulator_server_url
            .as_deref(),
    )?);
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

    if mempool_simulator.chain_server_live_tx_simulator().is_some() {
        wait_for_live_tx_simulator_ready(
            mempool_simulator.as_ref(),
            LIVE_TX_SIMULATOR_READY_TIMEOUT,
        )
        .await?;
    }

    // Start IPC after simulator, arrival recorder, signal publisher, and
    // simulation workers are ready so ingress timestamps are captured and the
    // detector can drain immediately.
    info!("\n🔌 Connecting to Reth IPC...");
    let ingress_observer = create_arrival_recording_ingress_observer(arrival_recorder.clone());
    let ipc_client = MempoolFetcherIPCClient::new_with_observer_and_signal_filter(
        Some(&cfg_ipc_path),
        Some(ingress_observer),
        Some(token_cache.clone()),
    )?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");

    info!("\n🏃 Starting main processing loop...\n");
    info!("📁 Run directory: {}", run_dir.display());

    // Successful simulations are counted in metrics. Only failures get their
    // own file so the run directory stays focused on actionable diagnostics.
    let simulation_error_log_path = Arc::new(run_dir.join("simulation_errors.log"));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(simulation_error_log_path.as_ref())?;
    let unresolved_intent_store = UnresolvedIntentStore::new(
        UNRESOLVED_INTENT_MAX_ENTRIES,
        UNRESOLVED_INTENT_MAX_LIFETIME,
        UNRESOLVED_INTENT_RETRY_INTERVAL,
        run_dir.join("unresolved_intents.log"),
    );
    let critical_detector_timing = DetectorLoopTiming::new();
    spawn_detector_timing_watchdog(
        "critical",
        critical_detector_timing.clone(),
        shutdown.clone(),
    );
    let critical_consumer_handle = spawn_critical_signal_consumer(
        ipc_client.clone(),
        tx_router.clone(),
        simulation_manager.clone(),
        metrics.clone(),
        unresolved_intent_store.clone(),
        critical_detector_timing.clone(),
        shutdown.clone(),
        args.batch_size,
    );

    let mut last_report = Instant::now();
    let mut consecutive_empty = 0u64;
    let mut last_report_total = 0u64;
    let start_time = Instant::now();
    let simulation_status_probe_in_flight = Arc::new(AtomicBool::new(false));
    let detector_timing = DetectorLoopTiming::new();
    spawn_detector_timing_watchdog("normal", detector_timing.clone(), shutdown.clone());

    // Main processing loop
    loop {
        if shutdown.load(Ordering::Relaxed) {
            info!("🛑 Shutdown signal received, stopping gracefully...");
            break;
        }

        {
            let _stage = detector_timing.stage("drain_completed_simulation_outcomes_pre");
            if time::timeout(
                Duration::from_secs(2),
                drain_completed_simulation_outcomes(
                    &mut simulation_result_rx,
                    metrics.as_ref(),
                    mempool_simulator.as_ref(),
                    simulation_error_log_path.as_ref(),
                    &unresolved_intent_store,
                ),
            )
            .await
            .is_err()
            {
                warn!("detector stage timed out lane=normal stage=drain_completed_simulation_outcomes_pre timeout_ms=2000");
            }
        }
        {
            let _stage = detector_timing.stage("retry_cache_waiting_unresolved_intents");
            if time::timeout(
                Duration::from_secs(2),
                retry_cache_waiting_unresolved_intents(
                    &unresolved_intent_store,
                    tx_router.as_ref(),
                    &simulation_manager,
                    metrics.as_ref(),
                ),
            )
            .await
            .is_err()
            {
                warn!("detector stage timed out lane=normal stage=retry_cache_waiting_unresolved_intents timeout_ms=2000");
            }
        }

        let new_txs = {
            let _stage = detector_timing.stage("ipc_get_normal_transactions_instant");
            ipc_client
                .get_normal_transactions_instant(args.batch_size)
                .await
        };
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
            process_signal_path_transactions(
                new_txs,
                &function_detector,
                tx_router.as_ref(),
                &simulation_manager,
                metrics.as_ref(),
                &unresolved_intent_store,
                &detector_timing,
                DetectorLane::Normal,
            )
            .await;
        }

        {
            let _stage = detector_timing.stage("drain_completed_simulation_outcomes_post");
            if time::timeout(
                Duration::from_secs(2),
                drain_completed_simulation_outcomes(
                    &mut simulation_result_rx,
                    metrics.as_ref(),
                    mempool_simulator.as_ref(),
                    simulation_error_log_path.as_ref(),
                    &unresolved_intent_store,
                ),
            )
            .await
            .is_err()
            {
                warn!("detector stage timed out lane=normal stage=drain_completed_simulation_outcomes_post timeout_ms=2000");
            }
        }

        if last_report.elapsed() > Duration::from_secs(cfg_report_interval) {
            let _stage = detector_timing.stage("interval_report");
            let interval_secs = last_report.elapsed().as_secs_f64().max(0.001);
            let total = metrics.total_processed.load(Ordering::Relaxed);
            let delta = total.saturating_sub(last_report_total);
            let rate = delta as f64 / interval_secs;
            let sims = metrics.simulations_completed.load(Ordering::Relaxed);
            let sim_errs = metrics.simulation_errors.load(Ordering::Relaxed);
            let report_result = time::timeout(Duration::from_secs(2), async {
                let publisher_stats = {
                    let publisher = signal_publisher.lock().await;
                    publisher.get_stats()
                };
                let ipc_stats = ipc_client.get_stats().await;
                let manager_stats = simulation_manager.stats().await;

                info!(
                    "📊 Interval stats: {} tx (+{}), {:.1}/s | Sims submitted/done/actionable_err: {}/{}/{} | Signals TE:{} LR:{} LP:{} TAX:{} SELL_BLOCKED:{} SUPPLY_RISK:{} | Published:{} ZMQ:{} DB:{} Err:{}",
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
                    publisher_stats.sell_blocked_signals,
                    publisher_stats.token_supply_risks,
                    publisher_stats.total_published,
                    publisher_stats.zmq_published,
                    publisher_stats.db_written,
                    publisher_stats.errors
                );
                info!(
                    "📊 Mempool ingress: received={} filtered_irrelevant={} dropped={} critical_received={} critical_dropped={} ipc_queue={} normal_queue={} critical_queue={} | simulation_queue current={} enqueued={} processed={} dropped={}",
                    ipc_stats.total,
                    ipc_stats.total_filtered_irrelevant,
                    ipc_stats.total_dropped,
                    ipc_stats.critical_received,
                    ipc_stats.critical_dropped,
                    ipc_stats.queue_size,
                    ipc_stats
                        .queue_size
                        .saturating_sub(ipc_stats.critical_queue_size),
                    ipc_stats.critical_queue_size,
                    manager_stats.queue_current_size,
                    manager_stats.queue_total_enqueued,
                    manager_stats.queue_total_processed,
                    manager_stats.queue_total_dropped
                );
                let timing_snapshot = detector_timing.snapshot();
                info!(
                    "📊 Detector loop timing: loops={} current_stage={} current_stage_ms={} last_loop_ms_ago={} max_stage={} max_stage_ms={} slow_stages={}",
                    timing_snapshot.completed_loops,
                    timing_snapshot.current_stage,
                    timing_snapshot.current_stage_ms,
                    timing_snapshot.last_loop_ms_ago,
                    timing_snapshot.max_stage,
                    timing_snapshot.max_stage_ms,
                    timing_snapshot.slow_stage_count
                );
                let critical_timing_snapshot = critical_detector_timing.snapshot();
                info!(
                    "📊 Critical detector loop timing: loops={} current_stage={} current_stage_ms={} last_loop_ms_ago={} max_stage={} max_stage_ms={} slow_stages={}",
                    critical_timing_snapshot.completed_loops,
                    critical_timing_snapshot.current_stage,
                    critical_timing_snapshot.current_stage_ms,
                    critical_timing_snapshot.last_loop_ms_ago,
                    critical_timing_snapshot.max_stage,
                    critical_timing_snapshot.max_stage_ms,
                    critical_timing_snapshot.slow_stage_count
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
                    "📊 LP approval path: ingress_txs erc20={} ownership_hits={} position={} manager_hits={} cache_misses={} | retry_attempts erc20={} ownership_hits={} position={} manager_hits={} cache_misses={} | published={} db_errors={}",
                    lp_approval_stats.ingress_erc20_approval_txs,
                    lp_approval_stats.ingress_ownership_token_pool_hits,
                    lp_approval_stats.ingress_position_approval_txs,
                    lp_approval_stats.ingress_position_manager_hits,
                    lp_approval_stats.ingress_pool_cache_miss_txs,
                    lp_approval_stats.retry_erc20_approval_attempts,
                    lp_approval_stats.retry_ownership_token_pool_hits,
                    lp_approval_stats.retry_position_approval_attempts,
                    lp_approval_stats.retry_position_manager_hits,
                    lp_approval_stats.retry_pool_cache_miss_attempts,
                    publisher_stats.lp_approvals,
                    publisher_stats.db_errors
                );
                let cache_stats = token_cache.stats().await;
                info!(
                    "📊 Token cache stats: {} tokens, {} pools, {} creators",
                    cache_stats.total_tokens, cache_stats.total_pools, cache_stats.total_creators
                );
                let cache_context = token_cache.context_snapshot().await;
                info!(
                    "📊 Token cache context: block={} status={:?} source={:?} accepted={} rejected_stale={} rejected_non_live={}",
                    cache_context.last_accepted_block,
                    cache_context.last_accepted_status,
                    cache_context.last_accepted_source,
                    cache_context.accepted_updates,
                    cache_context.rejected_stale_snapshots,
                    cache_context.rejected_non_live_snapshots
                );
                let unresolved_stats = unresolved_intent_store.stats().await;
                info!(
                    "📊 Unresolved intents: pending={} in_flight={} recorded={} resolved={} expired={} dropped={} avg_cache_wait_ms={:.1}",
                    unresolved_stats.pending,
                    unresolved_stats.in_flight,
                    unresolved_stats.recorded_total,
                    unresolved_stats.resolved_total,
                    unresolved_stats.expired_total,
                    unresolved_stats.dropped_total,
                    unresolved_stats.cache_wait_avg_ms
                );
                let nonce_dependency_stats =
                    simulation_manager.pending_nonce_dependency_stats().await;
                info!(
                    "📊 Pending nonce dependencies: senders={} txs={} recorded={} replaced={} lookups={} hits={} gaps={} expired={}",
                    nonce_dependency_stats.senders,
                    nonce_dependency_stats.transactions,
                    nonce_dependency_stats.recorded,
                    nonce_dependency_stats.replaced,
                    nonce_dependency_stats.lookups,
                    nonce_dependency_stats.hits,
                    nonce_dependency_stats.gaps,
                    nonce_dependency_stats.expired
                );
                let funding_dependency_stats =
                    simulation_manager.pending_funding_dependency_stats().await;
                info!(
                    "📊 Pending funding dependencies: recipients={} txs={} recorded={} replaced={} lookups={} hits={} gaps={} expired={}",
                    funding_dependency_stats.recipients,
                    funding_dependency_stats.transactions,
                    funding_dependency_stats.recorded,
                    funding_dependency_stats.replaced,
                    funding_dependency_stats.lookups,
                    funding_dependency_stats.hits,
                    funding_dependency_stats.gaps,
                    funding_dependency_stats.expired
                );
            })
            .await;
            if report_result.is_err() {
                warn!("detector stage timed out lane=normal stage=interval_report timeout_ms=2000");
            }
            last_report_total = total;

            schedule_latest_simulation_status_log(
                mempool_simulator.clone(),
                external_data_log_path.clone(),
                simulation_status_probe_in_flight.clone(),
            );
            last_report = Instant::now();
        }

        if let Some(sleep_time) = idle_sleep {
            time::sleep(sleep_time).await;
        }
        detector_timing.mark_loop_completed();
    }

    let total_runtime = start_time.elapsed();
    info!("\n🛑 Shutting down Mempool Signal Detection Service...");
    critical_consumer_handle.abort();
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
        "Publisher signals: TE:{} LR:{} LP:{} TAX:{} SELL_BLOCKED:{} SUPPLY_RISK:{} | Published:{} ZMQ:{} Logs:{} DB:{} Err:{}",
        publisher_stats.trading_enabled,
        publisher_stats.liquidity_removals,
        publisher_stats.lp_approvals,
        publisher_stats.tax_signals,
        publisher_stats.sell_blocked_signals,
        publisher_stats.token_supply_risks,
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

    info!("✅ Mempool signal detector shutdown complete");
    Ok(())
}

async fn wait_for_live_tx_simulator_ready(
    mempool_simulator: &MempoolSimulator,
    timeout: Duration,
) -> Result<()> {
    let started = Instant::now();
    let mut last_log = Instant::now()
        .checked_sub(Duration::from_secs(60))
        .unwrap_or_else(Instant::now);

    loop {
        match mempool_simulator.latest_simulation_status().await {
            Ok(status) if status.uses_tracked_live_state() => {
                info!(
                    "✅ Chain-server live tx simulator ready: block={} hash={:?} source={:?}",
                    status.selected_block_number, status.selected_block_hash, status.source
                );
                return Ok(());
            }
            Ok(status) => {
                if last_log.elapsed() >= Duration::from_secs(5) {
                    info!(
                        "⏳ Waiting for chain-server live tx simulator: selected_block={} source={:?}",
                        status.selected_block_number, status.source
                    );
                    last_log = Instant::now();
                }
            }
            Err(err) => {
                if last_log.elapsed() >= Duration::from_secs(5) {
                    info!("⏳ Waiting for chain-server live tx simulator: {}", err);
                    last_log = Instant::now();
                }
            }
        }

        if started.elapsed() >= timeout {
            bail!(
                "chain-server live tx simulator was not ready after {}s",
                timeout.as_secs()
            );
        }
        time::sleep(Duration::from_secs(1)).await;
    }
}
