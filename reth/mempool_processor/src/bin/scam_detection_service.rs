/*
 * Scam Detection Service Binary
 * 
 * This binary integrates pool state monitoring, transaction simulation,
 * scam detection, and database logging into a complete service.
 * 
 * It accepts pool state updates from Python via ZeroMQ, monitors
 * the mempool for transactions, simulates them, detects potential scams,
 * and logs alerts to PostgreSQL.
 */

use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;
use tracing::{info, debug, error, warn, Level};
use std::collections::HashMap;
use ethers::types::H256;
use serde_json;

use mempool_processor::mempool_processor::fetcher::{MempoolFetcher, FetchMode};
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::mempool_processor::db_logger::DbLogger;
use mempool_processor::tx_simulator::{StateDiffTracker, StateCache};
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::scam_detection::{
    ScamDetectionService, 
    SimulationResult, 
    PoolEffect,
    ScamDetectionConfig
};

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// WebSocket URL for Ethereum node (for mempool subscription)
    #[arg(long, env = "ETH_WS_URL")]
    eth_ws_url: Option<String>,
    
    /// ZeroMQ socket address for pool updates
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// Database hostname
    #[arg(long, env = "DB_HOST", default_value = "localhost")]
    db_host: String,
    
    /// Database port
    #[arg(long, env = "DB_PORT", default_value = "5432")]
    db_port: u16,
    
    /// Database name
    #[arg(long, env = "DB_NAME", default_value = "eth_db")]
    db_name: String,
    
    /// Database user
    #[arg(long, env = "DB_USER", default_value = "postgres")]
    db_user: String,
    
    /// Database password
    #[arg(long, env = "DB_PASSWORD", default_value = "postgres")]
    db_password: String,
    
    /// Print statistics every N seconds
    #[arg(long, default_value = "60")]
    stats_interval_seconds: u64,
    
    /// ETH reserve threshold for scam detection (in ETH)
    #[arg(long, default_value = "0.15")]
    eth_threshold: f64,
    
    /// Percentage threshold for large withdrawals (0.0-1.0)
    #[arg(long, default_value = "0.5")]
    percentage_threshold: f64,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Log file path
    #[arg(long, default_value = "logs/scam_detection_service.log")]
    log_file: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Configure logging with a higher default level to reduce noise
    // Only show warnings and errors by default, even in verbose mode
    let log_level = if args.verbose { Level::WARN } else { Level::ERROR };
    
    // Create log directory if it doesn't exist
    if let Some(log_dir) = std::path::Path::new(&args.log_file).parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    
    // Configure logging to file
    let file_appender = tracing_appender::rolling::daily(
        std::path::Path::new(&args.log_file).parent().unwrap_or(std::path::Path::new("logs")),
        std::path::Path::new(&args.log_file).file_name().unwrap_or(std::ffi::OsStr::new("scam_detection_service.log"))
    );
    
    // Use a custom filter that shows all levels for database operations
    // but restricts other modules to the configured level
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(file_appender)
        .init();
    
    // Log startup message
    info!("Scam Detection Service Starting");
    info!("ETH RPC URL: {}", args.eth_rpc_url);
    info!("Pool ZMQ Address: {}", args.pool_zmq_address);
    info!("Database: {}:{}/{}", args.db_host, args.db_port, args.db_name);
    
    // Create the ZeroMQ subscriber for pool updates with the proper ETH threshold and ZMQ address
    info!("Initializing pool subscriber with ETH threshold: {} and ZMQ address: {}", 
         args.eth_threshold, args.pool_zmq_address);
    let pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    
    // Get the pool cache from the subscriber
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Initialize fetcher for mempool transactions
    info!("Initializing mempool transaction fetcher...");
    let fetcher = MempoolFetcher::with_options(
        &args.eth_rpc_url,
        5000, // cache size
        true, // use batch requests
        100,  // max batch size
        1000, // timeout ms
        FetchMode::RpcBatch
    )?;
    
    // Create state diff tracker for transaction simulation
    info!("Initializing state diff tracker...");
    let provider = ethers::providers::Provider::<ethers::providers::Http>::try_from(args.eth_rpc_url.clone())?;
    let provider = Arc::new(provider);
    let mut tracker = StateDiffTracker::new(provider.clone(), None);
    
    // State cache for aggregating transaction effects
    let mut state_cache = StateCache::new();
    
    // Create scam detection service
    info!("Initializing scam detection service...");
    let scam_config = ScamDetectionConfig {
        eth_threshold: args.eth_threshold,
        percentage_threshold: args.percentage_threshold,
    };
    
    // Initialize database logger separately
    info!("Connecting to database...");
    let db_logger = DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name
    ).await?;
    
    let db_logger = Arc::new(db_logger);
    
    // Create service with updated parameters
    let service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    // Start the service
    info!("Scam Detection Service has been initialized. Starting processing...");

    // Spawn the pool subscriber listener in its own task
    tokio::spawn({
        // Use the already created pool_subscriber to ensure shared cache
        let pool_subscriber_clone = pool_subscriber;
        async move {
            info!("Starting pool subscriber listener...");
            if let Err(e) = pool_subscriber_clone.start_listening().await {
                error!("Pool subscriber listener failed: {}", e);
            }
        }
    });
    
    // Main processing loop
    let mut last_stats_time = Instant::now();
    let mut total_txs_processed = 0;
    let mut total_scams_detected = 0;
    
    info!("Starting main transaction processing loop");
    
    loop {
        // Fetch new transactions from mempool
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                if !transactions.is_empty() {
                    info!("Processing {} new transactions", transactions.len());
                    
                    for tx in &transactions {
                        // Process each transaction
                        process_transaction(
                            tx, 
                            &mut tracker, 
                            &mut state_cache, 
                            &service,
                            &db_logger,
                            &pool_cache
                        ).await;
                        
                        total_txs_processed += 1;
                    }
                }
            },
            Err(e) => {
                error!("Error fetching transactions: {}", e);
                time::sleep(Duration::from_secs(1)).await;
            }
        }
        
        // Check if it's time to print stats
        let elapsed = last_stats_time.elapsed();
        if elapsed >= Duration::from_secs(args.stats_interval_seconds) {
            // Count known pools in the cache
            let mut pools_count = 0;
            
            // We can iterate through all keys in the pool cache, but that might be expensive
            // For stats, just log that we're monitoring pools without an exact count
            
            info!("Stats: Processed {} transactions, Detected {} scams, Pool monitoring active",
                 total_txs_processed, total_scams_detected);
            
            last_stats_time = Instant::now();
        }
        
        // Small delay to prevent tight loops
        time::sleep(Duration::from_millis(100)).await;
    }
}

// Process a single transaction
async fn process_transaction(
    tx: &TransactionView,
    tracker: &mut StateDiffTracker,
    state_cache: &mut StateCache,
    service: &ScamDetectionService,
    db_logger: &Arc<DbLogger>,
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) {
    let tx_hash_hex = hex::encode(&tx.hash);
    debug!("Processing transaction: {}", tx_hash_hex);
    
    // Simulate the transaction
    let mut hash_bytes = [0u8; 32];
    if tx.hash.len() == 32 {
        hash_bytes.copy_from_slice(&tx.hash);
        let tx_hash = H256::from(hash_bytes);
        
        // Simulate the transaction to get state changes
        match tracker.simulate_transaction(tx).await {
            Ok(Some(changes)) => {
                debug!("Transaction simulation successful with {} state changes", changes.len());
                
                // Add to state cache
                state_cache.add_transaction(tx_hash, tx.clone(), changes.clone());
                
                // Process for scam detection
                let simulation = prepare_simulation_result(tx, &changes, pool_cache.clone());
                
                if let Some(sim_result) = simulation {
                    // Process simulation result with the service
                    match service.process_transaction(sim_result).await {
                        Ok(alerts) => {
                            if !alerts.is_empty() {
                                // Log when an alert is detected (keeping this visible)
                                info!("Detected {} potential scams in transaction {}", 
                                    alerts.len(), tx_hash_hex);
                                
                                // Log details about detected alerts (important DB writes)
                                for (i, alert) in alerts.iter().enumerate() {
                                    info!("Alert {}: Pool {} would be depleted to {} ETH (current: {} ETH)", 
                                        i+1, 
                                        alert.pool_address, 
                                        alert.simulated_eth_reserve,
                                        alert.current_eth_reserve);
                                }
                            }
                        },
                        Err(e) => {
                            // Keep error logging visible
                            error!("Error processing transaction for scam detection: {}", e);
                        }
                    }
                }
            },
            Ok(None) => {
                debug!("Transaction {} simulation produced no state changes", tx_hash_hex);
            },
            Err(e) => {
                error!("Error simulating transaction {}: {}", tx_hash_hex, e);
            }
        }
    } else {
        warn!("Invalid transaction hash length: {}", tx.hash.len());
    }
}

// Helper function to prepare a simulation result from transaction changes
fn prepare_simulation_result(
    tx: &TransactionView,
    changes: &[mempool_processor::tx_simulator::StateChange],
    pool_cache: Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> Option<SimulationResult> {
    let mut affected_pools = HashMap::new();
    
    // Extract pool effects from state changes
    for change in changes {
        // Calculate ETH delta
        let eth_delta = (change.balance_after.as_u128() as f64 - 
                         change.balance_before.as_u128() as f64) / 1e18;
        
        // Skip addresses where ETH is being added (positive delta)
        if eth_delta >= 0.0 {
            continue;
        }
        
        // Check if this is a known pool
        let addr_str = format!("{:?}", change.address);
        if let Some(pool_state) = pool_cache.get_pool(&addr_str) {
            let current_eth = pool_state.eth_reserve;
            let simulated_eth = current_eth + eth_delta;
            let percentage_change = eth_delta / current_eth;
            
            let effect = PoolEffect {
                pool_address: addr_str.clone(),
                current_eth_reserve: current_eth,
                simulated_eth_reserve: simulated_eth,
                eth_delta,
                percentage_change,
            };
            
            affected_pools.insert(addr_str, effect);
        }
    }
    
    // If we have affected pools, create a simulation result
    if !affected_pools.is_empty() {
        let from_addr = if !tx.from.is_empty() {
            format!("{:?}", tx.from)
        } else {
            "unknown".to_string()
        };
        
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&tx.hash);
        let tx_hash = H256::from(hash_bytes);
        
        Some(SimulationResult {
            tx_hash,
            from: from_addr,
            affected_pools,
        })
    } else {
        None
    }
} 