/// TxSimulator API Demo with NonBlockingIpcClient
/// 
/// Demonstrates the unified TxSimulator API that automatically handles latest blocks
/// and nonce retry logic, combined with microsecond-latency NonBlockingIpcClient.

use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, NonBlockingTransaction};
use reth_tx_simulator::DirectTxSimulator;
use eyre::Result;
use tracing::{info, error, warn};
use std::time::{Duration, Instant};
use tokio::time::timeout;

async fn convert_to_full_transaction(tx: &NonBlockingTransaction) -> mempool_processor::mempool_fetcher::FullTransaction {
    mempool_processor::mempool_fetcher::FullTransaction {
        hash: tx.hash.clone(),
        tx_data: tx.data.clone(),
        detection_time: Instant::now(),
        latency_ns: tx.detection_ns,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n🚀 TxSimulator API Demo with NonBlockingIpcClient");
    println!("=================================================\n");

    // Initialize DirectTxSimulator 
    let start = Instant::now();
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ DirectTxSimulator initialized in {:?}", start.elapsed());
    info!("   🎯 Direct Reth database access for ultra-fast simulation");

    // Connect to mempool via NonBlockingIpcClient
    info!("📡 Connecting to mempool via NonBlockingIpcClient...");
    let ipc_client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;
    info!("✅ NonBlockingIpcClient started - Sub-10μs detection!");

    let mut total_processed = 0u64;
    let mut successful_simulations = 0u64;
    let mut state_changes_found = 0u64;

    info!("\n🔄 Starting transaction processing...");

    // Process 10 transactions as demo
    while total_processed < 10 {
        // Get up to 5 transactions at once (instant fetch)
        let new_txs = ipc_client.get_transactions_instant(5).await;
        
        if new_txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        for tx in new_txs {
            total_processed += 1;
            
            info!("\n📊 Processing transaction #{}", total_processed);
            info!("   Hash: {}", &tx.hash[..16]);
            info!("   Detection latency: {:.3}ms", tx.detection_ns as f64 / 1_000_000.0);

            // Convert to FullTransaction for TxSimulator
            let full_tx = convert_to_full_transaction(&tx).await;

            // Convert to CallRequest for DirectTxSimulator
            let call_request = match reth_tx_simulator::ipc_to_call_request(&full_tx.tx_data) {
                Ok(req) => req,
                Err(e) => {
                    warn!("   ❌ Failed to convert transaction: {}", e);
                    continue;
                }
            };
            
            // Simulate with DirectTxSimulator (with call trace for state changes)
            let sim_start = Instant::now();
            match timeout(Duration::from_millis(5000), simulator.simulate_unsigned_transaction_with_call_trace(call_request)).await {
                Ok(Ok(state_changes)) => {
                    let sim_time = sim_start.elapsed();
                    successful_simulations += 1;
                    
                    info!("   ✅ Simulation successful in {:.3}ms", sim_time.as_secs_f64() * 1000.0);
                    info!("   🎯 Addresses affected: {}", state_changes.len());

                    if !state_changes.is_empty() {
                        state_changes_found += 1;
                        
                        // Show first few state changes
                        for (i, (address, changes)) in state_changes.iter().take(3).enumerate() {
                            let address_str = mempool_processor::common::address::alloy_address_to_checksum(*address);
                            info!("     {}. Address: {}", i + 1, address_str);
                            
                            if changes.eth_net.abs() > 0.0 {
                                info!("        ETH change: {:.8} ETH", changes.eth_net);
                            }
                            
                            if !changes.token_net.is_empty() {
                                info!("        Token changes: {}", changes.token_net.len());
                                for (token, amount) in changes.token_net.iter().take(2) {
                                    info!("          {}: {:.8}", token, amount);
                                }
                            }
                        }
                    } else {
                        info!("   ℹ️  No state changes detected");
                    }
                }
                Ok(Err(e)) => {
                    let sim_time = sim_start.elapsed();
                    warn!("   ❌ Simulation failed in {:.3}ms: {}", sim_time.as_secs_f64() * 1000.0, e);
                }
                Err(_) => {
                    error!("   💥 Simulation timed out after 5 seconds");
                }
            }

            if total_processed >= 10 {
                break;
            }
        }
    }

    // Summary
    info!("\n📊 DEMO SUMMARY:");
    info!("   Total processed: {}", total_processed);
    info!("   Successful simulations: {} ({:.1}%)", 
          successful_simulations, 
          (successful_simulations as f64 / total_processed as f64) * 100.0);
    info!("   Transactions with state changes: {} ({:.1}%)", 
          state_changes_found,
          (state_changes_found as f64 / total_processed as f64) * 100.0);
    
    info!("\n✅ Demo completed successfully!");
    info!("💡 This example demonstrates:");
    info!("   - NonBlockingIpcClient for microsecond detection");
    info!("   - TxSimulator with automatic latest block handling");
    info!("   - Nonce retry logic built-in");
    info!("   - Clean, unified API");

    Ok(())
}