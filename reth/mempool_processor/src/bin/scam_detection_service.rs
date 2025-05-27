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
use tokio::time;
use tracing::{info, error, Level};
use std::collections::HashMap;
use ethers::types::H256;
use revm_primitives::alloy_primitives::{Address, keccak256};

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
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/scam_detection_service.log")]
    log_file: String,
}

/// Ethereum address checksum utility
/// Converts an address to EIP-55 checksummed format to match Python's Web3.to_checksum_address()
fn to_checksum_address(address: Address) -> String {
    let addr_hex = hex::encode(address.as_slice());
    let hash = keccak256(addr_hex.as_bytes());
    let hash_hex = hex::encode(hash.as_slice());
    
    let mut result = String::with_capacity(42);
    result.push_str("0x");
    
    for (i, c) in addr_hex.chars().enumerate() {
        if c.is_ascii_digit() {
            result.push(c);
        } else {
            // Check if the corresponding hash character is >= 8
            let hash_char = hash_hex.chars().nth(i).unwrap_or('0');
            if hash_char >= '8' {
                result.push(c.to_ascii_uppercase());
            } else {
                result.push(c.to_ascii_lowercase());
            }
        }
    }
    
    result
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Configure logging - use INFO level to see scam detection logs
    let log_level = if args.verbose { Level::DEBUG } else { Level::WARN };
    
    // Create log directory if it doesn't exist
    if let Some(log_dir) = std::path::Path::new(&args.log_file).parent() {
        std::fs::create_dir_all(log_dir)?;
        println!("Created log directory: {:?}", log_dir);
    }
    
    // Configure logging to file
    let file_appender = tracing_appender::rolling::daily(
        std::path::Path::new(&args.log_file).parent().unwrap_or(std::path::Path::new("logs")),
        std::path::Path::new(&args.log_file).file_name().unwrap_or(std::ffi::OsStr::new("scam_detection_service.log"))
    );
    
    // Use a custom filter that shows important logs but suppresses noisy external crates
    use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};
    
    let filter = if args.verbose {
        // Verbose mode: show everything
        EnvFilter::from_default_env()
            .add_directive("mempool_processor=debug".parse()?)
            .add_directive("scam_detection_service=debug".parse()?)
    } else {
        // Production mode: only show important logs
        EnvFilter::from_default_env()
            .add_directive("mempool_processor=info".parse()?)
            .add_directive("scam_detection_service=info".parse()?)
            .add_directive("hyper=warn".parse()?)
            .add_directive("tokio_postgres=warn".parse()?)
            .add_directive("h2=warn".parse()?)
            .add_directive("tower=warn".parse()?)
            .add_directive("reqwest=warn".parse()?)
    };
    
    // Custom timer that uses local time to match Python logs
    struct LocalTimer;
    
    impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
        fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
            let now = chrono::Local::now();
            write!(w, "{}", now.format("%Y-%m-%d %H:%M:%S"))
        }
    }
    
    tracing_subscriber::registry()
        .with(fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false)  // Disable ANSI color codes in log files
            .with_target(false)  // Don't show the target module in logs for cleaner output
            .with_timer(LocalTimer)  // Use local time to match Python logs
        )
        .with(filter)
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
    
    // Initialize fetcher for mempool transactions optimized for real-time scam detection
    info!("Initializing mempool transaction fetcher...");
    let fetcher = MempoolFetcher::with_options(
        &args.eth_rpc_url,
        5000, // cache size
        true, // use batch requests
        50,   // max batch size - reduced for faster response
        1000, // timeout ms - optimized for 1-second real-time detection
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
                    for tx in &transactions {
                        // Process each transaction
                        let scams_found = process_transaction(
                            tx, 
                            &mut tracker, 
                            &mut state_cache, 
                            &service,
                            &db_logger,
                            &pool_cache
                        ).await;
                        
                        total_txs_processed += 1;
                        total_scams_detected += scams_found;
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
            // Get performance metrics from fetcher
            let (success_rate, current_timeout, batch_size) = fetcher.get_performance_metrics();
            
            info!("Stats: {} transactions processed, {} scams detected", 
                 total_txs_processed, total_scams_detected);
            info!("Fetcher performance: {:.2}% success rate, {}ms timeout, {} batch size", 
                 success_rate * 100.0, current_timeout, batch_size);
            last_stats_time = Instant::now();
        }
        
        // Small delay to prevent tight loops (reduced for real-time scam detection)
        time::sleep(Duration::from_millis(50)).await;
    }
}

// Process a single transaction and return number of scams detected
async fn process_transaction(
    tx: &TransactionView,
    tracker: &mut StateDiffTracker,
    state_cache: &mut StateCache,
    service: &ScamDetectionService,
    db_logger: &Arc<DbLogger>,
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> usize {
    let tx_hash_hex = hex::encode(&tx.hash);
    
    // Simulate the transaction
    let mut hash_bytes = [0u8; 32];
    if tx.hash.len() == 32 {
        hash_bytes.copy_from_slice(&tx.hash);
        let tx_hash = H256::from(hash_bytes);
        
        // Simulate the transaction to get state changes
        match tracker.simulate_transaction(tx).await {
            Ok(Some(changes)) => {
                // Add to state cache
                state_cache.add_transaction(tx_hash, tx.clone(), changes.clone());
                
                // Process for scam detection
                let simulation = prepare_simulation_result(tx, &changes, pool_cache.clone());
                
                if let Some(sim_result) = simulation {
                    // Process simulation result with the service
                    match service.process_transaction(sim_result).await {
                        Ok(alerts) => {
                            if !alerts.is_empty() {
                                // Only log urgent scam alerts
                                info!("🚨 SCAM DETECTED: {} alerts in tx {}", alerts.len(), tx_hash_hex);
                                
                                for alert in &alerts {
                                    info!("  Pool {} depleted: {} → {} ETH", 
                                        alert.pool_address, 
                                        alert.current_eth_reserve,
                                        alert.simulated_eth_reserve);
                                }
                                return alerts.len();
                            }
                        },
                        Err(e) => {
                            error!("Scam detection error: {}", e);
                        }
                    }
                }
            },
            Ok(None) => {
                // No state changes - silent
            },
            Err(_) => {
                // Simulation failed - silent (these are common)
            }
        }
    }
    
    0 // No scams detected
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
        
        // Format address properly - use checksummed format to match Python
        // Convert H160 to Address for checksum function
        let address_bytes = change.address.as_bytes();
        let alloy_address = Address::from_slice(address_bytes);
        let addr_str = to_checksum_address(alloy_address);
        
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
            format!("0x{}", hex::encode(&tx.from))
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