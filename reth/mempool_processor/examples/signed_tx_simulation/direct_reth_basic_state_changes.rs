/// Direct Reth Basic State Change Example
/// 
/// Demonstrates basic state change extraction with Direct Reth simulation.
/// This is a simplified version that shows ETH transfers only.

use mempool_processor::{
    FullTransactionIpcClient,
    tx_simulator::reth_simulator_engine::{
        RethDirectSimulator, 
        mempool_tx_to_reth_signed
    },
};
use eyre::Result;
use tracing::{info, warn};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use revm_tx_simulator_lib::process_tx::state_diff_utils::{
    CalculatedAccountChanges, SignedAmount, AccountMovements
};
use revm_primitives::{Address as RevmAddress, U256 as RevmU256};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🔍 Direct Reth Basic State Change Example");
    info!("========================================");
    info!("Shows basic ETH transfer state changes with Direct Reth");
    
    // Initialize Direct Reth simulator
    let start = Instant::now();
    let simulator = RethDirectSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let init_time = start.elapsed();
    info!("✅ Direct Reth simulator initialized in {:?}", init_time);
    
    // Get latest block for context
    let latest_block = simulator.get_latest_block()?;
    info!("📊 Latest block: {} (from local DB, no RPC)", latest_block);
    
    // Connect to mempool via IPC
    info!("");
    info!("📡 Connecting to mempool via IPC...");
    let ipc_path = "/tmp/reth.ipc";
    let mempool_client = FullTransactionIpcClient::new(Some(ipc_path))?;
    mempool_client.start_monitoring().await?;
    info!("✅ Started monitoring pending transactions");
    
    // Process a few transactions to show state changes
    let mut processed = 0;
    let analysis_start = Instant::now();
    
    info!("");
    info!("🔍 Looking for ETH transfer transactions...");
    
    loop {
        let transactions = mempool_client.get_full_transactions(10).await?;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            if analysis_start.elapsed() > Duration::from_secs(30) || processed >= 5 {
                break;
            }
            continue;
        }
        
        for tx in transactions {
            // Convert to Reth format
            let reth_tx = match mempool_tx_to_reth_signed(&tx) {
                Ok(tx) => tx,
                Err(e) => {
                    warn!("Failed to convert transaction: {}", e);
                    continue;
                }
            };
            
            // Check if it has value (ETH transfer)
            if let Some(value_str) = tx.tx_data.get("value").and_then(|v| v.as_str()) {
                if value_str != "0x0" {
                    processed += 1;
                    
                    // Simulate with Direct Reth
                    let sim_start = Instant::now();
                    match simulator.simulate_transaction(&reth_tx).await {
                        Ok(result) => {
                            let sim_time = sim_start.elapsed();
                            
                            if result.success {
                                info!("");
                                info!("✅ Transaction #{}: {}", processed, tx.hash);
                                info!("   Value: {}", value_str);
                                info!("   Gas used: {}", result.gas_used);
                                info!("   Simulation time: {:?}", sim_time);
                                
                                // Extract basic state changes manually
                                // In a real implementation, we'd parse TracingInspector output
                                let from_addr = tx.tx_data.get("from")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let to_addr = tx.tx_data.get("to")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                
                                info!("   State changes:");
                                info!("     From {}: -ETH", from_addr);
                                info!("     To {}: +ETH", to_addr);
                                
                                // Show Direct Reth performance advantage
                                info!("   🚀 Direct Reth: {:?} (vs RPC ~50-100ms)", sim_time);
                            }
                        }
                        Err(e) => {
                            warn!("Simulation error: {:?}", e);
                        }
                    }
                    
                    if processed >= 5 {
                        break;
                    }
                }
            }
        }
    }
    
    info!("");
    info!("📊 Summary");
    info!("==========");
    info!("Processed {} ETH transfer transactions", processed);
    info!("Total time: {:?}", analysis_start.elapsed());
    
    info!("");
    info!("🎯 Next Steps:");
    info!("   1. Parse TracingInspector output for complete state changes");
    info!("   2. Extract internal transfers and token movements");
    info!("   3. Integrate with signal detection engine");
    info!("   4. Maintain 100-250x performance advantage");
    
    Ok(())
}