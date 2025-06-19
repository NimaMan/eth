/*
 * Mempool Signal Detection Service - Parallel Processing Version
 * 
 * This version uses multiple worker threads to process transactions in parallel.
 * Architecture: IPC Client → Main Queue → Distributor → Worker Queues → Workers
 */

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error, debug};
use tokio::time;
use tokio::sync::{mpsc, Mutex, RwLock};
use chrono::Local;
use std::io::Write;

// Mempool processor imports
use mempool_processor::mempool_fetcher::ipc_ipc_variants::{FullTxIpcClient, FullIpcTransaction};
use mempool_processor::mempool_fetcher::TransactionView;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{ScamDetectionService, ScamDetectionConfig};
use mempool_processor::mempool_fetcher::processor::DbLogger;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::performance_metrics::PerformanceTracker;

// Ethers imports
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber};

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// ZeroMQ socket address for pool updates
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// Database connection parameters
    #[arg(long, env = "DB_HOST", default_value = "localhost")]
    db_host: String,
    
    #[arg(long, env = "DB_PORT", default_value = "5432")]
    db_port: u16,
    
    #[arg(long, env = "DB_NAME", default_value = "eth_db")]
    db_name: String,
    
    #[arg(long, env = "DB_USER", default_value = "postgres")]
    db_user: String,
    
    #[arg(long, env = "DB_PASSWORD", default_value = "postgres")]
    db_password: String,
    
    /// Number of parallel workers
    #[arg(long, default_value = "2")]
    workers: usize,
    
    /// Worker queue size
    #[arg(long, default_value = "1000")]
    worker_queue_size: usize,
    
    /// ETH threshold for scam detection
    #[arg(long, default_value = "0.01")]
    eth_threshold: f64,
    
    /// Percentage threshold for scam detection
    #[arg(long, default_value = "0.5")]
    percentage_threshold: f64,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Log file path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/signal_engine_parallel.log")]
    log_file: String,
}

/// Shared metrics for all workers
#[derive(Debug, Default)]
struct SharedMetrics {
    total_processed: AtomicU64,
    total_events: AtomicU64,
    pool_affected_count: AtomicU64,
    sub_1ms_detections: AtomicU64,
    
    // Queue monitoring
    main_queue_depth: AtomicU64,
    worker_queue_depths: Vec<AtomicU64>,
    
    // Worker health
    worker_active: Vec<AtomicBool>,
}

impl SharedMetrics {
    fn new(num_workers: usize) -> Self {
        let mut worker_queue_depths = Vec::with_capacity(num_workers);
        let mut worker_active = Vec::with_capacity(num_workers);
        
        for _ in 0..num_workers {
            worker_queue_depths.push(AtomicU64::new(0));
            worker_active.push(AtomicBool::new(true));
        }
        
        Self {
            worker_queue_depths,
            worker_active,
            ..Default::default()
        }
    }
}

/// Worker task that processes transactions
async fn worker_task(
    worker_id: usize,
    mut rx: mpsc::Receiver<FullIpcTransaction>,
    pool_cache: Arc<mempool_processor::pool_subscriber::PoolStateCache>,
    db_logger: Arc<DbLogger>,
    scam_config: ScamDetectionConfig,
    metrics: Arc<SharedMetrics>,
    scam_file: Arc<Mutex<std::fs::File>>,
    market_file: Arc<Mutex<std::fs::File>>,
    performance_tracker: Arc<PerformanceTracker>,
) -> Result<()> {
    info!("🔧 Worker {} starting", worker_id);
    
    // Create per-worker transaction simulator
    let tx_simulator = DebugTraceCallSimulator::new("http://localhost:8545").await?;
    
    // Create scam service once (it's not Send due to ZMQ)
    let scam_service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    // Per-worker statistics
    let mut worker_processed = 0u64;
    let mut processing_times = Vec::new();
    let mut simulation_times = Vec::new();
    
    // Process transactions
    while let Some(ipc_tx) = rx.recv().await {
        // Update queue depth
        let queue_len = rx.len();
        metrics.worker_queue_depths[worker_id].store(queue_len as u64, Ordering::Relaxed);
                let start_time = Instant::now();
                let detection_latency_ms = ipc_tx.latency_us as f64 / 1000.0;
                
                if detection_latency_ms < 1.0 {
                    metrics.sub_1ms_detections.fetch_add(1, Ordering::Relaxed);
                }
                
                // Start performance tracking
                let tx_timing = performance_tracker.start_transaction_with_arrival(
                    ipc_tx.hash.clone(), 
                    ipc_tx.detection_time
                ).await;
                tx_timing.write().await.mark_processing_start();
                
                // Convert transaction
                let tx = &ipc_tx.transaction;
                let tx_view = TransactionView {
                    hash: tx.hash.as_bytes().to_vec(),
                    from: tx.from.as_bytes().to_vec(),
                    to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                    value: tx.value,
                    gas_price: tx.gas_price,
                    gas_limit: Some(tx.gas),
                    nonce: Some(tx.nonce),
                    input_data: Some(tx.input.to_vec()),
                };
                
                // Simulate transaction
                let sim_start = Instant::now();
                match tx_simulator.process_transaction(&tx_view, &Default::default()).await {
                    Ok(Some(state_changes)) => {
                        let sim_elapsed = sim_start.elapsed().as_secs_f64() * 1000.0;
                        simulation_times.push(sim_elapsed);
                        
                        let mut affected_pools = HashMap::new();
                        
                        // Process state changes
                        for (address_str, changes) in &state_changes {
                            if let Some(pool_state) = pool_cache.get_pool(address_str) {
                                let eth_delta = if changes.eth_net_change.is_negative {
                                    -(changes.eth_net_change.absolute_value.to_string()
                                        .parse::<u128>()
                                        .unwrap_or(0) as f64 / 1e18)
                                } else {
                                    changes.eth_net_change.absolute_value.to_string()
                                        .parse::<u128>()
                                        .unwrap_or(0) as f64 / 1e18
                                };
                                
                                if eth_delta.abs() < 0.001 {
                                    continue;
                                }
                                
                                let current_eth = pool_state.eth_reserve;
                                let simulated_eth = current_eth + eth_delta;
                                let percentage_change = if current_eth > 0.0 {
                                    eth_delta / current_eth
                                } else {
                                    0.0
                                };
                                
                                if percentage_change.abs() > 0.01 || eth_delta < 0.0 {
                                    let effect = mempool_processor::signal_engine::PoolEffect {
                                        pool_address: address_str.clone(),
                                        current_eth_reserve: current_eth,
                                        simulated_eth_reserve: simulated_eth,
                                        current_token_reserve: pool_state.token_reserve,
                                        simulated_token_reserve: pool_state.token_reserve,
                                        eth_delta,
                                        token_delta: 0.0,
                                        percentage_change,
                                    };
                                    
                                    affected_pools.insert(address_str.clone(), effect);
                                }
                            }
                        }
                        
                        // Check for scams if pools affected
                        if !affected_pools.is_empty() {
                            metrics.pool_affected_count.fetch_add(1, Ordering::Relaxed);
                            
                            let simulation_result = mempool_processor::signal_engine::SimulationResult {
                                tx_hash: format!("{:?}", tx.hash),
                                affected_pools,
                                simulation_successful: true,
                                error_message: None,
                            };
                            
                            
                            // Use block_in_place for the non-Send scam service
                            let process_result = tokio::task::block_in_place(|| {
                                // Create a new runtime for the blocking task
                                let rt = tokio::runtime::Handle::current();
                                rt.block_on(async {
                                    scam_service.process_transaction(simulation_result).await
                                        .map_err(|e| format!("Scam detection error: {}", e))
                                })
                            });
                            
                            match process_result {
                                Ok(events) => {
                                    if !events.is_empty() {
                                        metrics.total_events.fetch_add(events.len() as u64, Ordering::Relaxed);
                                        
                                        for event in events {
                                            let level = match event.severity {
                                                mempool_processor::signal_engine::Severity::Critical => "🚨",
                                                mempool_processor::signal_engine::Severity::High => "⚠️",
                                                mempool_processor::signal_engine::Severity::Medium => "📊",
                                                mempool_processor::signal_engine::Severity::Low => "ℹ️",
                                            };
                                            
                                            info!("[W{}] {} {:?} TX: {} | ETH: {:.6} → {:.6} ({:.2}%)", 
                                                  worker_id, level, event.event_type, 
                                                  &event.tx_hash[..10], 
                                                  event.metrics.new_eth_reserve - event.metrics.eth_change,
                                                  event.metrics.new_eth_reserve,
                                                  event.metrics.eth_percent * 100.0);
                                            
                                            // Log to files
                                            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                                            let log_entry = format!(
                                                "[{}] [W{}] {} {:?} | TX: {} | Pool: {} | ETH: {:.6} → {:.6} ({:.2}%) | {}\n",
                                                timestamp, worker_id, level, event.event_type,
                                                event.tx_hash, event.pool_address,
                                                event.metrics.new_eth_reserve - event.metrics.eth_change,
                                                event.metrics.new_eth_reserve,
                                                event.metrics.eth_percent * 100.0,
                                                event.details
                                            );
                                            
                                            if event.event_type == mempool_processor::signal_engine::EventType::ScamAlert {
                                                let mut file = scam_file.lock().await;
                                                let _ = file.write_all(log_entry.as_bytes());
                                                let _ = file.flush();
                                            }
                                            
                                            let mut file = market_file.lock().await;
                                            let _ = file.write_all(log_entry.as_bytes());
                                            let _ = file.flush();
                                        }
                                    }
                                }
                                Err(err_msg) => {
                                    error!("[W{}] {}", worker_id, err_msg);
                                }
                            }
                        }
                        
                        // Track performance
                        performance_tracker.complete_transaction(tx_timing.clone()).await;
                        
                        let processing_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                        processing_times.push(processing_time_ms);
                        
                        worker_processed += 1;
                        metrics.total_processed.fetch_add(1, Ordering::Relaxed);
                        
                        // Log progress every 100 transactions
                        if worker_processed % 100 == 0 {
                            let avg_proc = processing_times.iter().sum::<f64>() / processing_times.len() as f64;
                            let avg_sim = simulation_times.iter().sum::<f64>() / simulation_times.len() as f64;
                            
                            info!("[W{}] Processed {} txs | Avg times - Process: {:.2}ms, Sim: {:.2}ms | Queue: {}", 
                                  worker_id, worker_processed, avg_proc, avg_sim, queue_len);
                            
                            // Keep only last 1000 measurements
                            if processing_times.len() > 1000 {
                                processing_times.drain(0..100);
                                simulation_times.drain(0..100);
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("[W{}] No state changes for tx {}", worker_id, tx.hash);
                        performance_tracker.complete_transaction(tx_timing).await;
                        worker_processed += 1;
                        metrics.total_processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        debug!("[W{}] Failed to simulate tx {}: {}", worker_id, tx.hash, e);
                        performance_tracker.complete_transaction(tx_timing).await;
                        worker_processed += 1;
                        metrics.total_processed.fetch_add(1, Ordering::Relaxed);
                    }
                }
    }
    
    metrics.worker_active[worker_id].store(false, Ordering::Relaxed);
    info!("[W{}] Worker stopped after processing {} transactions", worker_id, worker_processed);
    Ok(())
}

/// Distributor task that pulls from main queue and distributes to workers
async fn distributor_task(
    ipc_client: Arc<FullTxIpcClient>,
    worker_senders: Vec<mpsc::Sender<FullIpcTransaction>>,
    metrics: Arc<SharedMetrics>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    info!("📦 Distributor starting with {} workers", worker_senders.len());
    
    let mut current_worker = 0;
    let num_workers = worker_senders.len();
    let mut distributed_count = 0u64;
    
    loop {
        if shutdown.load(Ordering::Relaxed) {
            info!("Distributor shutting down");
            break;
        }
        
        // Get batch of transactions
        match ipc_client.get_full_transactions(50).await {
            Ok(txs) => {
                if txs.is_empty() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
                
                // Distribute transactions round-robin
                for tx in txs {
                    // Find next available worker
                    let mut attempts = 0;
                    let mut tx_to_send = Some(tx);
                    
                    while attempts < num_workers && tx_to_send.is_some() {
                        if worker_senders[current_worker].capacity() > 0 {
                            match worker_senders[current_worker].try_send(tx_to_send.take().unwrap()) {
                                Ok(_) => {
                                    distributed_count += 1;
                                    current_worker = (current_worker + 1) % num_workers;
                                    break;
                                }
                                Err(mpsc::error::TrySendError::Full(returned_tx)) => {
                                    // Try next worker
                                    tx_to_send = Some(returned_tx);
                                    current_worker = (current_worker + 1) % num_workers;
                                    attempts += 1;
                                }
                                Err(mpsc::error::TrySendError::Closed(returned_tx)) => {
                                    error!("Worker {} channel closed", current_worker);
                                    tx_to_send = Some(returned_tx);
                                    current_worker = (current_worker + 1) % num_workers;
                                    attempts += 1;
                                }
                            }
                        } else {
                            current_worker = (current_worker + 1) % num_workers;
                            attempts += 1;
                        }
                    }
                    
                    if attempts == num_workers {
                        warn!("All worker queues full, dropping transaction");
                    }
                }
                
                if distributed_count % 1000 == 0 {
                    info!("📦 Distributed {} transactions", distributed_count);
                }
            }
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    
    info!("📦 Distributor stopped after distributing {} transactions", distributed_count);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Validate worker count
    let num_cpus = num_cpus::get();
    if args.workers > num_cpus {
        warn!("Worker count {} exceeds CPU count {}, performance may degrade", args.workers, num_cpus);
    }
    
    // Create log directory if needed
    if let Some(log_dir) = std::path::Path::new(&args.log_file).parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    
    // Configure logging
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    use tracing_subscriber::EnvFilter;
    
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_file_with_timestamp = args.log_file.replace(".log", &format!("_{}.log", timestamp));
    
    let log_dir = std::path::Path::new(&args.log_file).parent().unwrap_or(std::path::Path::new("."));
    let scam_log_file = log_dir.join(format!("scam_alerts_parallel_{}.log", timestamp));
    let market_log_file = log_dir.join(format!("market_events_parallel_{}.log", timestamp));
    
    let file_appender = tracing_appender::rolling::never(
        std::path::Path::new(&log_file_with_timestamp).parent().unwrap_or(std::path::Path::new(".")),
        std::path::Path::new(&log_file_with_timestamp).file_name().unwrap_or(std::ffi::OsStr::new("service.log"))
    );
    
    let scam_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&scam_log_file)?));
    
    let market_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&market_log_file)?));
    
    info!("📝 Logging scams to: {}", scam_log_file.display());
    info!("📊 Logging market events to: {}", market_log_file.display());
    
    let filter = EnvFilter::new(
        format!("mempool_processor={},hyper=warn,reqwest=warn", 
                if args.verbose { "debug" } else { "info" })
    );
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_writer(file_appender.and(std::io::stdout))
        .init();
    
    info!("🚀 Starting Parallel Mempool Signal Detection Service");
    info!("   ⚡ Using {} workers", args.workers);
    info!("   📦 Worker queue size: {}", args.worker_queue_size);
    info!("   🎯 ETH threshold: {} ETH", args.eth_threshold);
    info!("   📊 Percentage threshold: {}%", args.percentage_threshold * 100.0);
    
    // Initialize HTTP provider
    let http_provider = Arc::new(Provider::<Http>::try_from(&args.eth_rpc_url)?);
    let latest_block = http_provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    info!("📦 Current block: #{}", latest_block.number.unwrap_or_default());
    
    // Initialize IPC client
    info!("🔌 Initializing Full TX IPC client...");
    let ipc_client = Arc::new(FullTxIpcClient::new(Some(&args.ipc_path))?);
    ipc_client.start_monitoring().await?;
    info!("✅ Full TX IPC subscription active");
    
    // Initialize pool subscriber
    info!("🏊 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        info!("Starting pool subscriber listener...");
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    let pool_count = pool_cache_clone.get_pool_count();
    info!("📊 Monitoring {} pools", pool_count);
    
    // Initialize database logger
    info!("💾 Connecting to database...");
    let db_logger = Arc::new(DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name
    ).await?);
    
    // Initialize scam detection service
    let scam_config = ScamDetectionConfig {
        thresholds: mempool_processor::signal_engine::SignalThresholds {
            eth_threshold: args.eth_threshold,
            scam_drain_percent: args.percentage_threshold,
            warning_drain_percent: 0.2,
            supply_increase_percent: 0.1,
            volume_spike_multiplier: 5.0,
            price_impact_percent: 0.15,
            small_pool_max_eth: 5.0,
            medium_pool_max_eth: 50.0,
        },
        enable_ml_scoring: false,
        min_confidence: 0.7,
    };
    
    // Note: ScamDetectionService will be created per-worker due to ZMQ socket
    info!("🛡️ Scam detection config prepared (service created per worker)");
    
    // Initialize performance tracker
    let performance_tracker = Arc::new(PerformanceTracker::new_optimized(50));
    
    // Initialize shared metrics
    let metrics = Arc::new(SharedMetrics::new(args.workers));
    
    // Create worker channels and spawn workers
    let mut worker_senders = Vec::new();
    let mut worker_handles = Vec::new();
    
    for worker_id in 0..args.workers {
        let (tx, rx) = mpsc::channel(args.worker_queue_size);
        worker_senders.push(tx);
        
        let pool_cache = pool_cache_clone.clone();
        let db_logger = db_logger.clone();
        let scam_config_clone = scam_config.clone();
        let metrics = metrics.clone();
        let scam_file = scam_file.clone();
        let market_file = market_file.clone();
        let performance_tracker = performance_tracker.clone();
        
        // Use spawn_local to avoid Send requirement or handle the error differently
        let handle = tokio::task::spawn(async move {
            match worker_task(
                worker_id,
                rx,
                pool_cache,
                db_logger,
                scam_config_clone,
                metrics,
                scam_file,
                market_file,
                performance_tracker,
            ).await {
                Ok(_) => {},
                Err(e) => {
                    error!("Worker {} failed: {}", worker_id, e);
                }
            }
        });
        
        worker_handles.push(handle);
    }
    
    // Start distributor
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    let metrics_clone = metrics.clone();
    let ipc_client_clone = ipc_client.clone();
    
    let distributor_handle = tokio::spawn(async move {
        if let Err(e) = distributor_task(
            ipc_client_clone,
            worker_senders,
            metrics_clone,
            shutdown_clone,
        ).await {
            error!("Distributor failed: {}", e);
        }
    });
    
    // Metrics reporting task
    let metrics_clone = metrics.clone();
    let pool_cache_clone = pool_cache_clone.clone();
    tokio::spawn(async move {
        let mut last_total = 0u64;
        let start_time = Instant::now();
        
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            
            let total = metrics_clone.total_processed.load(Ordering::Relaxed);
            let events = metrics_clone.total_events.load(Ordering::Relaxed);
            let pools_affected = metrics_clone.pool_affected_count.load(Ordering::Relaxed);
            let sub_1ms = metrics_clone.sub_1ms_detections.load(Ordering::Relaxed);
            
            let rate = (total - last_total) as f64 / 60.0;
            let elapsed = start_time.elapsed().as_secs_f64();
            let overall_rate = total as f64 / elapsed;
            
            info!("📊 METRICS REPORT:");
            info!("   Total processed: {} transactions", total);
            info!("   Processing rate: {:.1} tx/sec (last minute), {:.1} tx/sec (overall)", rate, overall_rate);
            info!("   Pools affected: {} ({:.1}%)", pools_affected, (pools_affected as f64 / total as f64 * 100.0));
            info!("   Market events: {}", events);
            info!("   Sub-1ms detections: {} ({:.1}%)", sub_1ms, (sub_1ms as f64 / total as f64 * 100.0));
            
            // Worker queue depths
            info!("   Worker queues:");
            for (i, depth) in metrics_clone.worker_queue_depths.iter().enumerate() {
                let d = depth.load(Ordering::Relaxed);
                let active = metrics_clone.worker_active[i].load(Ordering::Relaxed);
                info!("     Worker {}: {} items ({})", i, d, if active { "active" } else { "inactive" });
            }
            
            info!("   Pools monitored: {}", pool_cache_clone.get_pool_count());
            
            last_total = total;
        }
    });
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping...");
    
    shutdown.store(true, Ordering::Relaxed);
    
    // Wait for distributor to stop
    let _ = distributor_handle.await;
    
    // Wait for workers to finish (channels already closed by distributor)
    for handle in worker_handles {
        let _ = handle.await;
    }
    
    // Final metrics
    let total = metrics.total_processed.load(Ordering::Relaxed);
    let events = metrics.total_events.load(Ordering::Relaxed);
    let pools_affected = metrics.pool_affected_count.load(Ordering::Relaxed);
    
    info!("📊 FINAL STATISTICS:");
    info!("   Total processed: {} transactions", total);
    info!("   Pools affected: {} ({:.1}%)", pools_affected, (pools_affected as f64 / total as f64 * 100.0));
    info!("   Market events detected: {}", events);
    
    Ok(())
}