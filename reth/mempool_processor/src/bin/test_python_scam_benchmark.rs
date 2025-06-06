/*
 * Python Scam Detection Benchmark Test
 * 
 * This test uses the actual scam transactions detected by the Python service
 * to verify our Rust REVM simulation and state change detection.
 * 
 * Python detected scams:
 * 1. 15:40:49 - Token: 0x6426b6C2A9108Fa815bcccA3a3232301E1895742
 *    Pool: 0x548Db8fC431Dd7c39817BF0a59638B2bCA2eAcD5
 *    ETH: 0.559 → 0.289 (below threshold 0.4)
 * 
 * 2. 15:41:49 - Token: 0x8B77fE013C078ea92260589De96c1C5EE464da02  
 *    Pool: 0xB987d8E7A1F8594aD000B55c40b3deD8d8dfE93A
 *    ETH: 6.811687 → 0.000000 (complete drain!)
 */

use std::sync::Arc;
use tokio;
use tracing::{info, error, warn, debug};
use ethers::providers::{Http, Provider, Middleware};
use ethers::types::{H256, Block, TransactionReceipt, U256};
use std::str::FromStr;

use mempool_processor::pool_subscriber::cache::PoolStateCache;
use mempool_processor::tx_simulator::simulator::TxSimulator;
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::scam_detection::engine::ScamDetectionEngine;

struct PythonScamCase {
    description: &'static str,
    token_address: &'static str,
    pool_address: &'static str,
    current_eth: f64,
    simulated_eth: f64,
    threshold: f64,
    block_number: u64,
}

const PYTHON_SCAM_CASES: &[PythonScamCase] = &[
    PythonScamCase {
        description: "Scam 1: ETH drainage below threshold",
        token_address: "0x6426b6C2A9108Fa815bcccA3a3232301E1895742",
        pool_address: "0x548Db8fC431Dd7c39817BF0a59638B2bCA2eAcD5",
        current_eth: 0.559,
        simulated_eth: 0.289,
        threshold: 0.4,
        block_number: 22638767, // Approximate block when detected
    },
    PythonScamCase {
        description: "Scam 2: Complete pool drainage",
        token_address: "0x8B77fE013C078ea92260589De96c1C5EE464da02",
        pool_address: "0xB987d8E7A1F8594aD000B55c40b3deD8d8dfE93A",
        current_eth: 6.811687,
        simulated_eth: 0.0,
        threshold: 0.4,
        block_number: 22638772, // Approximate block when detected
    },
];

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🔬 PYTHON SCAM DETECTION BENCHMARK TEST");
    info!("==========================================");
    info!("");
    info!("Testing our Rust REVM simulation against Python-detected scams");
    info!("");

    // Initialize components
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545")?);
    let tx_simulator = TxSimulator::new(provider.clone()).await?;
    let pool_cache = Arc::new(PoolStateCache::new(0.15));
    let scam_engine = ScamDetectionEngine::new(pool_cache.clone());

    // Test each Python-detected scam case
    for (i, case) in PYTHON_SCAM_CASES.iter().enumerate() {
        info!("📊 Testing Case {}: {}", i + 1, case.description);
        info!("   Token: {}", case.token_address);
        info!("   Pool: {}", case.pool_address);
        info!("   Expected: {:.6} → {:.6} ETH", case.current_eth, case.simulated_eth);
        info!("");

        // First, add the pool to our cache with the expected current ETH level
        let pool_addr = case.pool_address.parse()
            .map_err(|e| eyre::eyre!("Invalid pool address: {}", e))?;
        pool_cache.update_pool_eth_balance(pool_addr, case.current_eth);
        
        info!("✅ Added pool to cache with current ETH: {:.6}", case.current_eth);

        // Get transactions from the block where the scam was detected
        match get_transactions_from_block(&provider, case.block_number).await {
            Ok(transactions) => {
                info!("📦 Found {} transactions in block {}", transactions.len(), case.block_number);
                
                // Test each transaction to see if we can detect the scam
                let mut scam_found = false;
                let mut pool_transactions_found = 0;
                
                for (tx_idx, tx) in transactions.iter().enumerate() {
                    // Check if this transaction involves our target pool
                    if transaction_involves_pool(tx, case.pool_address) {
                        pool_transactions_found += 1;
                        info!("🏊 Found pool transaction {}: {}", pool_transactions_found, tx.hash);
                        
                        // Test our REVM simulation
                        match test_revm_simulation(&tx_simulator, tx, case).await {
                            Ok(detected) => {
                                if detected {
                                    scam_found = true;
                                    info!("✅ SCAM DETECTED by our Rust implementation!");
                                } else {
                                    warn!("❌ Our implementation did NOT detect scam in this transaction");
                                }
                            }
                            Err(e) => {
                                error!("❌ REVM simulation failed: {}", e);
                            }
                        }
                    }
                }
                
                if pool_transactions_found == 0 {
                    warn!("⚠️  No transactions involving pool {} found in block {}", 
                          case.pool_address, case.block_number);
                    warn!("   This might mean:");
                    warn!("   1. The scam was detected in mempool before mining");
                    warn!("   2. Different block number");
                    warn!("   3. Our pool detection logic needs improvement");
                } else {
                    info!("📊 Summary: {} pool transactions found, scam detected: {}", 
                          pool_transactions_found, scam_found);
                }
            }
            Err(e) => {
                error!("❌ Failed to get transactions from block {}: {}", case.block_number, e);
            }
        }
        
        info!("");
        info!("────────────────────────────────────────────────────────");
        info!("");
    }

    info!("🏁 Benchmark test completed!");
    info!("");
    info!("Next steps if scams not detected:");
    info!("1. Check if our pool address detection logic works");
    info!("2. Verify REVM simulation matches Python's simulation");
    info!("3. Compare state diff extraction methods");
    info!("4. Test with mempool transactions instead of mined ones");

    Ok(())
}

async fn get_transactions_from_block(
    provider: &Arc<Provider<Http>>, 
    block_number: u64
) -> eyre::Result<Vec<TransactionView>> {
    let block = provider.get_block_with_txs(block_number).await?
        .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;
    
    let mut transactions = Vec::new();
    
    for tx in block.transactions {
        let transaction_view = TransactionView {
            hash: format!("{:#x}", tx.hash),
            from: format!("{:#x}", tx.from),
            to: tx.to.map(|addr| format!("{:#x}", addr)),
            value: tx.value,
            gas: tx.gas,
            gas_price: tx.gas_price,
            input: tx.input,
            nonce: tx.nonce,
            transaction_type: tx.transaction_type,
            max_fee_per_gas: tx.max_fee_per_gas,
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas,
            chain_id: tx.chain_id,
        };
        transactions.push(transaction_view);
    }
    
    Ok(transactions)
}

fn transaction_involves_pool(tx: &TransactionView, pool_address: &str) -> bool {
    // Check if transaction is TO the pool address
    if let Some(to) = &tx.to {
        if to.to_lowercase() == pool_address.to_lowercase() {
            return true;
        }
    }
    
    // TODO: Also check transaction input data for pool interactions
    // This would catch DEX router transactions that interact with pools
    
    false
}

async fn test_revm_simulation(
    tx_simulator: &TxSimulator,
    tx: &TransactionView,
    case: &PythonScamCase
) -> eyre::Result<bool> {
    info!("🧪 Testing REVM simulation for transaction: {}", tx.hash);
    
    // Simulate the transaction
    match tx_simulator.simulate_transaction(tx).await {
        Ok(Some(account_changes)) => {
            info!("✅ REVM simulation successful - {} accounts affected", account_changes.len());
            
            // Check if the pool address is in the account changes
            let pool_addr_lowercase = case.pool_address.to_lowercase();
            let mut pool_found = false;
            
            for (address, changes) in account_changes.iter() {
                if address.to_lowercase() == pool_addr_lowercase {
                    pool_found = true;
                    info!("🏊 Pool {} found in simulation results", address);
                    
                    // Extract ETH balance changes
                    if let Some(eth_change) = changes.eth_balance_change {
                        let current_balance = case.current_eth;
                        let simulated_balance = current_balance + (eth_change as f64 / 1e18);
                        
                        info!("💰 ETH Balance Simulation:");
                        info!("   Current: {:.6} ETH", current_balance);
                        info!("   Change: {:.6} ETH", eth_change as f64 / 1e18);
                        info!("   Simulated: {:.6} ETH", simulated_balance);
                        info!("   Python Expected: {:.6} ETH", case.simulated_eth);
                        
                        // Check if this matches Python's detection criteria
                        if simulated_balance < case.threshold && current_balance >= case.threshold {
                            info!("🚨 SCAM CRITERIA MET: Pool would drop below threshold!");
                            return Ok(true);
                        } else {
                            warn!("❌ Scam criteria not met by our simulation");
                            return Ok(false);
                        }
                    } else {
                        warn!("⚠️  No ETH balance change detected for pool");
                    }
                    break;
                }
            }
            
            if !pool_found {
                warn!("❌ Pool {} not found in REVM simulation results", case.pool_address);
                debug!("   Affected addresses:");
                for address in account_changes.keys() {
                    debug!("     - {}", address);
                }
            }
            
            Ok(false)
        }
        Ok(None) => {
            warn!("⚠️  REVM simulation returned no results");
            Ok(false)
        }
        Err(e) => {
            error!("❌ REVM simulation failed: {}", e);
            Err(e.into())
        }
    }
}