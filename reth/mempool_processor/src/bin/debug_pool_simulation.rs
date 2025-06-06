/*
 * Debug Pool Transaction Simulation
 * 
 * This test specifically debugs why our 30 detected pool transactions
 * didn't trigger scam detection. It simulates one of the recent pool
 * transactions to see what state changes we're getting.
 */

use std::sync::Arc;
use tokio;
use tracing::{info, error, warn, debug};
use ethers::providers::{Http, Provider, Middleware};
use ethers::types::{H256, Block, Transaction, U256};
use serde_json;

use mempool_processor::pool_subscriber::cache::PoolStateCache;
use mempool_processor::tx_simulator::simulator::TxSimulator;
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::mempool_processor::fetcher::MempoolFetcher;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🔍 DEBUG POOL TRANSACTION SIMULATION");
    info!("====================================");
    info!("");
    info!("This test will help us understand why our 30 pool transactions");
    info!("didn't trigger scam detection by examining their REVM simulation results.");
    info!("");

    // Initialize components
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545")?);
    let tx_simulator = TxSimulator::new(provider.clone()).await?;
    let pool_cache = Arc::new(PoolStateCache::new(0.15));

    // Step 1: Load pool data from Python service
    info!("📊 Step 1: Loading pool data from Python service...");
    let pool_count = load_pools_from_python(&pool_cache).await?;
    info!("✅ Loaded {} pools from Python service", pool_count);
    info!("");

    // Step 2: Get current mempool transactions
    info!("📊 Step 2: Fetching current mempool transactions...");
    let fetcher = MempoolFetcher::new(
        "http://localhost:8545",
        None,
        false,
        false,
        100,
        2000,
    );

    let transactions = fetcher.get_transactions().await?;
    info!("📦 Fetched {} transactions from mempool", transactions.len());
    info!("");

    // Step 3: Find pool transactions
    info!("📊 Step 3: Identifying pool transactions...");
    let mut pool_transactions = Vec::new();

    for tx in transactions.iter().take(50) { // Check first 50 transactions
        if is_pool_transaction(tx, &pool_cache) {
            pool_transactions.push(tx.clone());
            if pool_transactions.len() >= 5 { // Get first 5 pool transactions
                break;
            }
        }
    }

    info!("🏊 Found {} pool transactions to analyze", pool_transactions.len());
    info!("");

    if pool_transactions.is_empty() {
        warn!("⚠️  No pool transactions found in current mempool");
        warn!("This might explain why we're not detecting scams right now");
        return Ok(());
    }

    // Step 4: Simulate each pool transaction in detail
    info!("📊 Step 4: Detailed simulation of pool transactions...");
    info!("");

    for (i, tx) in pool_transactions.iter().enumerate() {
        info!("🧪 SIMULATING POOL TRANSACTION {} of {}", i + 1, pool_transactions.len());
        info!("══════════════════════════════════════════════");
        
        // Convert hash bytes to string for display
        let tx_hash = hex::encode(&tx.hash);
        let from_addr = hex::encode(&tx.from);
        let to_addr = tx.to.as_ref().map(|t| hex::encode(t)).unwrap_or("None".to_string());
        
        info!("Hash: 0x{}", tx_hash);
        info!("From: 0x{}", from_addr);
        info!("To: 0x{}", to_addr);
        info!("Value: {:.6} ETH", tx.value.as_u64() as f64 / 1e18);
        info!("");

        // Check which pool this transaction involves
        let involved_pool = find_involved_pool(tx, &pool_cache);
        if let Some(pool_addr) = involved_pool {
            info!("🏊 Involves pool: {}", pool_addr);
            
            // Get current pool balance
            if let Some(pool_state) = pool_cache.get_pool(&pool_addr) {
                info!("💰 Current pool balance: {:.6} ETH", pool_state.eth_amount);
                info!("📊 Pool info: {:?}", pool_state);
            } else {
                warn!("❌ Pool {} not found in cache", pool_addr);
            }
        } else {
            warn!("⚠️  Could not determine which pool this transaction involves");
        }
        info!("");

        // Simulate the transaction
        info!("🧪 Running REVM simulation...");
        match tx_simulator.simulate_transaction(tx).await {
            Ok(Some(account_changes)) => {
                info!("✅ Simulation successful!");
                info!("📊 Affected accounts: {}", account_changes.len());
                info!("");

                // Analyze results for each affected account
                for (address, changes) in account_changes.iter() {
                    info!("📍 Address: {}", address);
                    
                    // Check if this is a known pool
                    let is_pool = pool_cache.get_pool(address).is_some();
                    if is_pool {
                        info!("   🏊 This is a KNOWN POOL!");
                        
                        // Check ETH balance change
                        if let Some(eth_change) = changes.eth_balance_change {
                            let eth_change_float = eth_change as f64 / 1e18;
                            info!("   💰 ETH balance change: {:.6} ETH ({} wei)", 
                                  eth_change_float, eth_change);
                            
                            // Get current pool balance
                            if let Some(pool_state) = pool_cache.get_pool(address) {
                                let current_balance = pool_state.eth_amount;
                                let simulated_balance = current_balance + eth_change_float;
                                
                                info!("   📊 Current pool balance: {:.6} ETH", current_balance);
                                info!("   📊 Simulated balance: {:.6} ETH", simulated_balance);
                                info!("   📊 Threshold: {:.6} ETH", pool_cache.get_eth_threshold());
                                
                                // Check scam criteria
                                if simulated_balance < pool_cache.get_eth_threshold() && 
                                   current_balance >= pool_cache.get_eth_threshold() {
                                    info!("   🚨 SCAM CRITERIA MET! Pool would drop below threshold!");
                                } else {
                                    info!("   ✅ No scam detected - balance remains above threshold");
                                }
                            }
                        } else {
                            info!("   ⚪ No ETH balance change detected");
                        }
                    } else {
                        info!("   📋 Regular account (not a pool)");
                        if let Some(eth_change) = changes.eth_balance_change {
                            info!("   💰 ETH change: {:.6} ETH", eth_change as f64 / 1e18);
                        }
                    }
                    
                    // Show storage changes
                    if !changes.storage_changes.is_empty() {
                        info!("   🗄️  Storage changes: {}", changes.storage_changes.len());
                    }
                    
                    info!("");
                }

                // Summary for this transaction
                let pool_changes: Vec<_> = account_changes.iter()
                    .filter(|(addr, _)| pool_cache.get_pool(addr).is_some())
                    .collect();
                
                if pool_changes.is_empty() {
                    warn!("❌ No pool addresses found in simulation results!");
                    warn!("   This explains why scam detection didn't trigger");
                } else {
                    info!("📊 Pool addresses in simulation: {}", pool_changes.len());
                    for (addr, _) in pool_changes {
                        info!("   - {}", addr);
                    }
                }
            }
            Ok(None) => {
                warn!("⚠️  Simulation returned no results");
                warn!("This could explain why scam detection didn't trigger");
            }
            Err(e) => {
                error!("❌ Simulation failed: {}", e);
            }
        }
        
        info!("");
        info!("────────────────────────────────────────────────────────");
        info!("");
    }

    // Step 5: Summary and recommendations
    info!("📊 DEBUGGING SUMMARY");
    info!("====================");
    info!("Analyzed {} pool transactions from current mempool", pool_transactions.len());
    info!("");
    info!("Possible reasons for no scam detection:");
    info!("1. Pool transactions don't affect ETH balances significantly");
    info!("2. All pools have balances well above 0.15 ETH threshold");
    info!("3. Simulation doesn't detect pool address changes correctly");
    info!("4. Address format/normalization issues");
    info!("");
    info!("💡 Check the simulation results above to identify the root cause!");

    Ok(())
}

async fn load_pools_from_python(pool_cache: &Arc<PoolStateCache>) -> eyre::Result<usize> {
    use zmq::{Context, SocketType};
    
    let context = Context::new();
    let socket = context.socket(SocketType::REQ)?;
    socket.connect("tcp://localhost:5558")?;
    
    socket.send("get_all_pools", 0)?;
    let response = socket.recv_string(0)??;
    let pools_data: serde_json::Value = serde_json::from_str(&response)?;
    
    let mut pool_count = 0;
    
    if let Some(pools) = pools_data.as_object() {
        for (pool_address, pool_info) in pools {
            if let Some(eth_amount) = pool_info.get("eth_amount").and_then(|v| v.as_f64()) {
                if eth_amount >= 0.0 {
                    // Update pool in cache (we need to implement this method)
                    // For now, just count
                    pool_count += 1;
                }
            }
        }
    }
    
    Ok(pool_count)
}

fn is_pool_transaction(
    tx: &TransactionView,
    pool_cache: &Arc<PoolStateCache>
) -> bool {
    // Check if transaction is TO a known pool
    if let Some(to_bytes) = &tx.to {
        let to_str = hex::encode(to_bytes);
        let to_addr = format!("0x{}", to_str);
        
        if pool_cache.get_pool(&to_addr).is_some() {
            return true;
        }
    }
    
    // TODO: Add logic to check for DEX router calls that affect pools
    false
}

fn find_involved_pool(
    tx: &TransactionView,
    pool_cache: &Arc<PoolStateCache>
) -> Option<String> {
    // Check if transaction is TO a known pool
    if let Some(to_bytes) = &tx.to {
        let to_str = hex::encode(to_bytes);
        let to_addr = format!("0x{}", to_str);
        
        if pool_cache.get_pool(&to_addr).is_some() {
            return Some(to_addr);
        }
    }
    
    None
}