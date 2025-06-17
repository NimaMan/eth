/*
 * Mempool Signal Detection Service
 * 
 * This service monitors the Ethereum mempool for market signals including
 * scams, liquidity changes, and other trading opportunities.
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
use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{ScamDetectionService, ScamDetectionConfig};
use mempool_processor::mempool_fetcher::processor::DbLogger;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::common::address::to_checksum_address;
use mempool_processor::performance_metrics::PerformanceTracker;

// Ethers imports
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber, Address, H256, U256};

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// WebSocket URL for real-time mempool subscription
    #[arg(long, env = "ETH_WS_URL", default_value = "ws://localhost:8546")]
    eth_ws_url: String,
    
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
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/signal_engine_service.log")]
    log_file: String,
}

/// Convert ethers Transaction to TransactionView
fn convert_ethers_to_transaction_view(tx: &ethers::types::Transaction) -> TransactionView {
    TransactionView {
        hash: tx.hash.as_bytes().to_vec(),
        from: tx.from.as_bytes().to_vec(),
        to: tx.to.map(|addr| addr.as_bytes().to_vec()),
        value: tx.value,
        gas_price: tx.gas_price,
        gas_limit: Some(tx.gas),
        nonce: Some(tx.nonce),
        input_data: Some(tx.input.to_vec()),
    }
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
    
    // Configure logging with file output (no ANSI colors)
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    use tracing_subscriber::EnvFilter;
    
    // Add timestamp to log file names
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_file_with_timestamp = args.log_file.replace(".log", &format!("_{}.log", timestamp));
    
    // Create separate log files for scams and market events
    let log_dir = std::path::Path::new(&args.log_file).parent().unwrap_or(std::path::Path::new("."));
    let scam_log_file = log_dir.join(format!("scam_alerts_{}.log", timestamp));
    let market_log_file = log_dir.join(format!("market_events_{}.log", timestamp));
    
    // Main log file appender
    let file_appender = tracing_appender::rolling::never(
        std::path::Path::new(&log_file_with_timestamp).parent().unwrap_or(std::path::Path::new(".")),
        std::path::Path::new(&log_file_with_timestamp).file_name().unwrap_or(std::ffi::OsStr::new("service.log"))
    );
    
    // Create file handles for scam and market event logging
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
    
    // Filter out noisy debug logs from hyper
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
        .with_ansi(false) // Disable ANSI colors for cleaner log files
        .with_writer(file_appender.and(std::io::stdout))
        .init();
    
    info!("🚀 Starting Mempool Signal Detection Service");
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
    
    // Initialize WebSocket client
    info!("🔌 Initializing WebSocket client...");
    let ws_client = WebSocketClient::new(&args.eth_ws_url, &args.eth_rpc_url)?;
    ws_client.start_monitoring().await?;
    info!("✅ WebSocket subscription active");
    
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
    
    let scam_service = ScamDetectionService::new(
        pool_cache_clone.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    info!("🛡️ Scam detection service initialized");
    
    // Initialize transaction simulator
    let tx_simulator = DebugTraceCallSimulator::new(&args.eth_rpc_url).await?;
    
    // Get the scam and market file handles from earlier
    let scam_file = scam_file.clone();
    let market_file = market_file.clone();
    
    // Initialize performance tracker (log every 50 transactions, keep only last 50 in memory)
    let performance_tracker = Arc::new(PerformanceTracker::new_optimized(50));
    info!("📊 Performance tracking enabled (logging every 50 transactions)");
    
    // Performance metrics
    let mut total_processed = 0u64;
    let mut total_events = 0u64;
    let mut events_by_type = HashMap::<String, u64>::new();
    let mut last_report = Instant::now();
    let mut processing_times_ms: Vec<f64> = Vec::new();
    let mut pool_affected_count = 0u64;
    
    // Main processing loop
    info!("🔄 Starting main processing loop...");
    
    loop {
        // Get new transactions from WebSocket
        let new_txs = match ws_client.get_transactions(100).await {
            Ok(txs) => txs,
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        
        if new_txs.is_empty() {
            // Small sleep to avoid busy waiting
            time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        for ws_tx in new_txs {
            // Start performance tracking for this transaction using WebSocket arrival time
            let tx_timing = performance_tracker.start_transaction_with_arrival(
                ws_tx.hash.clone(), 
                ws_tx.arrival_time
            ).await;
            
            // Mark when we start processing (after receiving from mempool)
            tx_timing.write().await.mark_processing_start();
            
            let start_time = Instant::now();
            
            // Parse transaction hash
            let tx_hash = match ws_tx.hash.parse::<H256>() {
                Ok(hash) => hash,
                Err(e) => {
                    warn!("Invalid transaction hash: {}", e);
                    // Complete tracking even on error
                    performance_tracker.complete_transaction(tx_timing).await;
                    continue;
                }
            };
            
            // Fetch full transaction
            match http_provider.get_transaction(tx_hash).await {
                Ok(Some(tx)) => {
                    // Convert ethers transaction to TransactionView for the simulator
                    let tx_view = convert_ethers_to_transaction_view(&tx);
                    
                    // Use debug_traceCall to get state changes - simulate ALL transactions
                    match tx_simulator.process_transaction(&tx_view, &Default::default()).await {
                        Ok(Some(state_changes)) => {
                            let mut affected_pools = HashMap::new();
                            
                            // Process state changes for each address
                            for (address_str, changes) in &state_changes {
                                // address_str is now already checksummed from the simulator
                                
                                // Check if this address is a pool
                                if let Some(pool_state) = pool_cache_clone.get_pool(address_str) {
                                    // Calculate ETH delta (positive or negative)
                                    let eth_delta = if changes.eth_net_change.is_negative {
                                        -(changes.eth_net_change.absolute_value.to_string()
                                            .parse::<u128>()
                                            .unwrap_or(0) as f64 / 1e18)
                                    } else {
                                        changes.eth_net_change.absolute_value.to_string()
                                            .parse::<u128>()
                                            .unwrap_or(0) as f64 / 1e18
                                    };
                                    
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
                            
                            let pools_affected = affected_pools.len();
                            
                            // If pools are affected, check for scams
                            if !affected_pools.is_empty() {
                                let simulation_result = mempool_processor::signal_engine::SimulationResult {
                                    tx_hash: format!("{:?}", tx_hash),
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
                                                info!("{} {:?} DETECTED:", level, event.event_type);
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
                                                    "[{}] {} {:?} | TX: {} | Pool: {} | Token: {} | ETH: {:.6} -> {:.6} ({:.2}% loss) | Lost: {:.6} ETH | {}\n",
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
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Scam detection error: {}", e);
                                    }
                                }
                            }
                            
                            total_processed += 1;
                            let elapsed = start_time.elapsed();
                            
                            // Complete performance tracking
                            performance_tracker.complete_transaction(tx_timing.clone()).await;
                            
                            // Track processing time
                            let processing_time_ms = elapsed.as_secs_f64() * 1000.0;
                            processing_times_ms.push(processing_time_ms);
                            
                            // Keep only last 1000 measurements to avoid memory growth
                            if processing_times_ms.len() > 1000 {
                                processing_times_ms.remove(0);
                            }
                            
                            // Report brief statistics every 1000 transactions
                            if total_processed % 1000 == 0 {
                                let avg_time = processing_times_ms.iter().sum::<f64>() / processing_times_ms.len() as f64;
                                info!("Milestone: {} transactions processed | Avg: {:.2}ms | Pools affected: {} ({:.1}%) | Events: {}", 
                                     total_processed, avg_time, pool_affected_count,
                                     (pool_affected_count as f64 / total_processed as f64 * 100.0), total_events);
                            }
                            
                            // Track processing time for transactions that affected pools
                            if pools_affected > 0 {
                                pool_affected_count += 1;
                                debug!("Processed tx {} in {:.2}ms - {} pools affected", 
                                      tx_hash, processing_time_ms, pools_affected);
                            }
                        }
                        Ok(None) => {
                            debug!("No state changes detected for transaction {}", tx_hash);
                            // Complete tracking even when no state changes
                            performance_tracker.complete_transaction(tx_timing.clone()).await;
                        }
                        Err(e) => {
                            debug!("Failed to simulate transaction {}: {}", tx_hash, e);
                            // Complete tracking even on simulation error
                            performance_tracker.complete_transaction(tx_timing.clone()).await;
                        }
                    }
                }
                Ok(None) => {
                    debug!("Transaction {} not found", tx_hash);
                    // Complete tracking even when transaction not found
                    performance_tracker.complete_transaction(tx_timing).await;
                }
                Err(e) => {
                    warn!("Failed to fetch transaction {}: {}", tx_hash, e);
                    // Complete tracking even on fetch error
                    performance_tracker.complete_transaction(tx_timing).await;
                }
            }
        }
        
        // Report detailed statistics periodically
        if last_report.elapsed() > Duration::from_secs(300) {  // Every 5 minutes
            info!("📊 Performance Statistics:");
            info!("   Transactions processed: {}", total_processed);
            info!("   Transactions affecting pools: {} ({:.1}%)", 
                  pool_affected_count, 
                  (pool_affected_count as f64 / total_processed as f64 * 100.0));
            
            // Calculate timing statistics
            if !processing_times_ms.is_empty() {
                let avg_time = processing_times_ms.iter().sum::<f64>() / processing_times_ms.len() as f64;
                let min_time = processing_times_ms.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_time = processing_times_ms.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                info!("   Processing times:");
                info!("     - Average: {:.2}ms per transaction", avg_time);
                info!("     - Min: {:.2}ms", min_time);
                info!("     - Max: {:.2}ms", max_time);
                info!("     - Transactions/second: {:.0}", 1000.0 / avg_time);
            }
            
            info!("   Market events detected: {}", total_events);
            for (event_type, count) in &events_by_type {
                info!("     - {}: {}", event_type, count);
            }
            info!("   Pools monitored: {}", pool_cache_clone.get_pool_count());
            last_report = Instant::now();
        }
    }
}


