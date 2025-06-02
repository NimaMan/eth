/*
 * Scam Detection Service Binary
 * 
 * ALGORITHMIC DESCRIPTION:
 * This binary provides real-time scam detection for Ethereum mempool transactions:
 * 
 * 1. POOL STATE MONITORING:
 *    - ZeroMQ subscriber receives pool updates from Python service
 *    - Maintains real-time cache of pool reserves and addresses
 *    - Tracks ETH reserves for thousands of DeFi pools
 * 
 * 2. MEMPOOL TRANSACTION MONITORING:
 *    - HTTP RPC polling for new mempool transactions (every 50ms)
 *    - Filters and processes only new transactions to avoid duplicates
 *    - Prioritizes transactions that interact with known pools
 * 
 * 3. REVM TRANSACTION SIMULATION:
 *    - Uses validated REVM TransactionSimulator for accurate state prediction
 *    - Simulates each transaction against current blockchain state
 *    - Calculates precise ETH balance changes for all affected accounts
 *    - Generates detailed account state changes using revm_tx_simulator_lib
 * 
 * 4. SCAM DETECTION ANALYSIS:
 *    - Cross-references simulated state changes with pool cache
 *    - Detects pools being drained below ETH threshold (default: 0.15 ETH)
 *    - Identifies large percentage withdrawals (default: >50%)
 *    - Flags suspicious transactions before they can execute
 * 
 * 5. ALERT PERSISTENCE:
 *    - Logs detected scams to PostgreSQL database
 *    - Provides detailed transaction and pool information
 *    - Continues service operation even during database errors
 * 
 * This service serves the main objective of detecting and preventing DeFi pool scams
 * by simulating transactions before they execute and identifying suspicious patterns.
 */

use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{info, error, Level, debug, warn};
use std::collections::HashMap;
use ethers::types::H256;
use revm_primitives::alloy_primitives::{Address, keccak256};
use chrono;

// REVM imports for new simulation approach
use revm_context::BlockEnv as RevmBlockEnv;
use revm_primitives::hardfork::SpecId;
use ethers::providers::{Http as EthersHttp, Middleware, Provider as EthersProvider};
use ethers::types::{BlockId as EthersBlockId, BlockNumber as EthersBlockNumber};
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};

use mempool_processor::mempool_processor::fetcher::{MempoolFetcher, FetchMode};
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::mempool_processor::db_logger::DbLogger;
use mempool_processor::tx_simulator::TransactionSimulator;
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
    
    // Create timestamped log filename to avoid conflicts
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M").to_string();
    let log_path = std::path::Path::new(&args.log_file);
    let log_dir = log_path.parent().unwrap_or(std::path::Path::new("logs"));
    let log_stem = log_path.file_stem().unwrap_or(std::ffi::OsStr::new("scam_detection_service"));
    let log_ext = log_path.extension().unwrap_or(std::ffi::OsStr::new("log"));
    let timestamped_log_file = format!("{}_{}.{}", 
                                      log_stem.to_string_lossy(), 
                                      timestamp, 
                                      log_ext.to_string_lossy());
    let full_log_path = log_dir.join(timestamped_log_file);
    
    println!("Logging to: {:?}", full_log_path);
    
    // Configure logging to file
    let file_appender = tracing_appender::rolling::never(
        log_dir,
        full_log_path.file_name().unwrap()
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
        250,  // max batch size - increased for better throughput
        1000, // timeout ms - optimized for 1-second real-time detection
        FetchMode::RpcBatch
    )?;
    
    // Initialize REVM transaction simulator instead of old StateDiffTracker
    info!("Initializing REVM transaction simulator...");
    let simulator = TransactionSimulator::new(
        &args.eth_rpc_url,
        1, // chain_id (mainnet)
        SpecId::CANCUN
    ).await?;
    
    // Get current block environment for simulation
    info!("Fetching latest block for simulation context...");
    let provider = EthersProvider::<EthersHttp>::try_from(args.eth_rpc_url.clone())?;
    let latest_block = provider
        .get_block(EthersBlockId::Number(EthersBlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    let mut block_env = RevmBlockEnv::default();
    block_env.number = ethers_to_revm_u256(latest_block.number.unwrap_or_default().as_u64().into());
    block_env.beneficiary = latest_block.author.map_or_else(|| revm_primitives::Address::ZERO, |h160| ethers_to_revm_address(h160));
    block_env.timestamp = ethers_to_revm_u256(latest_block.timestamp);
    block_env.gas_limit = latest_block.gas_limit.as_u64();
    block_env.basefee = latest_block.base_fee_per_gas.map_or(0, |bf| bf.as_u64());
    block_env.difficulty = ethers_to_revm_u256(latest_block.difficulty);
    block_env.prevrandao = latest_block.mix_hash.map(|h| revm_primitives::B256::from(h.0));
    
    info!("Block environment: #{}, basefee: {} wei", block_env.number, block_env.basefee);
    
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
    
    // Initialize scam detection service
    info!("Initializing scam detection service...");
    let scam_config = ScamDetectionConfig {
        eth_threshold: args.eth_threshold,
        percentage_threshold: args.percentage_threshold,
    };
    
    info!("🎯 Scam detection thresholds: ETH < {:.3}, Percentage > {:.1}%", 
          args.eth_threshold, args.percentage_threshold * 100.0);
    
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
                        // Skip if we've already processed this transaction recently
                        if fetcher.is_transaction_processed(&tx.hash) {
                            continue;
                        }
                        
                        // Process each transaction using REVM simulation
                        let scams_found = process_transaction_with_revm(
                            tx, 
                            &simulator,
                            &block_env,
                            &service,
                            &db_logger,
                            &pool_cache
                        ).await;
                        
                        // Mark transaction as processed to avoid reprocessing
                        fetcher.mark_transaction_processed(&tx.hash);
                        
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

// Process a single transaction using REVM simulation and return number of scams detected
async fn process_transaction_with_revm(
    tx: &TransactionView,
    simulator: &TransactionSimulator,
    block_env: &RevmBlockEnv,
    service: &ScamDetectionService,
    _db_logger: &Arc<DbLogger>,
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> usize {
    let tx_hash_hex = hex::encode(&tx.hash);
    
    // Only log transaction details for transactions that involve known pools
    let to_addr = if let Some(to_bytes) = &tx.to {
        if !to_bytes.is_empty() {
            format!("0x{}", hex::encode(to_bytes))
        } else {
            "Empty".to_string()
        }
    } else {
        "None".to_string()
    };
    
    // Check if this transaction involves any known pools before detailed logging
    let involves_pool = pool_cache.get_pool(&to_addr).is_some();
    if involves_pool {
        info!("🎯 Transaction {} involves known pool: {} (value: {:.6} ETH)", 
              &tx_hash_hex[..8], to_addr, tx.value.as_u128() as f64 / 1e18);
    }
    
    // Simulate the transaction using REVM
    if tx.hash.len() == 32 {
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&tx.hash);
        let _tx_hash = H256::from(hash_bytes);
        
        // Only log simulation attempt for pool-related transactions
        if involves_pool {
            debug!("🧪 Simulating pool-related transaction {} with REVM", &tx_hash_hex[..8]);
        }
        
        // Use REVM TransactionSimulator to get detailed state changes
        match simulator.process_transaction(tx, block_env).await {
            Ok(Some(account_changes)) => {
                info!("✅ REVM simulation successful for tx {}: {} accounts affected", 
                      &tx_hash_hex[..8], account_changes.len());
                
                // Convert REVM account changes to pool effects format
                let simulation = prepare_simulation_result_from_revm_changes(
                    tx, 
                    &account_changes, 
                    pool_cache.clone()
                );
                
                if let Some(sim_result) = simulation {
                    info!("🔬 Created simulation result for tx {} with {} affected pools", 
                          &tx_hash_hex[..8], sim_result.affected_pools.len());
                    
                    // Log affected pools
                    for (pool_addr, effect) in &sim_result.affected_pools {
                        info!("  🏊 Affected pool {}: {:.6} → {:.6} ETH ({:.2}% change)", 
                              pool_addr, 
                              effect.current_eth_reserve,
                              effect.simulated_eth_reserve,
                              effect.percentage_change * 100.0);
                    }
                    
                    // Process simulation result with the service
                    match service.process_transaction(sim_result).await {
                        Ok(alerts) => {
                            if !alerts.is_empty() {
                                // Only log urgent scam alerts
                                info!("🚨 SCAM DETECTED: {} alerts in tx {}", alerts.len(), tx_hash_hex);
                                
                                for alert in &alerts {
                                    info!("  🚨 Pool {} depleted: {:.6} → {:.6} ETH", 
                                        alert.pool_address, 
                                        alert.current_eth_reserve,
                                        alert.simulated_eth_reserve);
                                }
                                return alerts.len();
                            }
                        },
                        Err(e) => {
                            error!("❌ Scam detection error for tx {}: {}", &tx_hash_hex[..8], e);
                        }
                    }
                }
            },
            Ok(None) => {
                // Only log no state changes for pool-related transactions
                if involves_pool {
                    debug!("⚪ No state changes for pool-related tx {}", &tx_hash_hex[..8]);
                }
            },
            Err(e) => {
                // Only log errors for pool-related transactions or unexpected errors
                if involves_pool || !e.to_string().contains("nonce") {
                    debug!("❌ REVM simulation failed for tx {}: {}", &tx_hash_hex[..8], e);
                }
            }
        }
    } else {
        error!("❌ Invalid transaction hash length for tx {}", tx_hash_hex);
    }
    
    0 // No scams detected
}

// Helper function to prepare a simulation result from REVM account changes
fn prepare_simulation_result_from_revm_changes(
    tx: &TransactionView,
    account_changes: &HashMap<revm_primitives::Address, revm_tx_simulator_lib::state_diff_utils::CalculatedAccountChanges>,
    pool_cache: Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> Option<SimulationResult> {
    let mut affected_pools = HashMap::new();
    let tx_hash_hex = hex::encode(&tx.hash);
    
    // Only log if we find pool interactions
    let mut pool_interactions_found = false;
    
    // Extract pool effects from REVM account changes
    for (address, changes) in account_changes {
        // Convert REVM address to checksummed string format using our utility function
        let addr_str = to_checksum_address(*address);
        
        // Calculate ETH delta from REVM account changes (using correct field name)
        let eth_delta = if changes.eth_net_change.is_negative {
            -(changes.eth_net_change.absolute_value.into_limbs()[0] as u128 as f64 / 1e18)
        } else {
            changes.eth_net_change.absolute_value.into_limbs()[0] as u128 as f64 / 1e18
        };
        
        // Skip addresses where ETH is being added (positive delta)
        if eth_delta >= 0.0 {
            continue;
        }
        
        // Skip very small changes (less than 0.001 ETH)
        if eth_delta.abs() < 0.001 {
            continue;
        }
        
        // Check if this address is a known pool (now using checksummed address)
        if let Some(pool_state) = pool_cache.get_pool(&addr_str) {
            pool_interactions_found = true;
            let current_eth = pool_state.eth_reserve;
            let simulated_eth = current_eth + eth_delta;
            let percentage_change = if current_eth > 0.0 {
                eth_delta / current_eth
            } else {
                0.0
            };
            
            debug!("🏊 Pool {} affected: {:.6} → {:.6} ETH ({:.2}% change)", 
                  addr_str, current_eth, simulated_eth, percentage_change * 100.0);
            
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
    
    // Only log preparation details if we found pool interactions
    if pool_interactions_found {
        debug!("🔧 Preparing simulation result for tx {} with {} pool interactions found", 
               &tx_hash_hex[..8], affected_pools.len());
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
        
        info!("✅ Created simulation result for tx {} with {} affected pools", 
              &tx_hash_hex[..8], affected_pools.len());
        
        Some(SimulationResult {
            tx_hash,
            from: from_addr,
            affected_pools,
        })
    } else {
        None
    }
} 