/*
 * Pool Detection Test Binary
 * 
 * This test processes mempool transactions and logs detailed information about
 * pool level changes for the first 100 transactions that affect monitored pools.
 * 
 * Algorithm:
 * 1. Connect to pool subscriber to get current pool states
 * 2. Fetch transactions from mempool
 * 3. Simulate each transaction to get state changes
 * 4. For each state change that affects a monitored pool:
 *    - Log the pool address (both original and checksummed)
 *    - Log current pool ETH level from Python
 *    - Log simulated ETH change
 *    - Log percentage change
 * 5. Stop after finding 100 pool-affecting transactions
 * 
 * This helps verify:
 * - Address format matching between Python and Rust
 * - Pool state retrieval from cache
 * - ETH level calculation accuracy
 * - Transaction simulation correctness
 */

use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn, error, Level};
use revm_primitives::alloy_primitives::{Address, keccak256};

use mempool_processor::mempool_processor::fetcher::{MempoolFetcher, FetchMode};
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::tx_simulator::StateDiffTracker;
use mempool_processor::pool_subscriber::PoolSubscriber;

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// ZeroMQ socket address for pool updates
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// ETH reserve threshold for pool monitoring
    #[arg(long, default_value = "0.1")]
    eth_threshold: f64,
    
    /// Number of pool-affecting transactions to log
    #[arg(long, default_value = "100")]
    target_count: usize,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
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
    let args = Args::parse();
    
    // Configure logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    
    use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};
    
    let filter = EnvFilter::from_default_env()
        .add_directive("test_pool_detection=info".parse()?)
        .add_directive("mempool_processor=info".parse()?)
        .add_directive("hyper=warn".parse()?)
        .add_directive("tokio_postgres=warn".parse()?)
        .add_directive("h2=warn".parse()?)
        .add_directive("tower=warn".parse()?)
        .add_directive("reqwest=warn".parse()?);
    
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();
    
    info!("🧪 Pool Detection Test Starting");
    info!("ETH RPC URL: {}", args.eth_rpc_url);
    info!("Pool ZMQ Address: {}", args.pool_zmq_address);
    info!("ETH Threshold: {}", args.eth_threshold);
    info!("Target Count: {} pool-affecting transactions", args.target_count);
    
    // Initialize pool subscriber
    info!("Connecting to pool subscriber...");
    let pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    tokio::spawn({
        let pool_subscriber_clone = pool_subscriber;
        async move {
            if let Err(e) = pool_subscriber_clone.start_listening().await {
                error!("Pool subscriber failed: {}", e);
            }
        }
    });
    
    // Wait a moment for pool data to arrive
    info!("Waiting for pool data...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Check how many pools we have
    let pool_count = pool_cache.get_pool_count();
    info!("📊 Monitoring {} pools from Python", pool_count);
    
    if pool_count == 0 {
        warn!("⚠️  No pools found! Make sure Python service is running and sending pool updates");
        return Ok(());
    }
    
    // Initialize mempool fetcher
    info!("Initializing mempool fetcher...");
    let fetcher = MempoolFetcher::with_options(
        &args.eth_rpc_url,
        1000, // cache size
        true, // use batch requests
        25,   // small batch size for testing
        3000, // timeout ms
        FetchMode::RpcBatch
    )?;
    
    // Initialize transaction simulator
    info!("Initializing transaction simulator...");
    let provider = ethers::providers::Provider::<ethers::providers::Http>::try_from(args.eth_rpc_url.clone())?;
    let provider = Arc::new(provider);
    let mut tracker = StateDiffTracker::new(provider.clone(), None);
    
    // Main test loop
    let mut pool_affecting_count = 0;
    let mut total_processed = 0;
    let start_time = Instant::now();
    
    info!("🔍 Starting transaction analysis...");
    
    loop {
        if pool_affecting_count >= args.target_count {
            break;
        }
        
        // Fetch transactions
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                for tx in &transactions {
                    total_processed += 1;
                    
                    // Simulate transaction
                    match tracker.simulate_transaction(tx).await {
                        Ok(Some(changes)) => {
                            let pool_effects = analyze_pool_effects(tx, &changes, &pool_cache);
                            
                            if !pool_effects.is_empty() {
                                pool_affecting_count += 1;
                                
                                let tx_hash = hex::encode(&tx.hash);
                                info!("🎯 Transaction #{} affecting {} pools: {}", 
                                     pool_affecting_count, pool_effects.len(), tx_hash);
                                
                                for effect in pool_effects {
                                    info!("  📍 Pool: {}", effect.pool_address);
                                    info!("    Current ETH: {:.6}", effect.current_eth);
                                    info!("    ETH Delta: {:.6}", effect.eth_delta);
                                    info!("    New ETH: {:.6}", effect.new_eth);
                                    info!("    Change %: {:.2}%", effect.percentage_change * 100.0);
                                    
                                    // Check if this would trigger scam detection
                                    if effect.new_eth < args.eth_threshold {
                                        info!("    🚨 WOULD TRIGGER SCAM ALERT (below {} ETH threshold)", args.eth_threshold);
                                    }
                                    if effect.percentage_change.abs() > 0.5 {
                                        info!("    ⚠️  LARGE CHANGE (>50%)");
                                    }
                                }
                                
                                if pool_affecting_count >= args.target_count {
                                    break;
                                }
                            }
                        },
                        Ok(None) => {
                            // No state changes
                        },
                        Err(_) => {
                            // Simulation failed (common, don't log)
                        }
                    }
                    
                    // Progress update every 1000 transactions
                    if total_processed % 1000 == 0 {
                        info!("📈 Processed {} transactions, found {} pool-affecting", 
                             total_processed, pool_affecting_count);
                    }
                }
            },
            Err(e) => {
                error!("Error fetching transactions: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
        
        // Small delay
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    let elapsed = start_time.elapsed();
    info!("✅ Test completed!");
    info!("📊 Final Stats:");
    info!("  Total transactions processed: {}", total_processed);
    info!("  Pool-affecting transactions: {}", pool_affecting_count);
    info!("  Time elapsed: {:.2}s", elapsed.as_secs_f64());
    info!("  Rate: {:.1} tx/s", total_processed as f64 / elapsed.as_secs_f64());
    
    Ok(())
}

#[derive(Debug)]
struct PoolEffect {
    pool_address: String,
    current_eth: f64,
    eth_delta: f64,
    new_eth: f64,
    percentage_change: f64,
}

fn analyze_pool_effects(
    _tx: &TransactionView,
    changes: &[mempool_processor::tx_simulator::StateChange],
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> Vec<PoolEffect> {
    let mut effects = Vec::new();
    
    for change in changes {
        // Calculate ETH delta
        let eth_delta = (change.balance_after.as_u128() as f64 - 
                         change.balance_before.as_u128() as f64) / 1e18;
        
        // Skip if no significant change
        if eth_delta.abs() < 0.001 {
            continue;
        }
        
        // Convert address to checksummed format
        let address_bytes = change.address.as_bytes();
        let alloy_address = Address::from_slice(address_bytes);
        let checksummed_addr = to_checksum_address(alloy_address);
        
        // Also try lowercase format for comparison
        let lowercase_addr = format!("0x{}", hex::encode(address_bytes).to_lowercase());
        
        // Check if this address is a monitored pool
        if let Some(pool_state) = pool_cache.get_pool(&checksummed_addr) {
            let current_eth = pool_state.eth_reserve;
            let new_eth = current_eth + eth_delta;
            let percentage_change = if current_eth > 0.0 { eth_delta / current_eth } else { 0.0 };
            
            effects.push(PoolEffect {
                pool_address: checksummed_addr,
                current_eth,
                eth_delta,
                new_eth,
                percentage_change,
            });
        } else if let Some(pool_state) = pool_cache.get_pool(&lowercase_addr) {
            // Try lowercase if checksummed didn't work
            let current_eth = pool_state.eth_reserve;
            let new_eth = current_eth + eth_delta;
            let percentage_change = if current_eth > 0.0 { eth_delta / current_eth } else { 0.0 };
            
            info!("🔍 Found pool with lowercase address: {} (checksummed: {})", 
                 lowercase_addr, checksummed_addr);
            
            effects.push(PoolEffect {
                pool_address: lowercase_addr,
                current_eth,
                eth_delta,
                new_eth,
                percentage_change,
            });
        }
    }
    
    effects
} 