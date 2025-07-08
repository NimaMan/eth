/// Test simulation with NonBlockingIpcClient (same as signal detector)
/// Simulates 1K transactions without signal detection, logging detailed state changes

use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, NonBlockingTransaction, TransactionView, FullTransaction};
use mempool_processor::tx_simulator::DirectTxSimulator;
use mempool_processor::common::address::alloy_address_to_checksum;
use tokio::time::Duration;
use tracing::{info, warn};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use eyre::Result;
use ethers::types::{H256, Address, U256};
use hex;

/// Convert NonBlockingTransaction to TransactionView (same as signal detector)
fn convert_nonblocking_to_transaction_view(tx: &NonBlockingTransaction) -> Result<TransactionView> {
    // Parse transaction data from JSON
    let hash = tx.data["hash"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing hash"))?
        .parse::<H256>()?;
    
    let from = tx.data["from"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing from"))?
        .parse::<Address>()?;
    
    let to = tx.data["to"].as_str()
        .and_then(|s| s.parse::<Address>().ok());
    
    let value = tx.data["value"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing value"))?
        .parse::<U256>()?;
    
    let gas_price = tx.data["gasPrice"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing gasPrice"))?
        .parse::<U256>()?;
    
    let gas_limit = tx.data["gas"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing gas"))?
        .parse::<U256>()?;
    
    let nonce = tx.data["nonce"].as_str()
        .ok_or_else(|| eyre::eyre!("Missing nonce"))?
        .parse::<U256>()?;
    
    let input_data = if let Some(input_str) = tx.data["input"].as_str() {
        let hex_str = input_str.strip_prefix("0x").unwrap_or(input_str);
        Some(hex::decode(hex_str)?)
    } else {
        None
    };
    
    Ok(TransactionView {
        hash: hash.as_bytes().to_vec(),
        from: from.as_bytes().to_vec(),
        to: to.map(|addr| addr.as_bytes().to_vec()),
        value,
        gas_price: Some(gas_price),
        gas_limit: Some(gas_limit),
        nonce: Some(nonce),
        input_data,
    })
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🚀 Starting NonBlockingIpcClient simulation test (1K transactions)");
    
    // Initialize Direct simulator with automatic nonce correction
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct simulator initialized with nonce correction");
    
    // Initialize NonBlockingIpcClient (same as signal detector)
    let mempool_client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    mempool_client.start().await?;
    info!("✅ NonBlockingIpcClient connected to IPC and started");
    
    // Create output file
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let output_filename = format!("/home/nima/code/crypto/logs/mempool/nonblocking_sim_1k_{}.log", timestamp);
    let mut output_file = File::create(&output_filename)?;
    
    writeln!(output_file, "NONBLOCKING IPC SIMULATION TEST - 1K TRANSACTIONS")?;
    writeln!(output_file, "=================================================")?;
    writeln!(output_file, "Timestamp: {}", chrono::Utc::now())?;
    writeln!(output_file, "Using same fetcher as signal detector\n")?;
    
    // Performance tracking
    let mut total_processed = 0u64;
    let mut simulation_successes = 0u64;
    let mut simulation_failures = 0u64;
    let mut transactions_with_state_changes = 0u64;
    let mut total_fetch_time_ms = 0.0;
    let mut total_sim_time_ms = 0.0;
    
    // Track error types
    let mut error_counts = HashMap::<String, u64>::new();
    
    // Timing buffers (same as signal detector)
    let mut ipc_detection_times = Vec::with_capacity(1000);
    let mut simulation_times = Vec::with_capacity(1000);
    
    info!("📡 Starting to fetch mempool transactions...");
    
    let start_time = Instant::now();
    
    while total_processed < 1_000 {
        // Fetch batch of transactions (same as signal detector)
        let fetch_start = Instant::now();
        let transactions = mempool_client.get_transactions(10).await?;
        let fetch_time = fetch_start.elapsed();
        total_fetch_time_ms += fetch_time.as_secs_f64() * 1000.0;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        
        info!("✅ Fetched {} transactions in {:?}", transactions.len(), fetch_time);
        
        for tx in transactions {
            if total_processed >= 1_000 {
                break;
            }
            
            total_processed += 1;
            
            // Track IPC detection time
            let detection_latency_us = tx.detection_ns / 1000;
            ipc_detection_times.push(detection_latency_us);
            
            // Log transaction start
            writeln!(output_file, "\n=== TRANSACTION {} ===", total_processed)?;
            writeln!(output_file, "Hash: {}", tx.hash)?;
            writeln!(output_file, "Detection latency: {} µs", detection_latency_us)?;
            
            // Convert NonBlockingTransaction to TransactionView (same as signal detector)
            let tx_view = match convert_nonblocking_to_transaction_view(&tx) {
                Ok(view) => view,
                Err(e) => {
                    warn!("Failed to convert transaction: {}", e);
                    simulation_failures += 1;
                    writeln!(output_file, "❌ CONVERSION FAILED: {}", e)?;
                    continue;
                }
            };
            
            // Log transaction details
            writeln!(output_file, "From: 0x{}", hex::encode(&tx_view.from))?;
            writeln!(output_file, "To: {}", tx_view.to.as_ref()
                .map(|a| format!("0x{}", hex::encode(a)))
                .unwrap_or_else(|| "Contract Creation".to_string()))?;
            writeln!(output_file, "Value: {:.6} ETH", 
                tx_view.value.to_string().parse::<u128>().unwrap_or(0) as f64 / 1e18)?;
            
            // Convert to FullTransaction for simulation
            let full_tx = FullTransaction {
                hash: tx.hash.clone(),
                tx_data: tx.data.clone(),
                detection_time: Instant::now(),
                latency_ns: tx.detection_ns,
            };
            
            // Simulate with call trace
            let sim_start = Instant::now();
            match simulator.simulate_with_call_trace(&full_tx).await {
                Ok(result) => {
                    let sim_time = sim_start.elapsed();
                    let sim_time_ms = sim_time.as_secs_f64() * 1000.0;
                    simulation_times.push((sim_time.as_micros() as u64, 0)); // (time, 0 for no events)
                    total_sim_time_ms += sim_time_ms;
                    simulation_successes += 1;
                    
                    writeln!(output_file, "✅ SIMULATION SUCCESS")?;
                    writeln!(output_file, "Simulation time: {:.2}ms", sim_time_ms)?;
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
                        
                        // Transaction summary
                        let total_eth_moved: f64 = result.detailed_changes.values()
                            .map(|c| c.eth_net.abs())
                            .sum();
                        let total_tokens_moved: usize = result.detailed_changes.values()
                            .map(|c| c.token_net.len())
                            .sum();
                            
                        writeln!(output_file, "\nTRANSACTION SUMMARY:")?;
                        writeln!(output_file, "  Total ETH moved: {:.6} ETH", total_eth_moved)?;
                        writeln!(output_file, "  Total token movements: {}", total_tokens_moved)?;
                    } else {
                        writeln!(output_file, "\nNO STATE CHANGES DETECTED")?;
                    }
                }
                Err(e) => {
                    simulation_failures += 1;
                    simulation_times.push((sim_start.elapsed().as_micros() as u64, 0));
                    
                    let error_type = e.to_string().split(':').next().unwrap_or("unknown").to_string();
                    *error_counts.entry(error_type.clone()).or_insert(0) += 1;
                    
                    writeln!(output_file, "❌ SIMULATION FAILED")?;
                    writeln!(output_file, "Error: {}", e)?;
                }
            }
            
            // Progress report every 100 transactions (same as signal detector)
            if total_processed % 100 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let tx_per_sec = total_processed as f64 / elapsed;
                let success_rate = (simulation_successes as f64 / total_processed as f64) * 100.0;
                let state_change_rate = if simulation_successes > 0 {
                    (transactions_with_state_changes as f64 / simulation_successes as f64) * 100.0
                } else { 0.0 };
                
                info!("📊 Progress: {}/1K | {:.1} tx/s | Success: {:.1}% | With changes: {:.1}%", 
                    total_processed, tx_per_sec, success_rate, state_change_rate);
                
                writeln!(output_file, "\n=== CHECKPOINT {} ===", total_processed)?;
                writeln!(output_file, "Throughput: {:.1} tx/sec", tx_per_sec)?;
                writeln!(output_file, "Success rate: {:.1}%", success_rate)?;
                writeln!(output_file, "State changes: {:.1}%", state_change_rate)?;
                
                // Calculate timing stats (same as signal detector)
                if !ipc_detection_times.is_empty() && !simulation_times.is_empty() {
                    let avg_detection = ipc_detection_times.iter().sum::<u64>() as f64 / ipc_detection_times.len() as f64;
                    let avg_simulation = simulation_times.iter().map(|(t, _)| t).sum::<u64>() as f64 / simulation_times.len() as f64;
                    
                    writeln!(output_file, "Avg IPC detection: {:.0} µs", avg_detection)?;
                    writeln!(output_file, "Avg simulation: {:.0} µs", avg_simulation)?;
                }
            }
        }
    }
    
    let total_elapsed = start_time.elapsed();
    
    // Final statistics
    let success_rate = (simulation_successes as f64 / total_processed as f64) * 100.0;
    let state_change_rate = if simulation_successes > 0 {
        (transactions_with_state_changes as f64 / simulation_successes as f64) * 100.0
    } else { 0.0 };
    
    writeln!(output_file, "\n\n=== FINAL RESULTS ===")?;
    writeln!(output_file, "Total processed: {}", total_processed)?;
    writeln!(output_file, "Total time: {:.2}s", total_elapsed.as_secs_f64())?;
    writeln!(output_file, "Overall throughput: {:.1} tx/sec", total_processed as f64 / total_elapsed.as_secs_f64())?;
    writeln!(output_file, "Simulation successes: {} ({:.1}%)", simulation_successes, success_rate)?;
    writeln!(output_file, "Simulation failures: {}", simulation_failures)?;
    writeln!(output_file, "Transactions with state changes: {} ({:.1}%)", 
        transactions_with_state_changes, state_change_rate)?;
    
    // Timing analysis
    if !ipc_detection_times.is_empty() && !simulation_times.is_empty() {
        ipc_detection_times.sort();
        simulation_times.sort_by_key(|(t, _)| *t);
        
        let detection_avg = ipc_detection_times.iter().sum::<u64>() as f64 / ipc_detection_times.len() as f64;
        let detection_min = ipc_detection_times[0];
        let detection_max = ipc_detection_times[ipc_detection_times.len() - 1];
        let detection_p50 = ipc_detection_times[ipc_detection_times.len() / 2];
        let detection_p99 = ipc_detection_times[ipc_detection_times.len() * 99 / 100];
        
        let sim_times_only: Vec<u64> = simulation_times.iter().map(|(t, _)| *t).collect();
        let sim_avg = sim_times_only.iter().sum::<u64>() as f64 / sim_times_only.len() as f64;
        let sim_min = sim_times_only[0];
        let sim_max = sim_times_only[sim_times_only.len() - 1];
        let sim_p50 = sim_times_only[sim_times_only.len() / 2];
        let sim_p99 = sim_times_only[sim_times_only.len() * 99 / 100];
        
        writeln!(output_file, "\n=== TIMING ANALYSIS ===")?;
        writeln!(output_file, "IPC Detection Latency:")?;
        writeln!(output_file, "  Average: {:.0} µs", detection_avg)?;
        writeln!(output_file, "  Min: {} µs", detection_min)?;
        writeln!(output_file, "  Max: {} µs", detection_max)?;
        writeln!(output_file, "  P50: {} µs", detection_p50)?;
        writeln!(output_file, "  P99: {} µs", detection_p99)?;
        
        writeln!(output_file, "\nSimulation Time:")?;
        writeln!(output_file, "  Average: {:.0} µs ({:.2} ms)", sim_avg, sim_avg / 1000.0)?;
        writeln!(output_file, "  Min: {} µs ({:.2} ms)", sim_min, sim_min as f64 / 1000.0)?;
        writeln!(output_file, "  Max: {} µs ({:.2} ms)", sim_max, sim_max as f64 / 1000.0)?;
        writeln!(output_file, "  P50: {} µs ({:.2} ms)", sim_p50, sim_p50 as f64 / 1000.0)?;
        writeln!(output_file, "  P99: {} µs ({:.2} ms)", sim_p99, sim_p99 as f64 / 1000.0)?;
        
        writeln!(output_file, "\nAverage Component Times:")?;
        writeln!(output_file, "  Fetch: {:.2} ms/batch", total_fetch_time_ms / (total_processed as f64 / 10.0))?;
        writeln!(output_file, "  Simulation: {:.2} ms/tx", total_sim_time_ms / total_processed as f64)?;
    }
    
    if !error_counts.is_empty() {
        writeln!(output_file, "\n=== ERROR BREAKDOWN ===")?;
        for (error_type, count) in error_counts {
            writeln!(output_file, "  {}: {} times ({:.1}%)", 
                error_type, count, (count as f64 / total_processed as f64) * 100.0)?;
        }
    }
    
    info!("✅ Test completed!");
    info!("📊 Success rate: {:.1}%", success_rate);
    info!("📊 State change rate: {:.1}%", state_change_rate);
    info!("📊 Overall throughput: {:.1} tx/sec", total_processed as f64 / total_elapsed.as_secs_f64());
    info!("📁 Results saved to: {}", output_filename);
    
    println!("\nDetailed results file: {}", output_filename);
    
    Ok(())
}