/// Test simulation with detailed address state changes for 1K transactions
/// Logs every transaction hash with its complete address state changes

use mempool_processor::tx_simulator::DirectTxSimulator;
use mempool_processor::mempool_fetcher::full_transaction_ipc_client::FullTransactionIpcClient;
use mempool_processor::common::address::alloy_address_to_checksum;
use tokio::time::Duration;
use tracing::info;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Starting detailed simulation test with 1K transactions");
    
    // Initialize Direct simulator with automatic nonce correction
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct simulator initialized with nonce correction");
    
    // Initialize mempool fetcher using FullTransactionIpcClient
    let fetcher = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    fetcher.start_monitoring().await?;
    info!("✅ Mempool monitoring started");
    
    // Create output file
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let output_filename = format!("/home/nima/code/crypto/logs/mempool/detailed_state_changes_1k_{}.log", timestamp);
    let mut output_file = File::create(&output_filename)?;
    
    writeln!(output_file, "DETAILED SIMULATION STATE CHANGES - 1K TRANSACTIONS")?;
    writeln!(output_file, "===================================================")?;
    writeln!(output_file, "Timestamp: {}", chrono::Utc::now())?;
    writeln!(output_file, "Target: 1,000 transactions\n")?;
    
    // Performance tracking
    let mut total_processed = 0u64;
    let mut simulation_successes = 0u64;
    let mut simulation_failures = 0u64;
    let mut transactions_with_state_changes = 0u64;
    
    // Track error types
    let mut error_counts = HashMap::<String, u64>::new();
    
    info!("📡 Starting to fetch mempool transactions...");
    
    while total_processed < 1_000 {
        // Fetch transactions in batches
        let transactions = fetcher.get_full_transactions(10).await?;
        
        if transactions.is_empty() {
            info!("⏳ No transactions available, waiting 100ms...");
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        
        info!("✅ Got {} transactions", transactions.len());
        writeln!(output_file, "\nProcessing batch of {} transactions", transactions.len())?;
        
        for tx in transactions {
            if total_processed >= 1_000 {
                break;
            }
            
            total_processed += 1;
            
            // Log transaction start
            writeln!(output_file, "\n=== TRANSACTION {} ===", total_processed)?;
            writeln!(output_file, "Hash: {}", tx.hash)?;
            writeln!(output_file, "Detection latency: {} µs", tx.latency_ns / 1000)?;
            
            // Simulate with call trace (automatic nonce correction in DirectTxSimulator)
            let start_time = Instant::now();
            match simulator.simulate_with_call_trace(&tx).await {
                Ok(result) => {
                    let sim_time = start_time.elapsed().as_secs_f64() * 1000.0;
                    simulation_successes += 1;
                    
                    writeln!(output_file, "✅ SIMULATION SUCCESS")?;
                    writeln!(output_file, "Simulation time: {:.2}ms", sim_time)?;
                    writeln!(output_file, "Block: {}", result.block_number)?;
                    writeln!(output_file, "Addresses affected: {}", result.detailed_changes.len())?;
                    
                    if !result.detailed_changes.is_empty() {
                        transactions_with_state_changes += 1;
                        
                        writeln!(output_file, "\nADDRESS STATE CHANGES:")?;
                        for (address, changes) in &result.detailed_changes {
                            let checksum_addr = alloy_address_to_checksum(*address);
                            writeln!(output_file, "\n  Address: {}", checksum_addr)?;
                            writeln!(output_file, "    ETH net change: {:.12} ETH", changes.eth_net)?;
                            
                            if !changes.token_net.is_empty() {
                                writeln!(output_file, "    Token changes: {} tokens", changes.token_net.len())?;
                                for (token_addr, amount) in &changes.token_net {
                                    writeln!(output_file, "      {}: {}", token_addr, amount)?;
                                }
                            } else {
                                writeln!(output_file, "    Token changes: None")?;
                            }
                        }
                        
                        // Summary for this transaction
                        let total_eth_moved: f64 = result.detailed_changes.values()
                            .map(|c| c.eth_net.abs())
                            .sum();
                        let total_tokens_moved: usize = result.detailed_changes.values()
                            .map(|c| c.token_net.len())
                            .sum();
                            
                        writeln!(output_file, "\nTRANSACTION SUMMARY:")?;
                        writeln!(output_file, "  Total ETH moved: {:.6} ETH", total_eth_moved)?;
                        writeln!(output_file, "  Total token movements: {}", total_tokens_moved)?;
                        
                        if total_eth_moved > 0.1 {
                            writeln!(output_file, "  🔥 SIGNIFICANT ETH MOVEMENT (>{} ETH)", 0.1)?;
                        }
                    } else {
                        writeln!(output_file, "\nNO STATE CHANGES DETECTED")?;
                    }
                }
                Err(e) => {
                    simulation_failures += 1;
                    let error_type = e.to_string().split(':').next().unwrap_or("unknown").to_string();
                    *error_counts.entry(error_type.clone()).or_insert(0) += 1;
                    
                    writeln!(output_file, "❌ SIMULATION FAILED")?;
                    writeln!(output_file, "Error type: {}", error_type)?;
                    writeln!(output_file, "Full error: {}", e)?;
                }
            }
                    
            // Progress report every 100 transactions
            if total_processed % 100 == 0 {
                let success_rate = (simulation_successes as f64 / total_processed as f64) * 100.0;
                let state_change_rate = if simulation_successes > 0 {
                    (transactions_with_state_changes as f64 / simulation_successes as f64) * 100.0
                } else { 0.0 };
                
                info!("📊 Processed: {}/1K | Success: {:.1}% | With state changes: {:.1}%", 
                    total_processed, success_rate, state_change_rate);
                
                writeln!(output_file, "\n=== PROGRESS CHECKPOINT {} ===", total_processed)?;
                writeln!(output_file, "Success rate: {:.1}%", success_rate)?;
                writeln!(output_file, "Transactions with state changes: {:.1}%", state_change_rate)?;
                writeln!(output_file, "Most common errors: {:?}", error_counts)?;
            }
        }
    }
    
    // Final statistics
    let success_rate = (simulation_successes as f64 / total_processed as f64) * 100.0;
    let state_change_rate = if simulation_successes > 0 {
        (transactions_with_state_changes as f64 / simulation_successes as f64) * 100.0
    } else { 0.0 };
    
    writeln!(output_file, "\n\n=== FINAL RESULTS ===")?;
    writeln!(output_file, "Total processed: {}", total_processed)?;
    writeln!(output_file, "Simulation successes: {} ({:.1}%)", simulation_successes, success_rate)?;
    writeln!(output_file, "Simulation failures: {}", simulation_failures)?;
    writeln!(output_file, "Transactions with state changes: {} ({:.1}%)", 
        transactions_with_state_changes, state_change_rate)?;
    
    if !error_counts.is_empty() {
        writeln!(output_file, "\nERROR BREAKDOWN:")?;
        for (error_type, count) in error_counts {
            writeln!(output_file, "  {}: {} times ({:.1}%)", 
                error_type, count, (count as f64 / total_processed as f64) * 100.0)?;
        }
    }
    
    // Verdict
    writeln!(output_file, "\n=== VERDICT ===")?;
    if success_rate > 90.0 {
        writeln!(output_file, "✅ SIMULATION IS WORKING WELL ({:.1}% success rate)", success_rate)?;
    } else if success_rate > 50.0 {
        writeln!(output_file, "⚠️  SIMULATION HAS ISSUES ({:.1}% success rate)", success_rate)?;
    } else {
        writeln!(output_file, "❌ SIMULATION IS BROKEN ({:.1}% success rate)", success_rate)?;
    }
    
    if state_change_rate > 20.0 {
        writeln!(output_file, "✅ GOOD STATE CHANGE DETECTION ({:.1}% of successful simulations)", state_change_rate)?;
    } else {
        writeln!(output_file, "⚠️  LOW STATE CHANGE DETECTION ({:.1}% of successful simulations)", state_change_rate)?;
    }
    
    info!("✅ Test completed!");
    info!("📊 Success rate: {:.1}%", success_rate);
    info!("📊 State change rate: {:.1}%", state_change_rate);
    info!("📁 Results saved to: {}", output_filename);
    
    println!("\nDetailed results file: {}", output_filename);
    
    Ok(())
}