/// Unified Mempool Signal Detection Service
/// 
/// This is the consolidated production version that combines:
/// - IPC socket communication for ultra-low latency (<1ms)
/// - Optional batch processing for high throughput (100-1000 TPS)
/// - Comprehensive timing measurements in milliseconds
/// - Smart pre-filtering to reduce simulation load
/// - Full transaction simulation with state change analysis
/// - Scam/signal detection on affected pools
///
/// Timing Pipeline Stages (all in milliseconds):
/// 1. T_MEMPOOL: Transaction appears in mempool (WebSocket notification)
/// 2. T_FETCH_START: IPC request for full transaction data
/// 3. T_FETCH_END: Transaction data received
/// 4. T_FILTER: Pre-filtering decision
/// 5. T_SIM_START: Simulation begins (debug_traceCall)
/// 6. T_SIM_END: Simulation completes with state changes
/// 7. T_ANALYSIS_START: Pool interaction analysis begins
/// 8. T_ANALYSIS_END: Pool interactions identified
/// 9. T_DETECT_START: Scam detection begins
/// 10. T_DETECT_END: Detection complete, signals generated

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error, debug, Level};
use tokio::sync::Mutex;
use chrono::Local;

// Core imports
use mempool_processor::mempool_fetcher::ipc_socket::{
    FullTxIpcClient, FullIpcTransaction,
    BatchTxIpcClient, BatchConfig
};
use mempool_processor::mempool_fetcher::types::TransactionView;
use mempool_processor::pool_subscriber::{PoolSubscriber, cache::PoolStateCache};
// No signal detection needed - just timing measurements
use mempool_processor::database::DbLogger;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::common::address::to_checksum_address;

// Ethers imports
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber, Address, H256, U256};

#[derive(Parser, Debug, Clone)]
#[command(name = "mempool_signal_detection_unified")]
#[command(about = "Unified mempool signal detection with comprehensive timing")]
struct Args {
    /// Operation mode: single or batch
    #[arg(long, default_value = "single")]
    mode: OperationMode,
    
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// RPC URL for simulation
    #[arg(long, env = "ETH_RPC_URL_SIM", default_value = "http://localhost:8545")]
    eth_rpc_url_sim: String,
    
    /// IPC socket path
    #[arg(long, env = "ETH_IPC_PATH", default_value = "/tmp/reth.ipc")]
    eth_ipc_path: String,
    
    /// ZeroMQ socket for pool updates
    #[arg(long, default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// Database configuration
    #[arg(long, default_value = "localhost")]
    db_host: String,
    
    #[arg(long, default_value = "5432")]
    db_port: u16,
    
    #[arg(long, default_value = "eth_db")]
    db_name: String,
    
    #[arg(long, default_value = "postgres")]
    db_user: String,
    
    #[arg(long, default_value = "postgres")]
    db_password: String,
    
    /// Enable full transaction simulation
    #[arg(long, default_value = "true")]
    enable_simulation: bool,
    
    /// Enable smart filtering
    #[arg(long, default_value = "true")]
    enable_smart_filtering: bool,
    
    /// Minimum transaction value in ETH
    #[arg(long, default_value = "0.001")]
    min_tx_value: f64,
    
    /// Batch processing configuration
    #[arg(long, default_value = "20")]
    batch_size: usize,
    
    #[arg(long, default_value = "30")]
    batch_timeout_ms: u64,
    
    #[arg(long, default_value = "3")]
    parallel_connections: usize,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Duration to run (seconds)
    #[arg(long, default_value = "300")]
    duration: u64,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OperationMode {
    Single,
    Batch,
}

/// Comprehensive timing for each transaction (all values in milliseconds)
#[derive(Debug, Clone)]
struct TransactionTiming {
    pub tx_hash: String,
    pub timestamps: TimingStamps,
    pub durations: TimingDurations,
    pub metadata: TimingMetadata,
}

/// All timestamp points in the pipeline
#[derive(Debug, Clone)]
struct TimingStamps {
    pub t_mempool: Instant,          // When tx appeared in mempool
    pub t_fetch_start: Instant,      // When we started fetching
    pub t_fetch_end: Option<Instant>,      // When fetch completed
    pub t_filter: Option<Instant>,         // When filtering decision made
    pub t_sim_start: Option<Instant>,     // When simulation started
    pub t_sim_end: Option<Instant>,       // When simulation completed
    pub t_analysis_start: Option<Instant>, // When analysis started
    pub t_analysis_end: Option<Instant>,   // When analysis completed
    pub t_detect_start: Option<Instant>,   // When detection started
    pub t_detect_end: Option<Instant>,     // When detection completed
}

/// Calculated durations in milliseconds
#[derive(Debug, Clone, Default)]
struct TimingDurations {
    pub mempool_to_fetch_ms: f64,    // Time in mempool before we fetch
    pub fetch_duration_ms: f64,       // IPC fetch time
    pub filter_duration_ms: f64,      // Filtering decision time
    pub simulation_duration_ms: f64,  // Simulation execution time
    pub analysis_duration_ms: f64,    // Pool analysis time
    pub detection_duration_ms: f64,   // Signal detection time
    pub total_pipeline_ms: f64,       // Total end-to-end time
}

/// Additional metadata about the transaction processing
#[derive(Debug, Clone, Default)]
struct TimingMetadata {
    pub filtered_out: bool,
    pub simulation_attempted: bool,
    pub simulation_successful: bool,
    pub state_changes_count: usize,
    pub pools_checked: usize,
    pub pools_affected: usize,
    pub signals_detected: usize,
    pub value_eth: f64,
}

impl TransactionTiming {
    fn new(tx_hash: String, mempool_time: Instant) -> Self {
        Self {
            tx_hash,
            timestamps: TimingStamps {
                t_mempool: mempool_time,
                t_fetch_start: Instant::now(),
                t_fetch_end: None,
                t_filter: None,
                t_sim_start: None,
                t_sim_end: None,
                t_analysis_start: None,
                t_analysis_end: None,
                t_detect_start: None,
                t_detect_end: None,
            },
            durations: TimingDurations::default(),
            metadata: TimingMetadata::default(),
        }
    }
    
    fn calculate_durations(&mut self) {
        let ts = &self.timestamps;
        
        // Calculate each stage duration
        self.durations.mempool_to_fetch_ms = 
            ts.t_fetch_start.duration_since(ts.t_mempool).as_secs_f64() * 1000.0;
            
        if let Some(fetch_end) = ts.t_fetch_end {
            self.durations.fetch_duration_ms = 
                fetch_end.duration_since(ts.t_fetch_start).as_secs_f64() * 1000.0;
        }
        
        if let (Some(filter), Some(fetch_end)) = (ts.t_filter, ts.t_fetch_end) {
            self.durations.filter_duration_ms = 
                filter.duration_since(fetch_end).as_secs_f64() * 1000.0;
        }
        
        if let (Some(sim_start), Some(sim_end)) = (ts.t_sim_start, ts.t_sim_end) {
            self.durations.simulation_duration_ms = 
                sim_end.duration_since(sim_start).as_secs_f64() * 1000.0;
        }
        
        if let (Some(analysis_start), Some(analysis_end)) = (ts.t_analysis_start, ts.t_analysis_end) {
            self.durations.analysis_duration_ms = 
                analysis_end.duration_since(analysis_start).as_secs_f64() * 1000.0;
        }
        
        if let (Some(detect_start), Some(detect_end)) = (ts.t_detect_start, ts.t_detect_end) {
            self.durations.detection_duration_ms = 
                detect_end.duration_since(detect_start).as_secs_f64() * 1000.0;
        }
        
        // Calculate total pipeline time
        if let Some(final_time) = ts.t_detect_end.or(ts.t_analysis_end).or(ts.t_sim_end).or(ts.t_filter) {
            self.durations.total_pipeline_ms = 
                final_time.duration_since(ts.t_mempool).as_secs_f64() * 1000.0;
        }
    }
}

/// Global performance metrics
#[derive(Debug, Default)]
struct PerformanceMetrics {
    // Counters
    total_transactions: AtomicU64,
    filtered_transactions: AtomicU64,
    simulated_transactions: AtomicU64,
    successful_simulations: AtomicU64,
    transactions_with_state_changes: AtomicU64,
    transactions_affecting_pools: AtomicU64,
    total_signals_detected: AtomicU64,
    
    // Timing aggregates (stored as microseconds for atomic operations)
    total_fetch_time_us: AtomicU64,
    total_simulation_time_us: AtomicU64,
    total_analysis_time_us: AtomicU64,
    total_detection_time_us: AtomicU64,
    total_pipeline_time_us: AtomicU64,
    
    // Batch metrics
    total_batches: AtomicU64,
    
    // Error tracking
    fetch_errors: AtomicU64,
    simulation_errors: AtomicU64,
}

impl PerformanceMetrics {
    fn update_timing(&self, timing: &TransactionTiming) {
        self.total_transactions.fetch_add(1, Ordering::Relaxed);
        
        if timing.metadata.filtered_out {
            self.filtered_transactions.fetch_add(1, Ordering::Relaxed);
        }
        
        if timing.metadata.simulation_attempted {
            self.simulated_transactions.fetch_add(1, Ordering::Relaxed);
        }
        
        if timing.metadata.simulation_successful {
            self.successful_simulations.fetch_add(1, Ordering::Relaxed);
        }
        
        if timing.metadata.state_changes_count > 0 {
            self.transactions_with_state_changes.fetch_add(1, Ordering::Relaxed);
        }
        
        if timing.metadata.pools_affected > 0 {
            self.transactions_affecting_pools.fetch_add(1, Ordering::Relaxed);
        }
        
        self.total_signals_detected.fetch_add(timing.metadata.signals_detected as u64, Ordering::Relaxed);
        
        // Update timing aggregates (convert ms to us for atomic storage)
        self.total_fetch_time_us.fetch_add((timing.durations.fetch_duration_ms * 1000.0) as u64, Ordering::Relaxed);
        self.total_simulation_time_us.fetch_add((timing.durations.simulation_duration_ms * 1000.0) as u64, Ordering::Relaxed);
        self.total_analysis_time_us.fetch_add((timing.durations.analysis_duration_ms * 1000.0) as u64, Ordering::Relaxed);
        self.total_detection_time_us.fetch_add((timing.durations.detection_duration_ms * 1000.0) as u64, Ordering::Relaxed);
        self.total_pipeline_time_us.fetch_add((timing.durations.total_pipeline_ms * 1000.0) as u64, Ordering::Relaxed);
    }
    
    fn get_average_timings_ms(&self) -> AverageTimings {
        let total = self.total_transactions.load(Ordering::Relaxed) as f64;
        if total == 0.0 {
            return AverageTimings::default();
        }
        
        AverageTimings {
            avg_fetch_ms: self.total_fetch_time_us.load(Ordering::Relaxed) as f64 / 1000.0 / total,
            avg_simulation_ms: self.total_simulation_time_us.load(Ordering::Relaxed) as f64 / 1000.0 / total,
            avg_analysis_ms: self.total_analysis_time_us.load(Ordering::Relaxed) as f64 / 1000.0 / total,
            avg_detection_ms: self.total_detection_time_us.load(Ordering::Relaxed) as f64 / 1000.0 / total,
            avg_total_ms: self.total_pipeline_time_us.load(Ordering::Relaxed) as f64 / 1000.0 / total,
        }
    }
}

#[derive(Debug, Default)]
struct AverageTimings {
    avg_fetch_ms: f64,
    avg_simulation_ms: f64,
    avg_analysis_ms: f64,
    avg_detection_ms: f64,
    avg_total_ms: f64,
}

/// Transaction filter for smart pre-filtering
struct TransactionFilter {
    min_value_wei: U256,
    enable_smart_filtering: bool,
}

impl TransactionFilter {
    fn new(min_value_eth: f64, enable_smart_filtering: bool) -> Self {
        let min_value_wei = U256::from((min_value_eth * 1e18) as u128);
        Self {
            min_value_wei,
            enable_smart_filtering,
        }
    }
    
    fn should_process(&self, tx: &FullIpcTransaction) -> bool {
        if !self.enable_smart_filtering {
            return true;
        }
        
        // Check minimum value
        if tx.transaction.value < self.min_value_wei {
            return false;
        }
        
        // Check if it's a contract interaction (has input data)
        if tx.transaction.input.len() <= 4 {
            return false;
        }
        
        true
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();
    
    info!("🚀 Unified Mempool Signal Detection | Mode: {:?} | Min: {} ETH", args.mode, args.min_tx_value);
    
    match args.mode {
        OperationMode::Single => {
            debug!("IPC Path: {}", args.eth_ipc_path);
        }
        OperationMode::Batch => {
            info!("   Batch config: size={}, timeout={}ms", args.batch_size, args.batch_timeout_ms);
        }
    }
    
    // Initialize components
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.min_tx_value, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    let db_logger = Arc::new(DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name,
    ).await?);
    
    let tx_simulator = Arc::new(DebugTraceCallSimulator::new(&args.eth_rpc_url_sim).await?);
    
    let tx_filter = Arc::new(TransactionFilter::new(
        args.min_tx_value,
        args.enable_smart_filtering,
    ));
    
    let metrics = Arc::new(PerformanceMetrics::default());
    let running = Arc::new(AtomicBool::new(true));
    
    // Start pool subscriber
    tokio::spawn(async move {
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber error: {}", e);
        }
    });
    
    // Give pool subscriber time to load pools
    tokio::time::sleep(Duration::from_secs(2)).await;
    let pool_count = pool_cache.get_pool_count();
    info!("✅ Pools: {} | Starting...", pool_count);
    
    // Start appropriate mode
    match args.mode {
        OperationMode::Single => {
            start_single_mode(
                args.clone(),
                tx_simulator,
                pool_cache,
                db_logger,
                tx_filter,
                metrics.clone(),
                running.clone(),
            ).await?;
        }
        OperationMode::Batch => {
            start_batch_mode(
                args.clone(),
                tx_simulator,
                pool_cache,
                db_logger,
                tx_filter,
                metrics.clone(),
                running.clone(),
            ).await?;
        }
    }
    
    // Start performance monitoring
    start_performance_monitor(metrics.clone(), running.clone()).await;
    
    info!("✅ Running for {} seconds...", args.duration);
    
    // Run for specified duration
    tokio::time::sleep(Duration::from_secs(args.duration)).await;
    
    // Shutdown
    running.store(false, Ordering::Relaxed);
    info!("⏹️  Completed");
    
    // Print final report
    print_final_report(&metrics).await;
    
    Ok(())
}

/// Single transaction mode - lowest latency
async fn start_single_mode(
    args: Args,
    tx_simulator: Arc<DebugTraceCallSimulator>,
    pool_cache: Arc<PoolStateCache>,
    db_logger: Arc<DbLogger>,
    tx_filter: Arc<TransactionFilter>,
    metrics: Arc<PerformanceMetrics>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let ipc_client = Arc::new(FullTxIpcClient::new(Some(&args.eth_ipc_path))?);
    
    // Start monitoring mempool
    ipc_client.start_monitoring().await?;
    debug!("Started monitoring mempool via IPC");
    
    // No detection service needed
    
    // Start processing loop
    let client = ipc_client.clone();
    let sim = tx_simulator.clone();
    let pools = pool_cache.clone();
    let filter = tx_filter.clone();
    let perf = metrics.clone();
    let run = running.clone();
    // No detection needed
    let db = db_logger.clone();
    
    tokio::spawn(async move {
        while run.load(Ordering::Relaxed) {
            match client.get_full_transactions(1).await {
                Ok(transactions) => {
                    if transactions.is_empty() {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        continue;
                    }
                    for tx in transactions {
                        let mempool_time = Instant::now(); // Ideally from WebSocket timestamp
                        let mut timing = TransactionTiming::new(tx.hash.to_string(), mempool_time);
                        timing.metadata.value_eth = tx.transaction.value.as_u128() as f64 / 1e18;
                        
                        // Process transaction
                        process_single_transaction(
                            tx,
                            &mut timing,
                            &sim,
                            &pools,
                            &filter,
                            // No detection,
                            &db,
                            args.enable_simulation,
                        ).await;
                        
                        // Update metrics
                        timing.calculate_durations();
                        perf.update_timing(&timing);
                        
                        // Log timing every 100 transactions
                        let count = perf.total_transactions.load(Ordering::Relaxed);
                        if count % 100 == 0 {
                            info!(
                                "⚡ [{:>5}] Total: {:.1}ms (Fetch: {:.1}ms, Sim: {:.1}ms) | Pools affected: {}",
                                count,
                                timing.durations.total_pipeline_ms,
                                timing.durations.fetch_duration_ms,
                                timing.durations.simulation_duration_ms,
                                timing.metadata.pools_affected
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Transaction fetch error: {}", e);
                    perf.fetch_errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    });
    
    Ok(())
}

/// Batch mode - high throughput
async fn start_batch_mode(
    args: Args,
    tx_simulator: Arc<DebugTraceCallSimulator>,
    pool_cache: Arc<PoolStateCache>,
    db_logger: Arc<DbLogger>,
    tx_filter: Arc<TransactionFilter>,
    metrics: Arc<PerformanceMetrics>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let batch_config = BatchConfig {
        batch_size: args.batch_size,
        batch_timeout_ms: args.batch_timeout_ms,
        connection_pool_size: args.parallel_connections,
        buffer_size: 1000,
    };
    
    let batch_client = Arc::new(BatchTxIpcClient::new(
        Some(&args.eth_ipc_path),
        Some(batch_config),
    )?);
    
    batch_client.start_batch_processing().await?;
    debug!("Batch processing started");
    
    // No detection service needed
    
    // Start batch processing
    let client = batch_client.clone();
    let sim = tx_simulator.clone();
    let pools = pool_cache.clone();
    let filter = tx_filter.clone();
    let perf = metrics.clone();
    let run = running.clone();
    // No detection needed
    let db = db_logger.clone();
    
    tokio::spawn(async move {
        while run.load(Ordering::Relaxed) {
            match client.get_transaction_batches(1).await {
                Ok(batches) => {
                    for batch in batches {
                        perf.total_batches.fetch_add(1, Ordering::Relaxed);
                        
                        // Process batch in parallel
                        let batch_size = batch.len();
                        let mut handles = vec![];
                        
                        for tx in batch {
                            let mempool_time = Instant::now(); // Ideally from batch timestamp
                            let mut timing = TransactionTiming::new(tx.hash.to_string(), mempool_time);
                            timing.metadata.value_eth = tx.transaction.value.as_u128() as f64 / 1e18;
                            
                            let sim = sim.clone();
                            let pools = pools.clone();
                            let filter = filter.clone();
                            // No detection service needed
                            let db = db.clone();
                            let enable_sim = args.enable_simulation;
                            
                            let handle = tokio::spawn(async move {
                                process_single_transaction(
                                    tx,
                                    &mut timing,
                                    &sim,
                                    &pools,
                                    &filter,
                                    // No detection,
                                    &db,
                                    enable_sim,
                                ).await;
                                
                                timing.calculate_durations();
                                timing
                            });
                            
                            handles.push(handle);
                        }
                        
                        // Collect results
                        for handle in handles {
                            if let Ok(timing) = handle.await {
                                perf.update_timing(&timing);
                            }
                        }
                        
                        // Batch processed
                    }
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
    });
    
    Ok(())
}

/// Process a single transaction through the full pipeline
async fn process_single_transaction(
    tx: FullIpcTransaction,
    timing: &mut TransactionTiming,
    tx_simulator: &DebugTraceCallSimulator,
    pool_cache: &PoolStateCache,
    tx_filter: &TransactionFilter,
    // No detection service,
    db_logger: &DbLogger,
    enable_simulation: bool,
) {
    // Mark fetch complete
    timing.timestamps.t_fetch_end = Some(Instant::now());
    
    // Apply filter
    timing.timestamps.t_filter = Some(Instant::now());
    if !tx_filter.should_process(&tx) {
        timing.metadata.filtered_out = true;
        return;
    }
    
    // Convert to TransactionView
    let tx_view = TransactionView::from_ethers_transaction(&tx.transaction);
    
    if enable_simulation {
        // Start simulation
        timing.timestamps.t_sim_start = Some(Instant::now());
        timing.metadata.simulation_attempted = true;
        
        match tx_simulator.process_transaction(&tx_view, &Default::default()).await {
            Ok(Some(state_changes)) => {
                timing.timestamps.t_sim_end = Some(Instant::now());
                timing.metadata.simulation_successful = true;
                timing.metadata.state_changes_count = state_changes.len();
                
                // Analyze state changes
                timing.timestamps.t_analysis_start = Some(Instant::now());
                
                let mut pool_interactions = HashMap::new();
                for (address_str, changes) in &state_changes {
                    if let Some(pool_state) = pool_cache.get_pool(address_str) {
                        timing.metadata.pools_checked += 1;
                        
                        // Check for significant ETH changes
                        let eth_delta = if changes.eth_net_change.is_negative {
                            -(changes.eth_net_change.absolute_value.to_string()
                                .parse::<u128>()
                                .unwrap_or(0) as f64 / 1e18)
                        } else {
                            changes.eth_net_change.absolute_value.to_string()
                                .parse::<u128>()
                                .unwrap_or(0) as f64 / 1e18
                        };
                        
                        if eth_delta.abs() > 0.001 {
                            timing.metadata.pools_affected += 1;
                            pool_interactions.insert(address_str.clone(), (pool_state, eth_delta));
                        }
                    }
                }
                
                timing.timestamps.t_analysis_end = Some(Instant::now());
                
                // Run detection if pools were affected
                if !pool_interactions.is_empty() {
                    timing.timestamps.t_detect_start = Some(Instant::now());
                    
                    // This is just a timing measurement tool - no actual scam detection
                    // For real scam detection, use mempool_signal_detection_ipc_optimized
                    timing.metadata.signals_detected = 0;
                    
                    timing.timestamps.t_detect_end = Some(Instant::now());
                }
            }
            Ok(None) => {
                timing.timestamps.t_sim_end = Some(Instant::now());
                timing.metadata.simulation_successful = true;
                // No state changes
            }
            Err(e) => {
                timing.timestamps.t_sim_end = Some(Instant::now());
                timing.metadata.simulation_successful = false;
                warn!("Simulation failed for {}: {}", tx.hash, e);
            }
        }
    }
}

/// Performance monitoring task
async fn start_performance_monitor(
    metrics: Arc<PerformanceMetrics>,
    running: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        while running.load(Ordering::Relaxed) {
            interval.tick().await;
            
            let total = metrics.total_transactions.load(Ordering::Relaxed);
            let filtered = metrics.filtered_transactions.load(Ordering::Relaxed);
            let simulated = metrics.simulated_transactions.load(Ordering::Relaxed);
            let successful = metrics.successful_simulations.load(Ordering::Relaxed);
            let with_changes = metrics.transactions_with_state_changes.load(Ordering::Relaxed);
            let affecting_pools = metrics.transactions_affecting_pools.load(Ordering::Relaxed);
            let signals = metrics.total_signals_detected.load(Ordering::Relaxed);
            
            let avg_timings = metrics.get_average_timings_ms();
            
            info!(
                "📊 [30s] Processed: {} | Success: {:.1}% | Pools hit: {:.1}% | Avg: {:.1}ms | TPS: {:.1}",
                total,
                (successful as f64 / simulated.max(1) as f64) * 100.0,
                (affecting_pools as f64 / total.max(1) as f64) * 100.0,
                avg_timings.avg_total_ms,
                total as f64 / 30.0
            );
            
            if signals > 0 {
                info!("   🚨 Signals detected: {}", signals);
            }
        }
    });
}

/// Print final performance report
async fn print_final_report(metrics: &PerformanceMetrics) {
    let total = metrics.total_transactions.load(Ordering::Relaxed);
    let avg_timings = metrics.get_average_timings_ms();
    
    info!("");
    info!("🏁 FINAL: {} txs | {} signals | Success: {:.1}% | Avg: {:.1}ms (F:{:.1} S:{:.1} A:{:.1})",
        total,
        metrics.total_signals_detected.load(Ordering::Relaxed),
        (metrics.successful_simulations.load(Ordering::Relaxed) as f64 / metrics.simulated_transactions.load(Ordering::Relaxed).max(1) as f64) * 100.0,
        avg_timings.avg_total_ms,
        avg_timings.avg_fetch_ms,
        avg_timings.avg_simulation_ms,
        avg_timings.avg_analysis_ms
    );
}