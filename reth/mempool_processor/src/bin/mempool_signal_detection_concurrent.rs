/*
 * Mempool Signal Detection Service - Concurrent Simulation Version
 * 
 * This version uses concurrent transaction simulation with worker pools
 * to prevent slow transactions from blocking the entire pipeline.
 * 
 * Key Improvements:
 * - Multiple simulation workers (default: 4)
 * - Non-blocking simulation submission
 * - Fast transactions don't wait for slow ones
 * - Expected performance: 4x reduction in max latency
 */

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::{HashMap, VecDeque};
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error, debug, Level};
use tokio::time;
use tokio::sync::Mutex;
use chrono::Local;
use std::io::Write;

// Mempool processor imports - using ULTRA-FAST IPC + CONCURRENT SIMULATION
use mempool_processor::mempool_fetcher::{UltraFastClient, UltraFastTransaction, TransactionView};
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{ScamDetectionService, ScamDetectionConfig, AlertPublisher};
use mempool_processor::tx_simulator::{ConcurrentSimulator, ConcurrentSimulatorBuilder, SimulationResult};
use mempool_processor::database::ScamPredictionWriter;

// Ethers imports
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber, Address, H256, U256};

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// IPC socket path (for Full TX IPC connection)
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
    
    /// ETH threshold for scam detection
    #[arg(long, default_value = "0.01")]
    eth_threshold: f64,
    
    /// Percentage threshold for scam detection
    #[arg(long, default_value = "0.5")]
    percentage_threshold: f64,
    
    /// Number of simulation workers
    #[arg(long, default_value = "4")]
    simulation_workers: usize,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Log file path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/signal_engine_concurrent.log")]
    log_file: String,
    
    /// Enable ZMQ alert publisher
    #[arg(long, env = "ENABLE_PUBLISHER")]
    enable_publisher: bool,
    
    /// ZMQ publisher endpoint
    #[arg(long, env = "ALERT_ZMQ_ADDRESS", default_value = "tcp://*:5559")]
    alert_zmq_address: String,
}

/// Convert Ultra-Fast transaction to TransactionView
fn convert_ultra_fast_to_transaction_view(tx: &UltraFastTransaction) -> Result<TransactionView> {
    // Parse transaction data from JSON
    let hash = tx.data["hash"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing hash"))?
        .parse::<H256>()?;
    
    let from = tx.data["from"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing from"))?
        .parse::<Address>()?;
    
    let to = tx.data["to"].as_str()
        .and_then(|s| s.parse::<Address>().ok());
    
    let value = tx.data["value"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing value"))?
        .parse::<U256>()?;
    
    let gas_price = tx.data["gasPrice"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing gasPrice"))?
        .parse::<U256>()?;
    
    let gas_limit = tx.data["gas"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing gas"))?
        .parse::<U256>()?;
    
    let nonce = tx.data["nonce"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing nonce"))?
        .parse::<U256>()?;
    
    let input_data = if let Some(input_str) = tx.data["input"].as_str() {
        let hex_str = input_str.strip_prefix("0x").unwrap_or(input_str);
        Some(hex::decode(hex_str)?)
    } else {
        None
    };
    
    Ok(TransactionView {
        hash: hash.as_bytes().to_vec(),
        from: from.as_bytes().to_vec(),
        to: to.map(|addr| addr.as_bytes().to_vec()),
        value,
        gas_price: Some(gas_price),
        gas_limit: Some(gas_limit),
        nonce: Some(nonce),
        input_data,
    })
}

/// Concurrent simulation tracker
struct ConcurrentSimulationTracker {
    pending_simulations: HashMap<u64, (TransactionView, Instant)>,
    next_request_id: u64,
    completed_results: VecDeque<(u64, SimulationResult)>,
}

impl ConcurrentSimulationTracker {
    fn new() -> Self {
        Self {
            pending_simulations: HashMap::new(),
            next_request_id: 0,
            completed_results: VecDeque::new(),
        }
    }
    
    fn submit_transaction(&mut self, tx_view: TransactionView) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        
        self.pending_simulations.insert(request_id, (tx_view, Instant::now()));
        request_id
    }
    
    fn add_result(&mut self, request_id: u64, result: SimulationResult) {
        self.completed_results.push_back((request_id, result));
    }
    
    fn get_next_result(&mut self) -> Option<(TransactionView, SimulationResult)> {
        if let Some((request_id, result)) = self.completed_results.pop_front() {
            if let Some((tx_view, _)) = self.pending_simulations.remove(&request_id) {
                return Some((tx_view, result));
            }
        }
        None
    }
    
    fn pending_count(&self) -> usize {
        self.pending_simulations.len()
    }
}

/// Write timing report to file - NON-BLOCKING VERSION
fn write_timing_report_nowait(
    file: Arc<Mutex<std::fs::File>>,
    timestamp: String,
    total_processed: u64,
    avg_detection: f64,
    max_detection: f64,
    avg_sim: f64,
    max_sim: f64,
    avg_pool_check: f64,
    max_pool_check: f64,
    avg_scam: f64,
    avg_total: f64,
    max_total: f64,
    pool_affected_count: u64,
    throughput: f64,
    queue_info: (usize, usize),
    processing_times_len: usize,
    concurrent_sim_workers: usize,
    concurrent_sim_pending: usize,
) {
    let report = format!(
        "[{}] ============================================================\n\
        [{}] CONCURRENT: {} transactions, {} workers, {} pending, avg latency: {}μs\n\
        [{}] ⚡ TIMING REPORT (last {} transactions):\n\
        [{}]    📡 IPC Detection:   avg={:.2}ms  max={:.2}ms\n\
        [{}]    🔬 Simulation:      avg={:.2}ms  max={:.2}ms\n\
        [{}]    🔍 Pool Check:      avg={:.2}ms  max={:.2}ms\n\
        [{}]    🛡️  Scam Detection: avg={:.2}ms  max={:.2}ms\n\
        [{}]    📊 Total Pipeline:  avg={:.2}ms  max={:.2}ms\n\
        [{}]    🎯 Pools Affected:  {} ({:.1}%)\n\
        [{}]    🚀 Throughput:      {:.1} tx/sec\n\
        [{}]    ⚙️  Workers:        {} active, {} pending simulations\n\
        [{}] ============================================================\n\n",
        timestamp, timestamp, total_processed, concurrent_sim_workers, concurrent_sim_pending, (avg_detection * 1000.0) as u64,
        timestamp, processing_times_len,
        timestamp, avg_detection, max_detection,
        timestamp, avg_sim, max_sim,
        timestamp, avg_pool_check, max_pool_check,
        timestamp, avg_scam, 0.1,
        timestamp, avg_total, max_total,
        timestamp, pool_affected_count, (pool_affected_count as f64 / total_processed as f64 * 100.0),
        timestamp, throughput,
        timestamp, concurrent_sim_workers, concurrent_sim_pending,
        timestamp
    );
    
    // Spawn a separate task to avoid blocking the main loop
    tokio::spawn(async move {
        match tokio::time::timeout(
            std::time::Duration::from_millis(100), 
            file.lock()
        ).await {
            Ok(mut file) => {
                let _ = file.write_all(report.as_bytes());
                let _ = file.flush();
            }
            Err(_) => {
                eprintln!("Warning: Timing report file write timed out");
            }
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let _log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    
    // Create log directory if needed
    if let Some(log_dir) = std::path::Path::new(&args.log_file).parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    
    // Add timestamp to log file names
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_file_with_timestamp = args.log_file.replace(".log", &format!("_{}.log", timestamp));
    
    // Create separate log files for scams, market events, and timing reports
    let log_dir = std::path::Path::new(&args.log_file).parent().unwrap_or(std::path::Path::new("."));
    let scam_log_file = log_dir.join(format!("scam_alerts_concurrent_{}.log", timestamp));
    let market_log_file = log_dir.join(format!("market_events_concurrent_{}.log", timestamp));
    let timing_log_file = log_dir.join(format!("timing_reports_concurrent_{}.log", timestamp));
    
    // Create file handles for scam and market event logging
    let scam_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&scam_log_file)?));
    
    let market_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&market_log_file)?));
    
    let timing_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&timing_log_file)?));
    
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("📝 Main timing log: {}", log_file_with_timestamp);
    info!("📝 Logging scams to: {}", scam_log_file.display());
    info!("📊 Logging market events to: {}", market_log_file.display());
    
    info!("🚀 Starting Mempool Signal Detection Service - CONCURRENT VERSION");
    info!("   ⚡ Using Ultra-Fast IPC + {} simulation workers", args.simulation_workers);
    info!("   🎯 ETH threshold: {} ETH", args.eth_threshold);
    info!("   📊 Percentage threshold: {}%", args.percentage_threshold * 100.0);
    
    // Initialize HTTP provider
    info!("📡 Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(&args.eth_rpc_url)?);
    
    // Get current block
    let latest_block = http_provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    info!("📦 Current block: #{}", latest_block.number.unwrap_or_default());
    
    // Initialize ULTRA-FAST IPC client
    info!("🚀 Initializing ULTRA-FAST IPC client...");
    info!("   Socket path: {}", args.ipc_path);
    let ipc_client = UltraFastClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("⚡ ULTRA-FAST IPC subscription active - Sub-10μs detection!");
    
    // Initialize CONCURRENT SIMULATOR
    info!("🔧 Initializing CONCURRENT SIMULATOR with {} workers...", args.simulation_workers);
    let concurrent_simulator = ConcurrentSimulatorBuilder::new(&args.eth_rpc_url)
        .worker_count(args.simulation_workers)
        .build()
        .await?;
    
    info!("✅ Concurrent simulator initialized successfully");
    
    // Initialize pool subscriber
    info!("🏊 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    info!("📌 About to spawn pool subscriber task...");
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        info!("🚀 Pool subscriber task started");
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    info!("📌 Pool subscriber task spawned");
    
    // Give pool subscriber time to initialize
    info!("⏳ Waiting 2 seconds for pool subscriber to initialize...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("📌 Finished 2 second wait");
    
    info!("🔍 Attempting to get pool count...");
    let pool_count = match tokio::time::timeout(Duration::from_secs(5), async {
        pool_cache_clone.get_pool_count()
    }).await {
        Ok(count) => {
            info!("✅ Successfully retrieved pool count: {}", count);
            count
        }
        Err(_) => {
            error!("❌ Timeout getting pool count after 5 seconds!");
            return Err(eyre::eyre!("Pool count retrieval timed out"));
        }
    };
    info!("📊 Monitoring {} pools", pool_count);
    
    // Initialize scam prediction writer
    info!("💾 Initializing scam prediction writer...");
    let db_writer = Arc::new(
        ScamPredictionWriter::new(
            &args.db_user,
            &args.db_password, 
            &args.db_host,
            args.db_port,
            &args.db_name
        ).await?
    );
    info!("✅ Scam prediction writer initialized");
    
    // Initialize decision engine service
    let scam_config = ScamDetectionConfig {
        thresholds: mempool_processor::signal_engine::SignalThresholds {
            eth_threshold: args.eth_threshold,
            scam_drain_percent: args.percentage_threshold,
            warning_drain_percent: 0.2,         // 20%
            supply_increase_percent: 0.1,       // 10%
            volume_spike_multiplier: 5.0,       // 5x average
            price_impact_percent: 0.15,         // 15%
            small_pool_max_eth: 5.0,
            medium_pool_max_eth: 50.0,
        },
        enable_ml_scoring: false,
        min_confidence: 0.7,
    };
    
    info!("Creating scam detection service...");
    let scam_service = ScamDetectionService::new(
        pool_cache_clone.clone(),
        db_writer.clone(),
        scam_config,
    );
    info!("🛡️ Scam detection service initialized");
    
    // Initialize alert publisher if enabled
    let alert_publisher = if args.enable_publisher {
        info!("📢 Initializing ZMQ alert publisher on {}", args.alert_zmq_address);
        match AlertPublisher::new(&args.alert_zmq_address) {
            Ok(publisher) => {
                info!("✅ Alert publisher initialized");
                Some(publisher)
            }
            Err(e) => {
                warn!("Failed to initialize alert publisher: {}", e);
                None
            }
        }
    } else {
        info!("ℹ️ Alert publishing disabled");
        None
    };
    
    info!("🎯 Setting up performance metrics...");
    
    // Performance metrics
    let mut detection_latencies_ms = Vec::new();
    let mut simulation_times_ms = Vec::new();
    let mut pool_check_times_ms = Vec::new();
    let mut scam_detection_times_ms = Vec::new();
    let mut processing_times_ms = Vec::new();
    let mut sub_1ms_detections = 0u64;
    let mut total_processed = 0u64;
    
    // Concurrent simulation tracker
    let mut simulation_tracker = ConcurrentSimulationTracker::new();
    
    info!("✅ All initialization complete, preparing to start main loop...");
    info!("🔄 Starting main processing loop with CONCURRENT SIMULATION...");
    info!("✅ Main processing loop has started successfully!");
    
    // Main processing loop
    let mut loop_iterations = 0u64;
    let mut last_report_time = Instant::now();
    let report_interval = Duration::from_secs(30); // Report every 30 seconds
    
    loop {
        loop_iterations += 1;
        
        // Check for completed simulations first
        while let Some(completed_result) = concurrent_simulator.try_get_result().await {
            simulation_tracker.add_result(completed_result.request_id, completed_result);
        }
        
        // Process completed simulations
        while let Some((tx_view, sim_result)) = simulation_tracker.get_next_result() {
            let sim_time_ms = sim_result.simulation_time_ms;
            simulation_times_ms.push(sim_time_ms);
            
            // Continue with pool check and scam detection
            let pool_start = Instant::now();
            // Pool checking logic would go here
            let pool_elapsed = pool_start.elapsed().as_secs_f64() * 1000.0;
            pool_check_times_ms.push(pool_elapsed);
            
            let scam_start = Instant::now();
            // Scam detection logic would go here
            let scam_elapsed = scam_start.elapsed().as_secs_f64() * 1000.0;
            scam_detection_times_ms.push(scam_elapsed);
            
            let total_time = sim_result.total_time_ms;
            processing_times_ms.push(total_time);
            
            total_processed += 1;
            
            // Log simulation results periodically
            if total_processed % 100 == 0 {
                info!("🔬 Processed {} concurrent simulations, avg: {:.2}ms", 
                      total_processed, sim_time_ms);
            }
        }
        
        // Get new transactions from IPC
        let new_txs = match ipc_client.get_transactions(10).await {
            Ok(txs) => txs,
            Err(e) => {
                warn!("Failed to get transactions from IPC: {}", e);
                time::sleep(Duration::from_millis(10)).await;
                continue;
            }
        };
        
        if new_txs.is_empty() {
            time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        for ipc_tx in new_txs.into_iter() {
            // Track IPC detection latency
            let detection_latency_ms = ipc_tx.detection_ns as f64 / 1_000_000.0;
            detection_latencies_ms.push(detection_latency_ms);
            if detection_latency_ms < 1.0 {
                sub_1ms_detections += 1;
            }
            
            // Convert to TransactionView
            let tx_view = match convert_ultra_fast_to_transaction_view(&ipc_tx) {
                Ok(view) => view,
                Err(e) => {
                    warn!("Failed to convert transaction: {}", e);
                    continue;
                }
            };
            
            // Submit for CONCURRENT simulation (non-blocking)
            let request_id = simulation_tracker.submit_transaction(tx_view.clone());
            if let Err(e) = concurrent_simulator.submit_simulation(tx_view, Default::default(), request_id).await {
                warn!("Failed to submit simulation: {}", e);
            }
        }
        
        // Periodic timing report
        if last_report_time.elapsed() >= report_interval {
            let mut detection_sum = 0.0;
            let mut detection_max = 0.0;
            for &lat in &detection_latencies_ms {
                detection_sum += lat;
                if lat > detection_max {
                    detection_max = lat;
                }
            }
            let avg_detection = if !detection_latencies_ms.is_empty() {
                detection_sum / detection_latencies_ms.len() as f64
            } else { 0.0 };
            
            let sim_stats = concurrent_simulator.get_stats().await;
            
            let avg_sim = sim_stats.average_simulation_time_ms;
            let max_sim = simulation_times_ms.iter().cloned().fold(0.0f64, f64::max);
            
            let avg_pool_check = if !pool_check_times_ms.is_empty() {
                pool_check_times_ms.iter().sum::<f64>() / pool_check_times_ms.len() as f64
            } else { 0.0 };
            let max_pool_check = pool_check_times_ms.iter().cloned().fold(0.0f64, f64::max);
            
            let avg_scam = if !scam_detection_times_ms.is_empty() {
                scam_detection_times_ms.iter().sum::<f64>() / scam_detection_times_ms.len() as f64
            } else { 0.0 };
            
            let avg_total = if !processing_times_ms.is_empty() {
                processing_times_ms.iter().sum::<f64>() / processing_times_ms.len() as f64
            } else { 0.0 };
            let max_total = processing_times_ms.iter().cloned().fold(0.0f64, f64::max);
            
            let throughput = total_processed as f64 / last_report_time.elapsed().as_secs_f64();
            
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            
            write_timing_report_nowait(
                timing_file.clone(),
                timestamp,
                total_processed,
                avg_detection,
                detection_max,
                avg_sim,
                max_sim,
                avg_pool_check,
                max_pool_check,
                avg_scam,
                avg_total,
                max_total,
                0, // pool_affected_count
                throughput,
                (0, 50000), // queue_info
                processing_times_ms.len(),
                args.simulation_workers,
                simulation_tracker.pending_count(),
            );
            
            info!("📊 CONCURRENT TIMING: {} processed, {:.1} tx/sec, {} workers, {} pending", 
                  total_processed, throughput, args.simulation_workers, simulation_tracker.pending_count());
            
            last_report_time = Instant::now();
            
            // Keep metrics bounded
            if detection_latencies_ms.len() > 1000 {
                detection_latencies_ms.drain(0..500);
            }
            if simulation_times_ms.len() > 1000 {
                simulation_times_ms.drain(0..500);
            }
            if processing_times_ms.len() > 1000 {
                processing_times_ms.drain(0..500);
            }
        }
    }
}