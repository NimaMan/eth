/// Test mempool simulation with 20K real transactions from mempool fetcher
/// This verifies that our Direct Reth simulator correctly captures address state changes

use reth_tx_simulator::RethDirectTxSimulator;
use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, NonBlockingTransaction};
use tokio::time::{Duration, timeout};
use tracing::{info, warn, error};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Starting mempool simulation test with 20K transactions");
    
    // Initialize Direct Reth simulator
    let simulator = RethDirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct Reth simulator initialized");
    
    // Initialize mempool fetcher  
    let fetcher = NonBlockingIpcClient::new(Some("/tmp/geth.ipc"))?;
    info!("✅ Mempool fetcher connected");
    
    // Create output file
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let output_filename = format!("/home/nima/code/crypto/logs/mempool/simulation_test_20k_{}.log", timestamp);
    let mut output_file = File::create(&output_filename)?;
    
    writeln!(output_file, "MEMPOOL SIMULATION TEST - 20K TRANSACTIONS")?;
    writeln!(output_file, "===========================================")?;
    writeln!(output_file, "Timestamp: {}", chrono::Utc::now())?;
    writeln!(output_file, "Target: 20,000 transactions\n")?;
    
    // Performance tracking
    let mut total_processed = 0u64;
    let mut simulation_successes = 0u64;
    let mut simulation_failures = 0u64;
    let mut simulation_timeouts = 0u64;
    let mut transactions_with_state_changes = 0u64;
    let mut total_addresses_affected = 0u64;
    let mut simulation_times = Vec::new();
    
    // Track error types
    let mut error_counts = HashMap::<String, u64>::new();
    
    info!("📡 Starting to fetch mempool transactions...");
    
    while total_processed < 20_000 {
        // Fetch transactions in batches
        match fetcher.get_transactions(10).await {
            Ok(transactions) => {
                if transactions.is_empty() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                
                for nonblocking_tx in transactions {
                    if total_processed >= 20_000 {
                        break;
                    }
                    
                    total_processed += 1;
                    
                    // Convert transaction to call request format
                    let call_request = match reth_tx_simulator::ipc_to_call_request(&nonblocking_tx.data) {
                        Ok(req) => req,
                        Err(e) => {
                            warn!("Failed to convert tx {}: {}", nonblocking_tx.hash, e);
                            simulation_failures += 1;
                            continue;
                        }
                    };
                    
                    // Use current block number for simulation
                    let current_block = 22_800_000u64; // Recent block number
                    
                    // Simulate with timeout
                    let start_time = std::time::Instant::now();
                    match timeout(Duration::from_millis(100), 
                        simulator.simulate_unsigned_transaction_with_call_trace_at_block(
                            call_request,
                            current_block
                        )).await {
                        
                        // Successful simulation
                        Ok(Ok(state_changes)) => {
                            let sim_time = start_time.elapsed().as_secs_f64() * 1000.0;
                            simulation_times.push(sim_time);
                            simulation_successes += 1;
                            if !state_changes.is_empty() {
                                transactions_with_state_changes += 1;
                                total_addresses_affected += state_changes.len() as u64;
                                
                                // Log significant state changes
                                let mut has_eth_changes = false;
                                let mut eth_total = 0.0;
                                
                                for (address, changes) in &state_changes {
                                    if changes.eth_net.abs() > 0.001 {
                                        has_eth_changes = true;
                                        eth_total += changes.eth_net.abs();
                                    }
                                }
                                
                                if has_eth_changes && eth_total > 0.1 {
                                    writeln!(output_file, 
                                        "TX {} | {} addresses | {:.3} ETH moved | {:.2}ms", 
                                        nonblocking_tx.hash, state_changes.len(), eth_total, sim_time)?;
                                }
                            }
                        }
                        
                        // Simulation error
                        Ok(Err(e)) => {
                            simulation_failures += 1;
                            let error_type = e.to_string().split(':').next().unwrap_or("unknown").to_string();
                            *error_counts.entry(error_type).or_insert(0) += 1;
                        }
                        
                        // Simulation timeout
                        Err(_) => {
                            simulation_timeouts += 1;
                            *error_counts.entry("timeout".to_string()).or_insert(0) += 1;
                        }
                    }
                    
                    // Progress report every 1000 transactions
                    if total_processed % 1000 == 0 {
                        let success_rate = (simulation_successes as f64 / total_processed as f64) * 100.0;
                        let avg_sim_time = if !simulation_times.is_empty() {
                            simulation_times.iter().sum::<f64>() / simulation_times.len() as f64
                        } else { 0.0 };
                        
                        info!("📊 Processed: {}/20K | Success: {:.1}% | Avg sim: {:.2}ms", 
                            total_processed, success_rate, avg_sim_time);
                        
                        writeln!(output_file, "\n=== PROGRESS REPORT {} ===", total_processed)?;
                        writeln!(output_file, "Success rate: {:.1}%", success_rate)?;
                        writeln!(output_file, "Average simulation time: {:.2}ms", avg_sim_time)?;
                        writeln!(output_file, "Transactions with state changes: {}", transactions_with_state_changes)?;
                        writeln!(output_file, "Average addresses per transaction: {:.1}", 
                            if transactions_with_state_changes > 0 { 
                                total_addresses_affected as f64 / transactions_with_state_changes as f64 
                            } else { 0.0 })?;
                    }
                }
            }
            Err(e) => {
                error!("Mempool fetch error: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    // Final statistics
    let success_rate = (simulation_successes as f64 / total_processed as f64) * 100.0;
    let avg_sim_time = if !simulation_times.is_empty() {
        simulation_times.iter().sum::<f64>() / simulation_times.len() as f64
    } else { 0.0 };
    let max_sim_time = simulation_times.iter().cloned().fold(0.0f64, f64::max);
    
    writeln!(output_file, "\n\nFINAL RESULTS")?;
    writeln!(output_file, "=============")?;
    writeln!(output_file, "Total processed: {}", total_processed)?;
    writeln!(output_file, "Simulation successes: {} ({:.1}%)", simulation_successes, success_rate)?;
    writeln!(output_file, "Simulation failures: {}", simulation_failures)?;
    writeln!(output_file, "Simulation timeouts: {}", simulation_timeouts)?;
    writeln!(output_file, "Transactions with state changes: {} ({:.1}%)", 
        transactions_with_state_changes, 
        (transactions_with_state_changes as f64 / simulation_successes as f64) * 100.0)?;
    writeln!(output_file, "Total addresses affected: {}", total_addresses_affected)?;
    writeln!(output_file, "Average simulation time: {:.2}ms", avg_sim_time)?;
    writeln!(output_file, "Maximum simulation time: {:.2}ms", max_sim_time)?;
    
    if !error_counts.is_empty() {
        writeln!(output_file, "\nERROR BREAKDOWN:")?;
        for (error_type, count) in error_counts {
            writeln!(output_file, "  {}: {} times", error_type, count)?;
        }
    }
    
    info!("✅ Test completed!");
    info!("📊 Success rate: {:.1}%", success_rate);
    info!("⏱️  Average simulation time: {:.2}ms", avg_sim_time);
    info!("📁 Results saved to: {}", output_filename);
    
    println!("\nResults file: {}", output_filename);
    
    Ok(())
}