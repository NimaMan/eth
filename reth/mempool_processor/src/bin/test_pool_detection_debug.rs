/*
 * Pool Detection Debug Test
 * 
 * This test specifically checks if our pool detection logic is working
 * by fetching current mempool transactions and seeing if we can detect
 * any pool interactions.
 */

use std::sync::Arc;
use tokio;
use tracing::{info, error, warn, debug};
use ethers::providers::{Http, Provider, Middleware};
use serde_json;

use mempool_processor::pool_subscriber::cache::PoolStateCache;
use mempool_processor::mempool_processor::fetcher::MempoolFetcher;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🔍 POOL DETECTION DEBUG TEST");
    info!("=============================");
    info!("");
    info!("Testing our pool detection logic with current mempool transactions");
    info!("");

    // Initialize components
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545")?);
    
    // Initialize pool cache and load from Python service (like production)
    let pool_cache = Arc::new(PoolStateCache::new(0.15));
    
    info!("📊 Step 1: Loading pool data from Python service...");
    match load_pools_from_python(&pool_cache).await {
        Ok(pool_count) => {
            info!("✅ Loaded {} pools from Python service", pool_count);
        }
        Err(e) => {
            error!("❌ Failed to load pools: {}", e);
            return Err(e);
        }
    }

    info!("");
    info!("📊 Step 2: Testing pool address recognition...");
    
    // Test the scam pool addresses specifically
    let test_pools = vec![
        "0x548Db8fC431Dd7c39817BF0a59638B2bCA2eAcD5", // Python scam 1
        "0xB987d8E7A1F8594aD000B55c40b3deD8d8dfE93A", // Python scam 2
    ];
    
    for pool_addr in &test_pools {
        let addr = pool_addr.parse()?;
        if let Some(eth_balance) = pool_cache.get_pool_eth_balance(addr) {
            info!("✅ Pool {} found in cache with {:.6} ETH", pool_addr, eth_balance);
        } else {
            warn!("❌ Pool {} NOT found in cache", pool_addr);
        }
    }

    info!("");
    info!("📊 Step 3: Fetching current mempool transactions...");
    
    // Create mempool fetcher
    let fetcher = MempoolFetcher::new(
        "http://localhost:8545",
        None, // No WS URL
        false, // Don't use streaming
        false, // Don't use RPC batch
        1000,  // Batch size
        2000,  // Timeout
    ).await?;
    
    // Get transactions
    let transactions = fetcher.get_transactions().await?;
    info!("📦 Fetched {} transactions from mempool", transactions.len());
    
    if transactions.is_empty() {
        warn!("⚠️  No transactions in mempool to test");
        return Ok(());
    }
    
    info!("");
    info!("📊 Step 4: Testing pool detection on mempool transactions...");
    
    let mut pool_transactions_found = 0;
    let mut transactions_checked = 0;
    
    for tx in transactions.iter().take(100) { // Test first 100 transactions
        transactions_checked += 1;
        
        // Check if this transaction involves any known pool
        let is_pool_tx = check_transaction_involves_pools(&tx, &pool_cache);
        
        if is_pool_tx {
            pool_transactions_found += 1;
            info!("🏊 Pool transaction found: {}", tx.hash);
            info!("   From: {}", tx.from);
            if let Some(to) = &tx.to {
                info!("   To: {}", to);
            }
            info!("   Value: {:.6} ETH", tx.value.as_u64() as f64 / 1e18);
            
            // Show which pool it interacts with
            if let Some(to) = &tx.to {
                let addr = to.parse().ok();
                if let Some(addr) = addr {
                    if let Some(eth_balance) = pool_cache.get_pool_eth_balance(addr) {
                        info!("   Pool balance: {:.6} ETH", eth_balance);
                    }
                }
            }
        }
        
        if transactions_checked % 20 == 0 {
            debug!("   Checked {} transactions, found {} pool transactions", 
                   transactions_checked, pool_transactions_found);
        }
    }
    
    info!("");
    info!("📊 RESULTS:");
    info!("   Transactions checked: {}", transactions_checked);
    info!("   Pool transactions found: {}", pool_transactions_found);
    info!("   Pool detection rate: {:.1}%", 
          pool_transactions_found as f64 / transactions_checked as f64 * 100.0);
    
    if pool_transactions_found == 0 {
        warn!("❌ NO POOL TRANSACTIONS DETECTED!");
        warn!("   This explains why our service shows 'pool_transactions: 0'");
        warn!("   Possible issues:");
        warn!("   1. Pool cache not loaded correctly");
        warn!("   2. Pool address format/case mismatch");
        warn!("   3. Transactions are DEX router calls (indirect pool interaction)");
        warn!("   4. Different mempool view than Python");
    } else {
        info!("✅ Pool detection is working!");
    }
    
    info!("");
    info!("📊 Step 5: Sample transaction analysis...");
    
    // Show details of first few transactions for debugging
    for (i, tx) in transactions.iter().take(3).enumerate() {
        info!("🔍 Transaction {} analysis:", i + 1);
        info!("   Hash: {}", tx.hash);
        info!("   From: {}", tx.from);
        if let Some(to) = &tx.to {
            info!("   To: {}", to);
            
            // Check if 'to' address is in our pool cache
            if let Ok(addr) = to.parse() {
                if pool_cache.get_pool_eth_balance(addr).is_some() {
                    info!("   ✅ TO address is a known pool!");
                } else {
                    info!("   ❌ TO address is not in pool cache");
                }
            }
        } else {
            info!("   To: (contract creation)");
        }
        info!("   Value: {:.6} ETH", tx.value.as_u64() as f64 / 1e18);
        info!("");
    }

    Ok(())
}

async fn load_pools_from_python(pool_cache: &Arc<PoolStateCache>) -> eyre::Result<usize> {
    use zmq::{Context, SocketType};
    
    let context = Context::new();
    let socket = context.socket(SocketType::REQ)?;
    socket.connect("tcp://localhost:5558")?;
    
    // Send request for pool data
    socket.send("get_all_pools", 0)?;
    
    // Receive response
    let response = socket.recv_string(0)??;
    let pools_data: serde_json::Value = serde_json::from_str(&response)?;
    
    let mut pool_count = 0;
    
    if let Some(pools) = pools_data.as_object() {
        for (pool_address, pool_info) in pools {
            if let Some(eth_amount) = pool_info.get("eth_amount").and_then(|v| v.as_f64()) {
                if eth_amount >= 0.0 {
                    if let Ok(addr) = pool_address.parse() {
                        pool_cache.update_pool_eth_balance(addr, eth_amount);
                        pool_count += 1;
                    }
                }
            }
        }
    }
    
    Ok(pool_count)
}

fn check_transaction_involves_pools(
    tx: &mempool_processor::mempool_processor::types::TransactionView,
    pool_cache: &Arc<PoolStateCache>
) -> bool {
    // Check if transaction is TO a known pool
    if let Some(to) = &tx.to {
        if let Ok(addr) = to.parse() {
            if pool_cache.get_pool_eth_balance(addr).is_some() {
                return true;
            }
        }
    }
    
    // TODO: Add more sophisticated detection for DEX router calls
    // that indirectly interact with pools through input data
    
    false
}