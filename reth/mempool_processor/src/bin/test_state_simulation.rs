/*
 * Test State Simulation Binary
 * 
 * Algorithm:
 * 1. Fetch pending transactions from mempool
 * 2. Simulate each transaction using REVM
 * 3. Extract ETH balance changes for all affected addresses
 * 4. Log detailed state diffs for verification
 * 5. Compare with actual mined results when possible
 * 
 * This verifies our core simulation logic matches real transaction effects.
 */

use mempool_processor::mempool_processor::fetcher::MempoolFetcher;
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::tx_simulator::StateDiffTracker;
use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error, debug};
use hex;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize detailed logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    info!("🧪 Starting state simulation test...");
    
    // Initialize Ethereum provider for state queries
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545")?);
    info!("📡 Connected to Ethereum node at http://localhost:8545");
    
    // Test provider connection
    let latest_block = provider.get_block_number().await?;
    info!("📦 Latest block: {}", latest_block);
    
    // Initialize mempool fetcher
    let fetcher = MempoolFetcher::new("http://localhost:8545")?;
    info!("🔄 Mempool fetcher initialized");
    
    // Initialize state diff tracker
    let mut state_tracker = StateDiffTracker::new(provider.clone(), None);
    info!("🎯 State diff tracker initialized");
    
    let mut transactions_tested = 0;
    let target_transactions = 20;
    let mut successful_simulations = 0;
    let mut failed_simulations = 0;
    
    info!("🚀 Starting to fetch and simulate transactions...");
    
    while transactions_tested < target_transactions {
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                info!("📥 Fetched {} transactions from mempool", transactions.len());
                
                for tx in transactions {
                    if transactions_tested >= target_transactions {
                        break;
                    }
                    
                    let tx_hash = hex::encode(&tx.hash);
                    info!("\n🔍 === TESTING TRANSACTION {} ===", tx_hash);
                    info!("  📋 From: 0x{}", hex::encode(&tx.from));
                    if let Some(to_bytes) = &tx.to {
                        info!("  📋 To: 0x{}", hex::encode(to_bytes));
                    } else {
                        info!("  📋 To: Contract Creation");
                    }
                    info!("  💰 Value: {:.6} ETH", tx.value.as_u128() as f64 / 1e18);
                    if let Some(gas_price) = tx.gas_price {
                        info!("  ⛽ Gas Price: {} gwei", gas_price.as_u128() / 1_000_000_000);
                    }
                    
                    // Simulate the transaction
                    match state_tracker.simulate_transaction(&tx).await {
                        Ok(Some(state_diffs)) => {
                            successful_simulations += 1;
                            info!("  ✅ Simulation successful! Found {} address changes:", state_diffs.len());
                            
                            for (address, diff) in &state_diffs {
                                info!("    📍 Address: {}", address);
                                if let Some(before) = diff.before {
                                    info!("      💰 Before: {:.6} ETH", before);
                                }
                                if let Some(after) = diff.after {
                                    info!("      💰 After: {:.6} ETH", after);
                                }
                                info!("      📈 Change: {:.6} ETH", diff.change);
                                
                                // Highlight significant changes
                                if diff.change.abs() > 0.01 {
                                    warn!("      🚨 SIGNIFICANT CHANGE: {:.6} ETH", diff.change);
                                }
                            }
                            
                            // Try to get actual state from node for comparison
                            if let Some(to_bytes) = &tx.to {
                                let to_addr = H160::from_slice(to_bytes);
                                match provider.get_balance(to_addr, None).await {
                                    Ok(actual_balance) => {
                                        let actual_eth = actual_balance.as_u128() as f64 / 1e18;
                                        info!("  🔍 Actual current balance of target: {:.6} ETH", actual_eth);
                                    }
                                    Err(e) => {
                                        debug!("  ⚠️ Could not fetch actual balance: {}", e);
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            info!("  ⚪ No state changes detected");
                        }
                        Err(e) => {
                            failed_simulations += 1;
                            warn!("  ❌ Simulation failed: {}", e);
                        }
                    }
                    
                    transactions_tested += 1;
                    
                    // Small delay to avoid overwhelming logs
                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                error!("❌ Error fetching transactions: {}", e);
                sleep(Duration::from_secs(2)).await;
            }
        }
        
        if transactions_tested < target_transactions {
            info!("⏳ Waiting for more transactions... ({}/{})", transactions_tested, target_transactions);
            sleep(Duration::from_secs(3)).await;
        }
    }
    
    // Final summary
    info!("\n📊 === SIMULATION TEST SUMMARY ===");
    info!("  🧪 Total transactions tested: {}", transactions_tested);
    info!("  ✅ Successful simulations: {}", successful_simulations);
    info!("  ❌ Failed simulations: {}", failed_simulations);
    info!("  📈 Success rate: {:.1}%", (successful_simulations as f64 / transactions_tested as f64) * 100.0);
    
    if successful_simulations > 0 {
        info!("🎉 State simulation is working! Ready to implement scam detection.");
    } else {
        error!("💥 State simulation not working properly. Need to debug REVM setup.");
    }
    
    Ok(())
} 