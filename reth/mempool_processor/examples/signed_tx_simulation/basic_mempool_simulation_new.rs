/// Basic Mempool Transaction Simulation with New reth_tx_simulator
/// 
/// Demonstrates how to fetch transactions from mempool and simulate them
/// using the new reth_tx_simulator for ultra-fast performance.

use mempool_processor::mempool_fetcher::{
    full_transaction_ipc_client::FullTransactionIpcClient,
    FullTransaction,
};
use reth_tx_simulator::RethDirectTxSimulator;
use reth_primitives::TransactionSigned;
use eyre::Result;
use tracing::{info, error, warn};
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use hex;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n🚀 Mempool Transaction Simulation with New reth_tx_simulator");
    println!("==========================================================\n");

    // Initialize new Direct Reth simulator
    let start = Instant::now();
    let simulator = RethDirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct Reth simulator initialized in {:?}", start.elapsed());

    // Connect to mempool
    info!("📡 Connecting to mempool via IPC...");
    let mempool_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    mempool_client.start_monitoring().await?;
    info!("✅ Mempool monitoring started\n");

    // Create log file
    std::fs::create_dir_all("/home/nima/code/crypto/logs/mempool")?;
    let log_path = format!("/home/nima/code/crypto/logs/mempool/new_sim_{}.log", 
        Local::now().format("%Y%m%d_%H%M%S"));
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    
    writeln!(log_file, "Mempool Simulation with New reth_tx_simulator")?;
    writeln!(log_file, "Started: {}", Local::now())?;
    writeln!(log_file, "============================================\n")?;

    // Process transactions
    let target_txs = 10;
    let mut processed = 0;
    let mut successful = 0;
    let mut failed = 0;
    let mut sim_times = Vec::new();
    let mut state_extraction_times = Vec::new();

    info!("📊 Processing {} transactions...\n", target_txs);

    while processed < target_txs {
        let transactions = mempool_client.get_full_transactions(5).await?;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        for tx in transactions {
            processed += 1;
            
            println!("Transaction {}/{}: {}", processed, target_txs, tx.hash);
            
            // Convert IPC transaction directly to TransactionSigned - NO RPC!
            let signed_tx = match convert_ipc_to_signed_tx(&tx) {
                Ok(tx) => tx,
                Err(e) => {
                    warn!("Failed to convert IPC tx: {}", e);
                    failed += 1;
                    writeln!(log_file, "[{}] ERROR: Failed to convert IPC tx for {}", 
                        Local::now().format("%H:%M:%S"), tx.hash)?;
                    continue;
                }
            };

            // Get latest block for simulation
            let latest_block = simulator.get_latest_block()?;

            // 1. Basic simulation
            let sim_start = Instant::now();
            match simulator.simulate_signed_transaction_at_block(&signed_tx, latest_block).await {
                Ok(result) => {
                    let sim_time = sim_start.elapsed();
                    sim_times.push(sim_time);
                    successful += 1;

                    println!("  ✅ Basic simulation in {:?}", sim_time);
                    println!("     Gas: {}, Success: {}", result.gas_used, result.success);

                    // Log details
                    writeln!(log_file, "[{}] SUCCESS", Local::now().format("%H:%M:%S"))?;
                    writeln!(log_file, "  Hash: {}", tx.hash)?;
                    writeln!(log_file, "  Detection latency: {} µs", tx.latency_ns / 1000)?;
                    writeln!(log_file, "  Simulation time: {:?}", sim_time)?;
                    writeln!(log_file, "  Gas used: {}", result.gas_used)?;
                    writeln!(log_file, "  Success: {}", result.success)?;
                    if let Some(reason) = result.revert_reason {
                        writeln!(log_file, "  Revert reason: {}", reason)?;
                    }
                }
                Err(e) => {
                    failed += 1;
                    error!("  ❌ Simulation failed: {}", e);
                    writeln!(log_file, "[{}] FAILED: {}", 
                        Local::now().format("%H:%M:%S"), tx.hash)?;
                    writeln!(log_file, "  Error: {}\n", e)?;
                    continue;
                }
            }

            // 2. Simulate with state changes (for first 3 transactions)
            if processed <= 3 {
                let state_start = Instant::now();
                match simulator.simulate_signed_transaction_with_state_changes_at_block(&signed_tx, latest_block).await {
                    Ok(state_changes) => {
                        let state_time = state_start.elapsed();
                        state_extraction_times.push(state_time);
                        
                        println!("  📊 State changes extracted in {:?}", state_time);
                        
                        if let Some(obj) = state_changes.as_object() {
                            println!("     {} addresses affected", obj.len());
                            writeln!(log_file, "  State extraction time: {:?}", state_time)?;
                            writeln!(log_file, "  Addresses affected: {}", obj.len())?;
                            
                            // Log first few state changes
                            for (i, (addr, changes)) in obj.iter().enumerate().take(3) {
                                writeln!(log_file, "    Address {}: {}", i + 1, addr)?;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("  ⚠️  State extraction failed: {}", e);
                    }
                }
            }

            writeln!(log_file)?;

            if processed >= target_txs {
                break;
            }
        }
    }

    // Summary
    println!("\n📊 SUMMARY");
    println!("==========");
    println!("Total processed: {}", processed);
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);

    if !sim_times.is_empty() {
        sim_times.sort();
        let avg = sim_times.iter().sum::<Duration>() / sim_times.len() as u32;
        let min = sim_times.first().unwrap();
        let max = sim_times.last().unwrap();
        
        println!("\nBasic simulation times:");
        println!("  Average: {:?}", avg);
        println!("  Min: {:?}", min);
        println!("  Max: {:?}", max);
        println!("  Throughput: {:.0} tx/sec", 1_000_000.0 / avg.as_micros() as f64);

        writeln!(log_file, "\nSUMMARY")?;
        writeln!(log_file, "Total: {}, Success: {}, Failed: {}", processed, successful, failed)?;
        writeln!(log_file, "Basic simulation avg: {:?}", avg)?;
        writeln!(log_file, "Throughput: {:.0} tx/sec", 1_000_000.0 / avg.as_micros() as f64)?;
    }

    if !state_extraction_times.is_empty() {
        let avg_state = state_extraction_times.iter().sum::<Duration>() / state_extraction_times.len() as u32;
        println!("\nState extraction times:");
        println!("  Average: {:?}", avg_state);
        writeln!(log_file, "State extraction avg: {:?}", avg_state)?;
    }

    println!("\n✅ Complete! Log saved to: {}", log_path);
    Ok(())
}

/// Convert IPC transaction data directly to TransactionSigned - NO RPC NEEDED!
fn convert_ipc_to_signed_tx(tx: &FullTransaction) -> Result<TransactionSigned> {
    use alloy_consensus::{TxLegacy, TxEip1559};
    use alloy_primitives::{TxKind, U256, Bytes};
    use reth_primitives::Transaction;
    use reth_ethereum::primitives::transaction::signature::Signature;
    
    let tx_data = &tx.tx_data;
    
    // Get transaction type
    let tx_type = tx_data["type"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    
    // Parse common fields
    let chain_id = tx_data["chainId"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .unwrap_or(1);
    
    let nonce = tx_data["nonce"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    
    let gas_limit = tx_data["gas"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .unwrap_or(21000);
    
    let to = tx_data["to"].as_str()
        .filter(|s| !s.is_empty() && *s != "null")
        .and_then(|s| s.parse::<alloy_primitives::Address>().ok());
    
    let value = tx_data["value"].as_str()
        .and_then(|s| U256::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok())
        .unwrap_or_default();
    
    let input = tx_data["input"].as_str()
        .and_then(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
        .unwrap_or_default();
    
    // Extract signature components
    let r = tx_data["r"].as_str()
        .and_then(|s| U256::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok())
        .ok_or_else(|| eyre::eyre!("Missing r"))?;
    
    let s = tx_data["s"].as_str()
        .and_then(|s| U256::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok())
        .ok_or_else(|| eyre::eyre!("Missing s"))?;
    
    let v = tx_data["v"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .ok_or_else(|| eyre::eyre!("Missing v"))?;
    
    // Build transaction based on type
    let transaction = match tx_type {
        0 => {
            // Legacy transaction
            let gas_price = tx_data["gasPrice"].as_str()
                .and_then(|s| s.strip_prefix("0x"))
                .and_then(|s| u128::from_str_radix(s, 16).ok())
                .unwrap_or(0);
            
            Transaction::Legacy(TxLegacy {
                chain_id: if v >= 37 { Some((v - 35) / 2) } else { None },
                nonce,
                gas_price,
                gas_limit,
                to: if let Some(addr) = to {
                    TxKind::Call(addr)
                } else {
                    TxKind::Create
                },
                value,
                input: Bytes::from(input.clone()),
            })
        }
        2 => {
            // EIP-1559 transaction
            let max_fee_per_gas = tx_data["maxFeePerGas"].as_str()
                .and_then(|s| s.strip_prefix("0x"))
                .and_then(|s| u128::from_str_radix(s, 16).ok())
                .unwrap_or(0);
            
            let max_priority_fee_per_gas = tx_data["maxPriorityFeePerGas"].as_str()
                .and_then(|s| s.strip_prefix("0x"))
                .and_then(|s| u128::from_str_radix(s, 16).ok())
                .unwrap_or(0);
            
            Transaction::Eip1559(TxEip1559 {
                chain_id,
                nonce,
                gas_limit,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                to: if let Some(addr) = to {
                    TxKind::Call(addr)
                } else {
                    TxKind::Create
                },
                value,
                input: Bytes::from(input.clone()),
                access_list: Default::default(),
            })
        }
        _ => return Err(eyre::eyre!("Unsupported transaction type: {}", tx_type))
    };
    
    // Create signature
    let odd_y_parity = if tx_type == 0 {
        // For legacy transactions, extract parity from v
        if v >= 37 {
            // EIP-155 transaction
            (v - 35) % 2 == 1
        } else {
            // Pre-EIP-155 transaction
            v == 28
        }
    } else {
        // For typed transactions, v is the y_parity (0 or 1)
        v == 1
    };
    
    let signature = Signature::new(r, s, odd_y_parity);
    
    // Create TransactionSigned
    Ok(TransactionSigned::new_unhashed(transaction, signature))
}