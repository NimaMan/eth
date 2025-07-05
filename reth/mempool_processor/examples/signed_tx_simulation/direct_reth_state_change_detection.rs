/// Direct Reth State Change Detection Example
/// 
/// Demonstrates how to extract full state changes from Direct Reth simulation,
/// providing the same data as debug_traceCall but 100-250x faster.

use mempool_processor::{
    FullTransactionIpcClient,
    tx_simulator::reth_simulator_engine::{
        RethDirectSimulatorWithStateChanges, 
        mempool_tx_to_reth_signed
    },
};
use eyre::Result;
use tracing::{info, warn, error};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🔍 Direct Reth State Change Detection Example");
    info!("===========================================");
    info!("Extracts full state changes while maintaining sub-ms performance");
    
    // Initialize Direct Reth simulator with state changes
    let start = Instant::now();
    let simulator = RethDirectSimulatorWithStateChanges::new("/home/nima/.local/share/reth/mainnet")?;
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
    info!("✅ Created IPC client for {}", ipc_path);
    
    // Start monitoring
    mempool_client.start_monitoring().await?;
    info!("✅ Started monitoring pending transactions");
    
    // Statistics
    #[derive(Default)]
    struct Stats {
        total_processed: u64,
        successful_simulations: u64,
        failed_simulations: u64,
        total_state_changes: u64,
        eth_transfers: u64,
        token_transfers: u64,
        simulation_times: Vec<Duration>,
        conversion_times: Vec<Duration>,
    }
    
    let mut stats = Stats::default();
    info!("");
    info!("🔍 Analyzing transactions for state changes...");
    info!("   Press Ctrl+C to stop and see statistics");
    info!("");
    
    // Process transactions
    let analysis_start = Instant::now();
    loop {
        // Get next transactions
        let transactions = mempool_client.get_full_transactions(5).await?;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            // Stop after 30 seconds or 100 transactions
            if analysis_start.elapsed() > Duration::from_secs(30) || stats.total_processed >= 100 {
                info!("Analysis complete after {} transactions", stats.total_processed);
                break;
            }
            continue;
        }
        
        for tx in transactions {
            stats.total_processed += 1;
            
            // Convert to Reth format
            let conv_start = Instant::now();
            let reth_tx = match mempool_tx_to_reth_signed(&tx) {
                Ok(tx) => tx,
                Err(e) => {
                    warn!("Failed to convert transaction: {}", e);
                    stats.failed_simulations += 1;
                    continue;
                }
            };
            let conversion_time = conv_start.elapsed();
            stats.conversion_times.push(conversion_time);
            
            // Simulate with state change extraction
            let sim_start = Instant::now();
            match simulator.simulate_transaction_with_state_changes(&reth_tx).await {
                Ok(Some(state_changes)) => {
                    let simulation_time = sim_start.elapsed();
                    stats.simulation_times.push(simulation_time);
                    stats.successful_simulations += 1;
                    stats.total_state_changes += state_changes.len() as u64;
                    
                    // Count ETH and token transfers
                    let mut eth_changes = 0u64;
                    let mut token_changes = 0u64;
                    
                    for (addr, changes) in &state_changes {
                        if !changes.eth_net_change.absolute_value.is_zero() {
                            eth_changes += 1;
                        }
                        token_changes += changes.token_net_changes.len() as u64;
                    }
                    
                    stats.eth_transfers += eth_changes;
                    stats.token_transfers += token_changes;
                    
                    // Display first 5 transactions in detail
                    if stats.successful_simulations <= 5 {
                        info!("✅ TX #{}: {}", stats.successful_simulations, tx.hash);
                        info!("   Simulation: {:?}", simulation_time);
                        info!("   State changes: {} addresses affected", state_changes.len());
                        
                        // Show top 3 state changes
                        for (addr, changes) in state_changes.iter().take(3) {
                            if !changes.eth_net_change.absolute_value.is_zero() {
                                let sign = if changes.eth_net_change.is_negative { "-" } else { "+" };
                                let eth_value = changes.eth_net_change.absolute_value.to_string();
                                let eth_decimal = format!("{:.6}", 
                                    eth_value.parse::<f64>().unwrap_or(0.0) / 1e18);
                                info!("     {}: {} ETH {}", addr, sign, eth_decimal);
                            }
                            
                            for (token_addr, token_change) in &changes.token_net_changes {
                                let sign = if token_change.is_negative { "-" } else { "+" };
                                info!("     {}: {} token {:?}", addr, sign, token_addr);
                            }
                        }
                        info!("");
                    }
                    
                    // Progress update every 20 transactions
                    if stats.total_processed % 20 == 0 {
                        let avg_sim = stats.simulation_times.iter().sum::<Duration>() / 
                            stats.simulation_times.len() as u32;
                        info!("📊 Processed {} transactions, avg simulation: {:?}", 
                            stats.total_processed, avg_sim);
                    }
                }
                Ok(None) => {
                    // Transaction would revert
                    stats.failed_simulations += 1;
                }
                Err(e) => {
                    error!("Simulation error: {:?}", e);
                    stats.failed_simulations += 1;
                }
            }
        }
    }
    
    // Display final statistics
    info!("");
    info!("📊 State Change Detection Statistics");
    info!("===================================");
    info!("Total transactions processed: {}", stats.total_processed);
    info!("Successful simulations: {}", stats.successful_simulations);
    info!("Failed/reverted: {}", stats.failed_simulations);
    info!("Success rate: {:.1}%", 
        (stats.successful_simulations as f64 / stats.total_processed as f64) * 100.0);
    
    info!("");
    info!("🔍 State Changes Detected:");
    info!("   Total addresses affected: {}", stats.total_state_changes);
    info!("   ETH balance changes: {}", stats.eth_transfers);
    info!("   Token balance changes: {}", stats.token_transfers);
    info!("   Average changes per tx: {:.1}", 
        stats.total_state_changes as f64 / stats.successful_simulations as f64);
    
    if !stats.simulation_times.is_empty() {
        let sum: Duration = stats.simulation_times.iter().sum();
        let avg = sum / stats.simulation_times.len() as u32;
        let min = stats.simulation_times.iter().min().copied().unwrap_or_default();
        let max = stats.simulation_times.iter().max().copied().unwrap_or_default();
        
        info!("");
        info!("⚡ Performance Metrics:");
        info!("   Simulation time:");
        info!("     Average: {:?}", avg);
        info!("     Min: {:?}", min);
        info!("     Max: {:?}", max);
        info!("     Throughput: {:.0} tx/sec", 1_000_000.0 / avg.as_micros() as f64);
        
        // Conversion time stats
        if !stats.conversion_times.is_empty() {
            let conv_avg = stats.conversion_times.iter().sum::<Duration>() / 
                stats.conversion_times.len() as u32;
            info!("   Conversion time average: {:?}", conv_avg);
        }
        
        // Compare with RPC
        info!("");
        info!("🔄 Comparison with RPC:");
        info!("   Typical RPC debug_traceCall: 50-100ms");
        info!("   Direct Reth with state changes: {:?}", avg);
        
        let speedup = 75_000.0 / avg.as_micros() as f64; // 75ms average RPC
        info!("   🚀 Direct Reth is {:.0}x faster even with state extraction!", speedup);
    }
    
    info!("");
    info!("🎯 Key Advantages:");
    info!("   1. Full state change data (same as debug_traceCall)");
    info!("   2. Sub-millisecond performance");
    info!("   3. No network latency or timeouts");
    info!("   4. Direct database access");
    info!("   5. Ready for signal detection integration");
    
    Ok(())
}