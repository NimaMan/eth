/*
 * Mempool Signal Detection Service - Full TX IPC Version
 * 
 * This version uses the best performing Full TX IPC method (1.040ms avg latency)
 * instead of WebSocket (28.3ms) for maximum performance.
 */

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error, debug, Level};
use tokio::time;
use tokio::sync::Mutex;
use chrono::Local;
use std::io::Write;
use hex;

// Mempool processor imports - using non-blocking IPC
use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, NonBlockingTransaction, TransactionView};
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{ScamDetectionService, ScamDetectionConfig, AlertPublisher};
use mempool_processor::tx_simulator::{DirectTxSimulator, CallTraceResult};

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
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Log file path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/signal_engine_full_tx_ipc.log")]
    log_file: String,
    
    /// Enable ZMQ alert publisher
    #[arg(long, env = "ENABLE_PUBLISHER")]
    enable_publisher: bool,
    
    /// ZMQ publisher endpoint
    #[arg(long, env = "ALERT_ZMQ_ADDRESS", default_value = "tcp://*:5559")]
    alert_zmq_address: String,
    
    /// Reth database directory for Direct simulator (20-40x faster than RPC)
    #[arg(long, env = "RETH_DATADIR", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_datadir: String,
}

/// Check if transaction is a liquidity removal based on function signature
fn is_liquidity_removal(input_data: &Option<Vec<u8>>) -> Option<&'static str> {
    if let Some(data) = input_data {
        if data.len() >= 4 {
            let selector = hex::encode(&data[0..4]);
            match selector.as_str() {
                "02751cec" => Some("removeLiquidityETH"),
                "baa2abde" => Some("removeLiquidity"),
                "af2979eb" => Some("removeLiquidityETHSupportingFeeOnTransferTokens"),
                "5b0d5984" => Some("removeLiquidityETHWithPermit"),
                "ded9382a" => Some("removeLiquidityETHWithPermitSupportingFeeOnTransferTokens"),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Convert NonBlockingTransaction to FullTransaction for Direct simulator
fn convert_to_full_transaction(tx: &NonBlockingTransaction) -> mempool_processor::mempool_fetcher::FullTransaction {
    mempool_processor::mempool_fetcher::FullTransaction {
        hash: tx.hash.clone(),
        tx_data: tx.data.clone(),
        detection_time: Instant::now(),
        latency_ns: tx.detection_ns,
    }
}

/// Convert non-blocking IPC transaction to TransactionView
fn convert_nonblocking_to_transaction_view(tx: &NonBlockingTransaction) -> Result<TransactionView> {
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
    max_scam: f64,
    avg_total: f64,
    max_total: f64,
    batch_pool_affected: u64,
    batch_simulation_failures: u64,
    batch_failure_rate: f64,
    queue_info: (usize, usize),
    batch_size: usize,
) {
    let report = format!(
        "[{}] ============================================================\n\
        [{}] IPC Full: {} transactions, avg latency: {}μs, queue: {}/{}\n\
        [{}] ⚡ TIMING REPORT (last {} transactions):\n\
        [{}]    📡 IPC Detection:   avg={:.2}ms  max={:.2}ms\n\
        [{}]    🔬 Simulation:      avg={:.2}ms  max={:.2}ms\n\
        [{}]    🔍 Pool Check:      avg={:.2}ms  max={:.2}ms\n\
        [{}]    🛡️  Scam Detection: avg={:.2}ms  max={:.2}ms\n\
        [{}]    📊 Total Pipeline:  avg={:.2}ms  max={:.2}ms\n\
        [{}]    🎯 Pools Affected:  {} ({:.1}%)\n\
        [{}]    ❌ Sim Failures:    {} ({:.1}%)\n\
        [{}] ============================================================\n\n",
        timestamp, timestamp, total_processed, (avg_detection * 1000.0) as u64, queue_info.0, queue_info.1,
        timestamp, batch_size,
        timestamp, avg_detection, max_detection,
        timestamp, avg_sim, max_sim,
        timestamp, avg_pool_check, max_pool_check,
        timestamp, avg_scam, max_scam,
        timestamp, avg_total, max_total,
        timestamp, batch_pool_affected, (batch_pool_affected as f64 / batch_size as f64 * 100.0),
        timestamp, batch_simulation_failures, batch_failure_rate,
        timestamp
    );
    
    // Spawn a separate task to avoid blocking the main loop
    tokio::spawn(async move {
        // Use a timeout to avoid infinite blocking, but try harder than try_lock
        match tokio::time::timeout(
            std::time::Duration::from_millis(100), 
            file.lock()
        ).await {
            Ok(mut file) => {
                let _ = file.write_all(report.as_bytes());
                let _ = file.flush();
            }
            Err(_) => {
                // File write timed out, but don't block the main processing
                warn!("Timing report file write timed out");
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
    
    // Simple logging setup like the working timing tests
    
    // Add timestamp to log file names
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_file_with_timestamp = args.log_file.replace(".log", &format!("_{}.log", timestamp));
    
    // Create separate log files for scams, market events, timing reports, and liquidity removals
    let log_dir = std::path::Path::new(&args.log_file).parent().unwrap_or(std::path::Path::new("."));
    let scam_log_file = log_dir.join(format!("scam_alerts_full_tx_{}.log", timestamp));
    let market_log_file = log_dir.join(format!("market_events_full_tx_{}.log", timestamp));
    let timing_log_file = log_dir.join(format!("timing_reports_full_tx_{}.log", timestamp));
    let liquidity_removal_log_file = log_dir.join(format!("liquidity_removals_full_tx_{}.log", timestamp));
    
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
    
    let liquidity_removal_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&liquidity_removal_log_file)?));
    
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("📝 Main timing log: {}", log_file_with_timestamp);
    info!("📝 Logging scams to: {}", scam_log_file.display());
    info!("📊 Logging market events to: {}", market_log_file.display());
    info!("💧 Logging liquidity removals to: {}", liquidity_removal_log_file.display());
    
    info!("🚀 Starting Mempool Signal Detection Service - Full TX IPC Version");
    info!("   ⚡ Using Full TX IPC for 1.040ms average latency");
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
    
    // Initialize non-blocking IPC client
    info!("🚀 Initializing non-blocking IPC client...");
    info!("   Socket path: {}", args.ipc_path);
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("⚡ Non-blocking IPC subscription active - Sub-10μs detection!");
    
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
    
    use mempool_processor::database::ScamPredictionWriter;
    
    // Try to create real ScamPredictionWriter with actual database parameters
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
                info!("✅ Alert publisher initialized successfully");
                Some(Arc::new(Mutex::new(publisher)))
            }
            Err(e) => {
                error!("❌ Failed to initialize alert publisher: {}", e);
                error!("   Continuing without alert publishing");
                None
            }
        }
    } else {
        info!("ℹ️ Alert publishing disabled");
        None
    };
    
    // Initialize Direct transaction simulator (20-40x faster than RPC)
    info!("🔧 Creating Direct transaction simulator with database: {}", &args.reth_datadir);
    let tx_simulator = match DirectTxSimulator::new(&args.reth_datadir) {
        Ok(simulator) => {
            info!("✅ Direct transaction simulator initialized successfully");
            info!("   ⚡ 20-40x faster than RPC simulation");
            simulator
        }
        Err(e) => {
            error!("❌ Failed to create Direct transaction simulator: {}", e);
            error!("   Make sure Reth database exists at: {}", &args.reth_datadir);
            return Err(e);
        }
    };
    
    // Get the file handles from earlier
    let scam_file = scam_file.clone();
    let market_file = market_file.clone();
    let timing_file = timing_file.clone();
    
    
    // Performance metrics
    info!("🎯 Setting up performance metrics...");
    let mut total_processed = 0u64;
    let mut total_events = 0u64;
    let mut events_by_type = HashMap::<String, u64>::new();
    let mut last_report = Instant::now();
    let mut total_pool_affected_count = 0u64;
    let mut sub_1ms_detections = 0u64;
    let mut simulation_failures = 0u64;
    let mut batch_simulation_failures = 0u64;
    
    // Timing vectors - only keep last 1000 transactions for reporting
    let mut detection_latencies_ms: Vec<f64> = Vec::with_capacity(1000);
    let mut simulation_times_ms: Vec<f64> = Vec::with_capacity(1000);
    let mut pool_check_times_ms: Vec<f64> = Vec::with_capacity(1000);
    let mut scam_detection_times_ms: Vec<f64> = Vec::with_capacity(1000);
    let mut total_pipeline_times_ms: Vec<f64> = Vec::with_capacity(1000);
    
    // Track transactions in current 1K batch
    let mut last_report_count = 0u64;
    let mut batch_pool_affected = 0u64;
    
    info!("✅ All initialization complete, preparing to start main loop...");
    
    // Main processing loop with ADAPTIVE BACKOFF
    // =============================================
    // Transactions arrive in BURSTS (3-34 at once) every 67-362ms
    // We fetch INSTANTLY when available, but use adaptive backoff when empty:
    // - First 1ms: Check every 100μs (catch stragglers)
    // - Next 90ms: Check every 1ms (typical inter-burst)  
    // - After 100ms: Check every 10ms (long gaps)
    info!("🔄 Starting main processing loop with Full TX IPC and adaptive backoff...");
    
    let mut consecutive_empty = 0u64;
    
    loop {
        // Log that we're in the loop
        if total_processed == 0 {
            info!("✅ Main processing loop has started successfully!");
        }
        
        
        // Get new transactions from ULTRA-FAST IPC (NO WAITING!)
        let new_txs = ipc_client.get_transactions_instant(100).await;  // Take up to 100 at once
        
        if new_txs.is_empty() {
            consecutive_empty += 1;
            
            // ADAPTIVE BACKOFF to reduce CPU usage during empty periods
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
        
        if new_txs.len() > 10 {
            info!("📦 Got {} transactions from IPC (batch processing)", new_txs.len());
        }
        
        for (tx_idx, ipc_tx) in new_txs.into_iter().enumerate() {
            
            // Log every 50th transaction to show we're receiving them
            static mut TX_COUNT: u64 = 0;
            unsafe {
                TX_COUNT += 1;
                if TX_COUNT % 50 == 0 {
                    info!("📥 Received transaction #{} from IPC", TX_COUNT);
                }
            }
            
            // Track IPC detection latency
            let detection_latency_ms = ipc_tx.detection_ns as f64 / 1_000_000.0;
            if detection_latency_ms < 1.0 {
                sub_1ms_detections += 1;
            }
            
            
            let pipeline_start = Instant::now(); // Start timing the entire pipeline
            
            // We already have the full transaction from non-blocking IPC!
            let tx_view = match convert_nonblocking_to_transaction_view(&ipc_tx) {
                Ok(view) => view,
                Err(e) => {
                    warn!("Failed to convert transaction: {}", e);
                    continue;
                }
            };
            
            // Check for liquidity removal
            if let Some(removal_type) = is_liquidity_removal(&tx_view.input_data) {
                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let log_entry = format!(
                    "[{}] 💧 {} | TX: 0x{} | From: 0x{} | To: {} | Value: {:.6} ETH | Gas: {} | IPC: {:.3}ms\n",
                    timestamp,
                    removal_type,
                    hex::encode(&tx_view.hash),
                    hex::encode(&tx_view.from),
                    tx_view.to.as_ref().map(|a| format!("0x{}", hex::encode(a))).unwrap_or_else(|| "None".to_string()),
                    tx_view.value.to_string().parse::<u128>().unwrap_or(0) as f64 / 1e18,
                    tx_view.gas_limit.unwrap_or_default(),
                    detection_latency_ms
                );
                
                // Write to liquidity removal log file
                {
                    let mut file = liquidity_removal_file.lock().await;
                    let _ = file.write_all(log_entry.as_bytes());
                    let _ = file.flush();
                }
                
                info!("💧 {} detected in tx 0x{}", removal_type, hex::encode(&tx_view.hash));
            }
            
            // Check if this is a simple ETH transfer (no data, direct to EOA)
            let is_simple_transfer = tx_view.input_data.as_ref().map_or(true, |d| d.is_empty()) 
                && tx_view.to.is_some();
            
            // Skip simulation for simple transfers but still track timing
            if is_simple_transfer {
                debug!("Skipping simulation for simple ETH transfer");
                total_processed += 1;
                
                // Add zero timing for skipped transactions
                let total_pipeline_ms = pipeline_start.elapsed().as_secs_f64() * 1000.0;
                detection_latencies_ms.push(detection_latency_ms);
                simulation_times_ms.push(0.0);
                pool_check_times_ms.push(0.0);
                scam_detection_times_ms.push(0.0);
                total_pipeline_times_ms.push(total_pipeline_ms);
                continue;
            }
            
            // Convert to FullTransaction for Direct simulator
            let full_tx = convert_to_full_transaction(&ipc_tx);
            
            // Update block number for every transaction (only takes ~6.5μs)
            if let Err(e) = tx_simulator.update_latest_block().await {
                warn!("Failed to update latest block: {}", e);
            }
            
            // Use Direct simulator with call tracer for detailed state changes
            let sim_start = Instant::now();
            match time::timeout(
                Duration::from_millis(100),
                tx_simulator.simulate_with_call_trace(&full_tx)
            ).await {
                Ok(Ok(CallTraceResult { detailed_changes, .. })) => {
                    let sim_elapsed = sim_start.elapsed().as_secs_f64() * 1000.0;
                    simulation_times_ms.push(sim_elapsed);
                    
                    // Temporarily log ALL simulations to debug (remove liquidity removal check)
                    static mut DEBUG_COUNT: u32 = 0;
                    unsafe {
                        DEBUG_COUNT += 1;
                        // Log first 5 simulations with any state changes
                        if DEBUG_COUNT <= 5 && detailed_changes.len() > 0 {
                        info!("🔬 LIQUIDITY REMOVAL SIMULATION DETAILS:");
                        info!("   Transaction: 0x{}", hex::encode(&tx_view.hash));
                        info!("   Simulation time: {:.3}ms", sim_elapsed);
                        info!("   Total addresses affected: {}", detailed_changes.len());
                        info!("   📊 ALL STATE CHANGES:");
                        
                        for (idx, (address, changes)) in detailed_changes.iter().enumerate() {
                            let address_str = mempool_processor::common::address::alloy_address_to_checksum(*address);
                            
                            // Log all addresses, not just those with significant changes
                            if changes.eth_net.abs() > 0.0 || !changes.token_net.is_empty() {
                                info!("   {}. Address: {}", idx + 1, address_str);
                                
                                if changes.eth_net.abs() > 0.0 {
                                    info!("      ETH change: {:.8} ETH", changes.eth_net);
                                }
                                
                                if !changes.token_net.is_empty() {
                                    for (token_addr, amount) in &changes.token_net {
                                        info!("      Token {} change: {:.8}", token_addr, amount);
                                    }
                                }
                                
                                // Check if this is a monitored pool
                                if let Some(pool_state) = pool_cache_clone.get_pool(&address_str) {
                                    info!("      ✅ THIS IS A MONITORED POOL!");
                                    info!("      Current ETH reserve: {:.6} ETH", pool_state.eth_reserve);
                                    info!("      Current token reserve: {:.6}", pool_state.token_reserve);
                                    let new_eth = pool_state.eth_reserve + changes.eth_net;
                                    info!("      New ETH reserve would be: {:.6} ETH", new_eth);
                                    let percent_change = if pool_state.eth_reserve > 0.0 {
                                        (changes.eth_net / pool_state.eth_reserve) * 100.0
                                    } else {
                                        0.0
                                    };
                                    info!("      Percentage change: {:.2}%", percent_change);
                                }
                            }
                        }
                        info!("   ====== END SIMULATION DETAILS ======\n");
                    } else if total_processed % 50 == 0 {
                        // Still log periodically for non-liquidity removal txs
                        info!("🔬 Simulated tx #{}: {} state changes in {:.3}ms", 
                              total_processed, detailed_changes.len(), sim_elapsed);
                    }
                    } // Close unsafe block
                    
                    let mut affected_pools = HashMap::new();
                    
                    // Process state changes for each address
                    let pool_check_start = Instant::now();
                    for (address, changes) in &detailed_changes {
                        // Convert address to checksummed string
                        let address_str = mempool_processor::common::address::alloy_address_to_checksum(*address);
                        
                        // Check if this address is a pool
                        if let Some(pool_state) = pool_cache_clone.get_pool(&address_str) {
                            // ETH delta is already in the correct format (positive/negative float)
                            let eth_delta = changes.eth_net;
                            
                            // Skip very small changes (less than 0.001 ETH)
                            if eth_delta.abs() < 0.001 {
                                continue;
                            }
                            
                            // For scam detection, we're primarily interested in ETH leaving pools
                            // But we need to track all changes for comprehensive market events
                            let current_eth = pool_state.eth_reserve;
                            let simulated_eth = current_eth + eth_delta;
                            let percentage_change = if current_eth > 0.0 {
                                eth_delta / current_eth
                            } else {
                                0.0
                            };
                            
                            // Process significant changes (> 1% or any negative change)
                            if percentage_change.abs() > 0.01 || eth_delta < 0.0 {
                                debug!("Pool {} affected: {:.6} → {:.6} ETH ({:.2}% change)", 
                                      address_str, current_eth, simulated_eth, percentage_change * 100.0);
                                
                                // For now, focus on ETH changes (token tracking can be added later)
                                let effect = mempool_processor::signal_engine::PoolEffect {
                                    pool_address: address_str.clone(),
                                    current_eth_reserve: current_eth,
                                    simulated_eth_reserve: simulated_eth,
                                    current_token_reserve: pool_state.token_reserve,
                                    simulated_token_reserve: pool_state.token_reserve, // Not tracking token changes yet
                                    eth_delta,
                                    token_delta: 0.0,
                                    percentage_change,
                                };
                                
                                affected_pools.insert(address_str.clone(), effect);
                            }
                        }
                    }
                    let pool_check_elapsed = pool_check_start.elapsed().as_secs_f64() * 1000.0;
                    pool_check_times_ms.push(pool_check_elapsed);
                    
                    let pools_affected = affected_pools.len();
                    
                    // Always measure scam detection time, even if no pools affected
                    let scam_start = Instant::now();
                    if !affected_pools.is_empty() {
                        let simulation_result = mempool_processor::signal_engine::SimulationResult {
                            tx_hash: ipc_tx.hash.clone(),
                            affected_pools,
                            simulation_successful: true,
                            error_message: None,
                        };
                        
                        match scam_service.process_transaction(simulation_result).await {
                            Ok(events) => {
                        if !events.is_empty() {
                                    total_events += events.len() as u64;
                                    for event in events {
                                        let event_type_str = format!("{:?}", event.event_type);
                                        *events_by_type.entry(event_type_str.clone()).or_insert(0) += 1;
                                        
                                        let level = match event.severity {
                                            mempool_processor::signal_engine::Severity::Critical => "🚨",
                                            mempool_processor::signal_engine::Severity::High => "⚠️",
                                            mempool_processor::signal_engine::Severity::Medium => "📊",
                                            mempool_processor::signal_engine::Severity::Low => "ℹ️",
                                        };
                                        
                                        // Use info! for better visibility in logs
                                        info!("{} {:?} DETECTED (IPC: {:.3}ms):", level, event.event_type, detection_latency_ms);
                                        info!("   Transaction Hash: {}", event.tx_hash);
                                        info!("   Pool Address: {}", event.pool_address);
                                        info!("   Token Address: {}", event.token_address);
                                        info!("   Severity: {:?}", event.severity);
                                        info!("   Confidence: {:.2}", event.confidence);
                                        info!("   ETH Impact: {:.4} ETH ({:.1}%)", event.metrics.eth_change, event.metrics.eth_percent * 100.0);
                                        info!("   Current ETH: {:.6} ETH", event.metrics.new_eth_reserve - event.metrics.eth_change);
                                        info!("   New ETH: {:.6} ETH", event.metrics.new_eth_reserve);
                                        info!("   ETH Lost: {:.6} ETH", -event.metrics.eth_change);
                                        info!("   Details: {}", event.details);
                                        
                                        // Write to appropriate separate log file
                                        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                                        let log_entry = format!(
                                            "[{}] {} {:?} | TX: {} | Pool: {} | Token: {} | ETH: {:.6} -> {:.6} ({:.2}% loss) | Lost: {:.6} ETH | IPC: {:.3}ms | {}\n",
                                            timestamp,
                                            level,
                                            event.event_type,
                                            event.tx_hash,
                                            event.pool_address,
                                            event.token_address,
                                            event.metrics.new_eth_reserve - event.metrics.eth_change,
                                            event.metrics.new_eth_reserve,
                                            event.metrics.eth_percent * 100.0,
                                            -event.metrics.eth_change,
                                            detection_latency_ms,
                                            event.details
                                        );
                                        
                                        // Write to scam log if it's a scam alert
                                        if event.event_type == mempool_processor::signal_engine::EventType::ScamAlert {
                                            let mut file = scam_file.lock().await;
                                            let _ = file.write_all(log_entry.as_bytes());
                                            let _ = file.flush();
                                        }
                                        
                                        // Always write to market events log
                                        {
                                            let mut file = market_file.lock().await;
                                            let _ = file.write_all(log_entry.as_bytes());
                                            let _ = file.flush();
                                        }
                                        
                                        // Publish alert via ZMQ if enabled
                                        if let Some(ref publisher) = alert_publisher {
                                            let event_clone = event.clone();
                                            let publisher_clone = publisher.clone();
                                            tokio::spawn(async move {
                                                let mut pub_guard = publisher_clone.lock().await;
                                                if let Err(e) = pub_guard.publish_event(&event_clone) {
                                                    error!("Failed to publish alert: {}", e);
                                                }
                                            });
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Scam detection error: {}", e);
                            }
                        }
                    }
                    let scam_elapsed = scam_start.elapsed().as_secs_f64() * 1000.0;
                    
                    total_processed += 1;
                    let total_pipeline_elapsed = Instant::now().duration_since(pipeline_start);
                    
                    
                    // Track timing for this transaction
                    let total_pipeline_ms = total_pipeline_elapsed.as_secs_f64() * 1000.0;
                    
                    // Add to batch timing vectors
                    detection_latencies_ms.push(detection_latency_ms);
                    simulation_times_ms.push(sim_elapsed);
                    pool_check_times_ms.push(pool_check_elapsed);
                    scam_detection_times_ms.push(scam_elapsed);
                    total_pipeline_times_ms.push(total_pipeline_ms);
                    
                        
                    // Log timing for every 1000th transaction to see the breakdown
                    if total_processed % 1000 == 0 {
                        info!("📊 TX {} timing: IPC:{:.3}ms → Sim:{:.3}ms → Total:{:.3}ms | {} pools", 
                              &ipc_tx.hash[..10], detection_latency_ms, sim_elapsed, total_pipeline_ms, pools_affected);
                    }
                    
                    // Track processing time for transactions that affected pools
                    if pools_affected > 0 {
                        total_pool_affected_count += 1;
                        batch_pool_affected += 1;
                        info!("🎯 POOL AFFECTED TX {} timing: IPC:{:.3}ms → Sim:{:.3}ms → Total:{:.3}ms | {} pools", 
                              &ipc_tx.hash[..10], detection_latency_ms, sim_elapsed, total_pipeline_ms, pools_affected);
                    }
                }
                Ok(Err(e)) => {
                    let sim_elapsed = sim_start.elapsed().as_secs_f64() * 1000.0;
                    simulation_times_ms.push(sim_elapsed);
                    debug!("Failed to simulate transaction {}: {}", ipc_tx.hash, e);
                    
                    // Track simulation failure
                    simulation_failures += 1;
                    batch_simulation_failures += 1;
                    
                    // Add 0ms for pool check since simulation failed
                    pool_check_times_ms.push(0.0);
                    
                    total_processed += 1;
                    
                    // Track timing even for errors
                    let total_pipeline_elapsed = Instant::now().duration_since(pipeline_start);
                    let total_pipeline_ms = total_pipeline_elapsed.as_secs_f64() * 1000.0;
                    
                    // Add timing data for failed simulation
                    detection_latencies_ms.push(detection_latency_ms);
                    pool_check_times_ms.push(0.0);
                    scam_detection_times_ms.push(0.0);
                    total_pipeline_times_ms.push(total_pipeline_ms);
                }
                Err(_) => {
                    // Timeout occurred
                    let sim_elapsed = 100.0; // Record as 100ms timeout
                    simulation_times_ms.push(sim_elapsed);
                    warn!("Simulation timeout for tx: {}", ipc_tx.hash);
                    
                    // Track simulation failure
                    simulation_failures += 1;
                    batch_simulation_failures += 1;
                    
                    // Add 0ms for pool check since simulation timed out
                    pool_check_times_ms.push(0.0);
                    
                    total_processed += 1;
                    
                    // Track timing for timeouts
                    let total_pipeline_elapsed = Instant::now().duration_since(pipeline_start);
                    let total_pipeline_ms = total_pipeline_elapsed.as_secs_f64() * 1000.0;
                    
                    // Add timing data for timeout
                    detection_latencies_ms.push(detection_latency_ms);
                    pool_check_times_ms.push(0.0);
                    scam_detection_times_ms.push(0.0);
                    total_pipeline_times_ms.push(total_pipeline_ms);
                }
            }
            
            // Report statistics when we cross each 1000 boundary
            if total_processed >= last_report_count + 1000 {
                
                // Calculate batch size
                let batch_size = detection_latencies_ms.len();
                
                let avg_detection = if !detection_latencies_ms.is_empty() {
                    detection_latencies_ms.iter().sum::<f64>() / detection_latencies_ms.len() as f64
                } else { 0.0 };
                
                let avg_sim = if !simulation_times_ms.is_empty() { 
                    simulation_times_ms.iter().sum::<f64>() / simulation_times_ms.len() as f64 
                } else { 0.0 };
                
                let avg_pool_check = if !pool_check_times_ms.is_empty() {
                    pool_check_times_ms.iter().sum::<f64>() / pool_check_times_ms.len() as f64
                } else { 0.0 };
                
                let avg_scam = if !scam_detection_times_ms.is_empty() { 
                    scam_detection_times_ms.iter().sum::<f64>() / scam_detection_times_ms.len() as f64 
                } else { 0.0 };
                
                let avg_total = if !total_pipeline_times_ms.is_empty() {
                    total_pipeline_times_ms.iter().sum::<f64>() / total_pipeline_times_ms.len() as f64
                } else { 0.0 };
                
                let sub_1ms_pct = (sub_1ms_detections as f64 / total_processed as f64) * 100.0;
                
                // Calculate max values
                let max_detection = if !detection_latencies_ms.is_empty() {
                    detection_latencies_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
                } else { 0.0 };
                
                let max_sim = if !simulation_times_ms.is_empty() {
                    simulation_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
                } else { 0.0 };
                
                let max_pool_check = if !pool_check_times_ms.is_empty() {
                    pool_check_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
                } else { 0.0 };
                
                let max_total = if !total_pipeline_times_ms.is_empty() {
                    total_pipeline_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
                } else { 0.0 };
                
                let max_scam = if !scam_detection_times_ms.is_empty() {
                    scam_detection_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
                } else { 0.0 };
                
                // Calculate failure rate
                let batch_failure_rate = (batch_simulation_failures as f64 / batch_size as f64) * 100.0;
                
                // Get queue info
                // Queue info not available in new client, use 0 for now
                let queue_info = (0, 50000);
                // Log to tracing
                info!("============================================================");
                info!("IPC Full: {} transactions, avg latency: {}μs, queue: {}/{}", 
                      total_processed, (avg_detection * 1000.0) as u64, queue_info.0, queue_info.1);
                info!("⚡ TIMING REPORT (last {} transactions):", batch_size);
                info!("   📡 IPC Detection:   avg={:.2}ms  max={:.2}ms", avg_detection, max_detection);
                info!("   🔬 Simulation:      avg={:.2}ms  max={:.2}ms", avg_sim, max_sim);
                info!("   🔍 Pool Check:      avg={:.2}ms  max={:.2}ms", avg_pool_check, max_pool_check);
                info!("   🛡️  Scam Detection: avg={:.2}ms  max={:.2}ms", avg_scam, max_scam);
                info!("   📊 Total Pipeline:  avg={:.2}ms  max={:.2}ms", avg_total, max_total);
                info!("   🎯 Pools Affected:  {} ({:.1}%)", batch_pool_affected, 
                      (batch_pool_affected as f64 / batch_size as f64 * 100.0));
                info!("   ❌ Sim Failures:    {} ({:.1}%)", batch_simulation_failures, batch_failure_rate);
                info!("============================================================");
                
                // Write to timing log file
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                // Use non-blocking write to avoid deadlock
                write_timing_report_nowait(
                    timing_file.clone(),
                    timestamp,
                    total_processed,
                    avg_detection,
                    max_detection,
                    avg_sim,
                    max_sim,
                    avg_pool_check,
                    max_pool_check,
                    avg_scam,
                    max_scam,
                    avg_total,
                    max_total,
                    batch_pool_affected,
                    batch_simulation_failures,
                    batch_failure_rate,
                    queue_info,
                    batch_size
                );
                
                // Clear buffers for next batch
                detection_latencies_ms.clear();
                simulation_times_ms.clear();
                pool_check_times_ms.clear();
                scam_detection_times_ms.clear();
                total_pipeline_times_ms.clear();
                batch_pool_affected = 0;
                batch_simulation_failures = 0;
                
                // Update last report count
                last_report_count = total_processed;
            }
        }  // End of for ipc_tx in new_txs loop
        
        // Report detailed statistics periodically
        if last_report.elapsed() > Duration::from_secs(60) {  // Every minute for better visibility
            info!("📊 DETAILED PERFORMANCE REPORT (Full TX IPC):");
            info!("   Total Transactions: {}", total_processed);
            info!("   Transactions Affecting Pools: {} ({:.1}%)", 
                  total_pool_affected_count, 
                  (total_pool_affected_count as f64 / total_processed as f64 * 100.0));
            info!("   Simulation Failures: {} ({:.1}%)", 
                  simulation_failures, 
                  (simulation_failures as f64 / total_processed as f64 * 100.0));
            
            info!("\n   ⏱️  TIMING BREAKDOWN:");
            
            // IPC detection statistics
            if !detection_latencies_ms.is_empty() {
                let avg_detection = detection_latencies_ms.iter().sum::<f64>() / detection_latencies_ms.len() as f64;
                let min_detection = detection_latencies_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_detection = detection_latencies_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let sub_1ms_pct = (sub_1ms_detections as f64 / total_processed as f64) * 100.0;
                
                info!("   1️⃣  IPC Detection (Mempool → Full TX):");
                info!("      • Average: {:.3}ms", avg_detection);
                info!("      • Min: {:.3}ms | Max: {:.3}ms", min_detection, max_detection);
                info!("      • Sub-1ms: {:.1}% ({} transactions)", sub_1ms_pct, sub_1ms_detections);
            }
            
            // Simulation statistics
            if !simulation_times_ms.is_empty() {
                let avg_sim = simulation_times_ms.iter().sum::<f64>() / simulation_times_ms.len() as f64;
                let min_sim = simulation_times_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_sim = simulation_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                info!("\n   2️⃣  Transaction Simulation:");
                info!("      • Average: {:.3}ms", avg_sim);
                info!("      • Min: {:.3}ms | Max: {:.3}ms", min_sim, max_sim);
            }
            
            // Scam detection statistics
            if !scam_detection_times_ms.is_empty() {
                let avg_scam = scam_detection_times_ms.iter().sum::<f64>() / scam_detection_times_ms.len() as f64;
                let min_scam = scam_detection_times_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_scam = scam_detection_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                info!("\n   3️⃣  Scam Detection Analysis:");
                info!("      • Average: {:.3}ms", avg_scam);
                info!("      • Min: {:.3}ms | Max: {:.3}ms", min_scam, max_scam);
            }
            
            // Total pipeline statistics
            if !total_pipeline_times_ms.is_empty() {
                let avg_total = total_pipeline_times_ms.iter().sum::<f64>() / total_pipeline_times_ms.len() as f64;
                let min_total = total_pipeline_times_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_total = total_pipeline_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                info!("\n   🎯 TOTAL PIPELINE (End-to-End):");
                info!("      • Average: {:.3}ms", avg_total);
                info!("      • Min: {:.3}ms | Max: {:.3}ms", min_total, max_total);
                info!("      • Throughput: {:.0} tx/second", 1000.0 / avg_total);
            }
            
            info!("   Market events detected: {}", total_events);
            for (event_type, count) in &events_by_type {
                info!("     - {}: {}", event_type, count);
            }
            info!("   Pools monitored: {}", pool_cache_clone.get_pool_count());
            info!("   Full TX IPC benefits: 27x faster detection than WebSocket!");
            last_report = Instant::now();
        }
    }
}