/*
 * Optimized Mempool Signal Detection Service - IPC Version
 * 
 * This is the production-ready optimized version that:
 * 1. Uses IPC for ultra-low latency transaction access
 * 2. Enables full transaction simulation 
 * 3. Implements intelligent pre-filtering
 * 4. Provides comprehensive performance monitoring
 * 5. Maintains backward compatibility with existing logging
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

// Mempool processor imports
use mempool_processor::mempool_fetcher::ipc_socket::{FullTxIpcClient, FullIpcTransaction};
use mempool_processor::mempool_fetcher::types::TransactionView;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{ScamDetectionService, ScamDetectionConfig};
use mempool_processor::database::DbLogger;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::common::address::to_checksum_address;
use mempool_processor::performance_metrics::PerformanceTracker;

// Ethers imports for price checking
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber, Address, H256, U256};

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node (for simulator and block info)
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// IPC socket path for transaction monitoring
    #[arg(long, env = "ETH_IPC_PATH", default_value = "/tmp/reth.ipc")]
    eth_ipc_path: String,
    
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
    
    /// Enable full transaction simulation (recommended for production)
    #[arg(long)]
    enable_simulation: bool,
    
    /// Enable intelligent pre-filtering to reduce simulation load
    #[arg(long)]
    enable_smart_filtering: bool,
    
    /// Minimum transaction value to simulate (in ETH)
    #[arg(long, default_value = "0.001")]
    min_tx_value: f64,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Log file path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/signal_engine_ipc_optimized.log")]
    log_file: String,
}

/// Transaction pre-filter to reduce simulation load
struct TransactionFilter {
    min_value_wei: U256,
    enable_smart_filtering: bool,
}

impl TransactionFilter {
    fn new(min_tx_value: f64, enable_smart_filtering: bool) -> Self {
        let min_value_wei = U256::from((min_tx_value * 1e18) as u64);
        Self {
            min_value_wei,
            enable_smart_filtering,
        }
    }
    
    /// Quick check if transaction is worth simulating
    fn should_simulate(&self, tx: &FullIpcTransaction) -> bool {
        // Always simulate if smart filtering is disabled
        if !self.enable_smart_filtering {
            return true;
        }
        
        let tx_data = &tx.transaction;
        
        // Filter 1: Transaction value threshold
        if tx_data.value < self.min_value_wei {
            return false;
        }
        
        // Filter 2: Has contract interaction (has input data)
        if tx_data.input.is_empty() {
            return false;
        }
        
        // Filter 3: Reasonable gas limit (not a simple transfer)
        if tx_data.gas < U256::from(50000) {
            return false;
        }
        
        // Filter 4: Gas price suggests urgency
        if let Some(gas_price) = tx_data.gas_price {
            if gas_price < U256::from(1_000_000_000u64) { // < 1 gwei
                return false;
            }
        }
        
        true
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    
    // Create log directory if needed
    if let Some(log_dir) = std::path::Path::new(&args.log_file).parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    
    // Configure logging with file output
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    use tracing_subscriber::EnvFilter;
    
    // Add timestamp to log file names
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_file_with_timestamp = args.log_file.replace(".log", &format!("_{}.log", timestamp));
    
    // Create separate log files for different alert types
    let log_dir = std::path::Path::new(&args.log_file).parent().unwrap_or(std::path::Path::new("."));
    let scam_log_file = log_dir.join(format!("scam_alerts_ipc_optimized_{}.log", timestamp));
    let market_log_file = log_dir.join(format!("market_events_ipc_optimized_{}.log", timestamp));
    let performance_log_file = log_dir.join(format!("performance_ipc_optimized_{}.log", timestamp));
    
    // Main log file appender
    let file_appender = tracing_appender::rolling::never(
        std::path::Path::new(&log_file_with_timestamp).parent().unwrap_or(std::path::Path::new(".")),
        std::path::Path::new(&log_file_with_timestamp).file_name().unwrap_or(std::ffi::OsStr::new("service.log"))
    );
    
    // Create file handles for specialized logging
    let scam_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&scam_log_file)?));
    
    let market_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&market_log_file)?));
        
    let performance_file = Arc::new(Mutex::new(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&performance_log_file)?));
    
    info!("📝 Logging configuration:");
    info!("   🚨 Scam alerts: {}", scam_log_file.display());
    info!("   📊 Market events: {}", market_log_file.display());
    info!("   ⚡ Performance metrics: {}", performance_log_file.display());
    
    // Filter out noisy debug logs
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
    
    info!("🚀 Starting Optimized Mempool Signal Detection Service (IPC)");
    info!("============================================================");
    info!("   🎯 ETH threshold: {} ETH", args.eth_threshold);
    info!("   📊 Percentage threshold: {}%", args.percentage_threshold * 100.0);
    info!("   🔌 IPC socket: {}", args.eth_ipc_path);
    info!("   🧠 Simulation enabled: {}", args.enable_simulation);
    info!("   🔍 Smart filtering enabled: {}", args.enable_smart_filtering);
    info!("   💰 Min transaction value: {} ETH", args.min_tx_value);
    info!("");
    
    // Initialize HTTP provider (still needed for simulator and block info)
    info!("📡 Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(&args.eth_rpc_url)?);
    
    // Get current block
    let latest_block = http_provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    info!("📦 Current block: #{}", latest_block.number.unwrap_or_default());
    
    // Initialize IPC client with full transaction support
    info!("🔌 Initializing optimized IPC client...");
    let ipc_client = FullTxIpcClient::new(Some(&args.eth_ipc_path))?;
    ipc_client.start_monitoring().await?;
    info!("✅ IPC subscription active - receiving full transaction data");
    
    // Initialize pool subscriber
    info!("🏊 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        info!("Starting pool subscriber listener...");
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    
    // Give pool subscriber time to initialize
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
    
    info!("✅ Database initialization complete");
    
    // Initialize signal detection service
    info!("🔨 Creating optimized signal detection configuration...");
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
        enable_ml_scoring: false, // Can be enabled in future
        min_confidence: 0.7,
    };
    
    let scam_service = ScamDetectionService::new(
        pool_cache_clone.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    info!("🛡️ Signal detection service initialized");
    
    // Initialize transaction simulator
    let tx_simulator = if args.enable_simulation {
        info!("🔧 Initializing transaction simulator...");
        let simulator = DebugTraceCallSimulator::new(&args.eth_rpc_url).await?;
        info!("✅ Transaction simulator ready");
        Some(simulator)
    } else {
        warn!("⚠️  Transaction simulation DISABLED - only transaction monitoring");
        None
    };
    
    // Initialize transaction filter
    let tx_filter = TransactionFilter::new(args.min_tx_value, args.enable_smart_filtering);
    info!("🔍 Transaction filter initialized");
    
    // Initialize performance tracker
    let performance_tracker = Arc::new(PerformanceTracker::new_optimized(50));
    info!("📊 Performance tracking enabled (logging every 50 transactions)");
    
    // Performance metrics
    let mut total_processed = 0u64;
    let mut total_simulated = 0u64;
    let mut total_filtered_out = 0u64;
    let mut total_events = 0u64;
    let mut events_by_type = HashMap::<String, u64>::new();
    let mut last_report = Instant::now();
    let mut processing_times_ms: Vec<f64> = Vec::new();
    let mut simulation_times_ms: Vec<f64> = Vec::new();
    let mut pool_affected_count = 0u64;
    
    // Main processing loop
    info!("🔄 Starting optimized main processing loop...");
    let ipc_connected = ipc_client.is_connected().await;
    info!("📡 IPC connected: {}", ipc_connected);
    
    let mut loop_count = 0u64;
    let mut last_tx_count_log = Instant::now();
    let service_start_time = Instant::now();
    
    loop {
        loop_count += 1;
        
        // Log every 5 seconds to show we're alive
        if last_tx_count_log.elapsed() > Duration::from_secs(5) {
            info!("⏳ Loop #{}, Total processed: {}, Simulated: {}, Filtered: {}, Events: {}", 
                  loop_count, total_processed, total_simulated, total_filtered_out, total_events);
            last_tx_count_log = Instant::now();
        }
        
        // Get new transactions from IPC
        let new_txs = match ipc_client.get_full_transactions(20).await { // Optimized batch size
            Ok(txs) => txs,
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        
        if new_txs.is_empty() {
            // Small sleep to avoid busy waiting
            time::sleep(Duration::from_millis(5)).await; // Reduced sleep time
            continue;
        }
        
        debug!("📦 Received {} new transactions via IPC", new_txs.len());
        
        for ipc_tx in new_txs {
            // Start performance tracking for this transaction
            let tx_timing = performance_tracker.start_transaction_with_arrival(
                ipc_tx.hash.clone(), 
                ipc_tx.detection_time
            ).await;
            
            // Mark when we start processing
            tx_timing.write().await.mark_processing_start();
            
            let process_start_time = Instant::now();
            total_processed += 1;
            
            // Apply intelligent pre-filtering
            if !tx_filter.should_simulate(&ipc_tx) {
                total_filtered_out += 1;
                debug!("🔍 Filtered out tx {} (low value/simple transfer)", &ipc_tx.hash[..10]);
                
                // Complete tracking for filtered transactions
                performance_tracker.complete_transaction(tx_timing.clone()).await;
                continue;
            }
            
            // Transaction passed filter - proceed with simulation if enabled
            let simulation_result = if let Some(ref simulator) = tx_simulator {
                let sim_start = Instant::now();
                
                let result = simulator.process_transaction(&ipc_tx.tx_view, &Default::default()).await;
                
                let sim_elapsed = sim_start.elapsed();
                simulation_times_ms.push(sim_elapsed.as_secs_f64() * 1000.0);
                total_simulated += 1;
                
                result
            } else {
                Ok(None)
            };
            
            // Process simulation results
            match simulation_result {
                Ok(Some(state_changes)) => {
                    let mut affected_pools = HashMap::new();
                    
                    // Process state changes for each address
                    for (address_str, changes) in &state_changes {
                        // Check if this address is a pool
                        if let Some(pool_state) = pool_cache_clone.get_pool(address_str) {
                            // Calculate ETH delta
                            let eth_delta = if changes.eth_net_change.is_negative {
                                -(changes.eth_net_change.absolute_value.to_string()
                                    .parse::<u128>()
                                    .unwrap_or(0) as f64 / 1e18)
                            } else {
                                changes.eth_net_change.absolute_value.to_string()
                                    .parse::<u128>()
                                    .unwrap_or(0) as f64 / 1e18
                            };
                            
                            // Skip very small changes
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
                            
                            // Process significant changes
                            if percentage_change.abs() > 0.01 || eth_delta < 0.0 {
                                debug!("Pool {} affected: {:.6} → {:.6} ETH ({:.2}% change)", 
                                      address_str, current_eth, simulated_eth, percentage_change * 100.0);
                                
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
                    
                    let pools_affected = affected_pools.len();
                    
                    // Check for signals if pools are affected
                    if !affected_pools.is_empty() {
                        let simulation_result = mempool_processor::signal_engine::SimulationResult {
                            tx_hash: format!("{:?}", ipc_tx.transaction.hash),
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
                                        
                                        // Log event details
                                        info!("{} {:?} DETECTED (IPC Optimized):", level, event.event_type);
                                        info!("   Transaction Hash: {}", event.tx_hash);
                                        info!("   Pool Address: {}", event.pool_address);
                                        info!("   Token Address: {}", event.token_address);
                                        info!("   Severity: {:?}", event.severity);
                                        info!("   Confidence: {:.2}", event.confidence);
                                        info!("   ETH Impact: {:.4} ETH ({:.1}%)", event.metrics.eth_change, event.metrics.eth_percent * 100.0);
                                        info!("   IPC Latency: {}μs", ipc_tx.latency_us);
                                        info!("   Details: {}", event.details);
                                        
                                        // Write to specialized log files
                                        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                                        let log_entry = format!(
                                            "[{}] {} {:?} | TX: {} | Pool: {} | Token: {} | ETH: {:.6} -> {:.6} ({:.2}% loss) | Lost: {:.6} ETH | IPC_LAT: {}μs | {}\n",
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
                                            ipc_tx.latency_us,
                                            event.details
                                        );
                                        
                                        // Write to appropriate log file
                                        if event.event_type == mempool_processor::signal_engine::EventType::ScamAlert {
                                            let mut file = scam_file.lock().await;
                                            let _ = file.write_all(log_entry.as_bytes());
                                            let _ = file.flush();
                                        }
                                        
                                        {
                                            let mut file = market_file.lock().await;
                                            let _ = file.write_all(log_entry.as_bytes());
                                            let _ = file.flush();
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Signal detection error: {}", e);
                            }
                        }
                    }
                    
                    if pools_affected > 0 {
                        pool_affected_count += 1;
                    }
                }
                Ok(None) => {
                    debug!("No state changes for transaction {}", &ipc_tx.hash[..10]);
                }
                Err(e) => {
                    debug!("Failed to simulate transaction {}: {}", &ipc_tx.hash[..10], e);
                }
            }
            
            // Complete performance tracking
            let processing_time = process_start_time.elapsed();
            processing_times_ms.push(processing_time.as_secs_f64() * 1000.0);
            performance_tracker.complete_transaction(tx_timing.clone()).await;
            
            // Keep only last 1000 measurements to avoid memory growth
            if processing_times_ms.len() > 1000 {
                processing_times_ms.remove(0);
            }
            if simulation_times_ms.len() > 1000 {
                simulation_times_ms.remove(0);
            }
        }
        
        // Report detailed performance statistics periodically
        if last_report.elapsed() > Duration::from_secs(60) {  // Every minute
            let service_uptime = service_start_time.elapsed();
            let ipc_stats = ipc_client.get_stats().await;
            
            let performance_summary = format!(
                "PERFORMANCE_REPORT|{:.1}s|processed:{}/simulated:{}/filtered:{}/events:{}|ipc_lat:{}μs|proc_avg:{:.2}ms|sim_avg:{:.2}ms|pools_affected:{}|tps:{:.1}",
                service_uptime.as_secs_f64(),
                total_processed,
                total_simulated,
                total_filtered_out,
                total_events,
                ipc_stats.avg_latency_us,
                processing_times_ms.iter().sum::<f64>() / processing_times_ms.len().max(1) as f64,
                simulation_times_ms.iter().sum::<f64>() / simulation_times_ms.len().max(1) as f64,
                pool_affected_count,
                total_processed as f64 / service_uptime.as_secs_f64()
            );
            
            info!("📊 {}", performance_summary);
            
            // Write to performance log
            {
                let mut file = performance_file.lock().await;
                let _ = writeln!(file, "{}", performance_summary);
                let _ = file.flush();
            }
            
            // Detailed console output
            info!("📊 Detailed Performance Statistics:");
            info!("   ⏱️  Service uptime: {:.1}s", service_uptime.as_secs_f64());
            info!("   📦 Transactions processed: {}", total_processed);
            info!("   🔬 Transactions simulated: {} ({:.1}%)", 
                  total_simulated, 
                  (total_simulated as f64 / total_processed as f64 * 100.0));
            info!("   🔍 Transactions filtered: {} ({:.1}%)", 
                  total_filtered_out, 
                  (total_filtered_out as f64 / total_processed as f64 * 100.0));
            info!("   🎯 Market events detected: {}", total_events);
            info!("   🏊 Pools affected: {} ({:.1}%)", 
                  pool_affected_count,
                  (pool_affected_count as f64 / total_processed as f64 * 100.0));
            info!("   🚀 Processing rate: {:.1} TPS", 
                  total_processed as f64 / service_uptime.as_secs_f64());
            
            // IPC performance
            info!("   📡 IPC Performance:");
            info!("     - Average latency: {}μs", ipc_stats.avg_latency_us);
            info!("     - Sub-1ms: {:.1}%", 
                  (ipc_stats.sub_1ms_count as f64 / ipc_stats.total_transactions as f64) * 100.0);
            info!("     - Sub-10ms: {:.1}%", 
                  (ipc_stats.sub_10ms_count as f64 / ipc_stats.total_transactions as f64) * 100.0);
            
            // Processing performance
            if !processing_times_ms.is_empty() {
                let avg_processing = processing_times_ms.iter().sum::<f64>() / processing_times_ms.len() as f64;
                let min_processing = processing_times_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_processing = processing_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                info!("   ⚡ Processing times:");
                info!("     - Average: {:.2}ms", avg_processing);
                info!("     - Min: {:.2}ms", min_processing);
                info!("     - Max: {:.2}ms", max_processing);
            }
            
            // Simulation performance
            if !simulation_times_ms.is_empty() {
                let avg_simulation = simulation_times_ms.iter().sum::<f64>() / simulation_times_ms.len() as f64;
                let min_simulation = simulation_times_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_simulation = simulation_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                info!("   🔬 Simulation times:");
                info!("     - Average: {:.2}ms", avg_simulation);
                info!("     - Min: {:.2}ms", min_simulation);
                info!("     - Max: {:.2}ms", max_simulation);
            }
            
            // Event breakdown
            info!("   🎯 Event breakdown:");
            for (event_type, count) in &events_by_type {
                info!("     - {}: {}", event_type, count);
            }
            
            info!("   📊 Pool monitoring: {} pools tracked", pool_cache_clone.get_pool_count());
            info!("");
            
            last_report = Instant::now();
        }
    }
}