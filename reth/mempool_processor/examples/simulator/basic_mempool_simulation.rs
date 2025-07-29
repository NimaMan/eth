/// Basic Mempool Transaction Simulation
/// 
/// Demonstrates how to fetch transactions from mempool and simulate them
/// using Direct Reth for ultra-fast performance (20-40x faster than RPC).

use mempool_processor::mempool_fetcher::{
    full_transaction_ipc_client::FullTransactionIpcClient,
};
use reth_tx_simulator::RethTxSimulator;
use reth_primitives::TransactionSigned;
use alloy_rlp::Decodable;
use eyre::Result;
use tracing::{info, warn};
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use hex;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n🚀 Basic Mempool Transaction Simulation");
    println!("======================================\n");

    // Initialize Direct Reth simulator
    let start = Instant::now();
    let simulator = RethTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct Reth simulator initialized in {:?}", start.elapsed());

    // Connect to mempool
    info!("📡 Connecting to mempool via IPC...");
    let mempool_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    mempool_client.start_monitoring().await?;
    info!("✅ Mempool monitoring started\n");

    // Create log file
    std::fs::create_dir_all("/home/nima/code/crypto/logs/mempool/dev")?;
    let log_path = format!("/home/nima/code/crypto/logs/mempool/dev/basic_sim_1k_{}.log", 
        Local::now().format("%Y%m%d_%H%M%S"));
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    
    writeln!(log_file, "Basic Mempool Simulation Log")?;
    writeln!(log_file, "Started: {}", Local::now())?;
    writeln!(log_file, "============================\n")?;

    // Process transactions
    let target_txs = 1000;
    let mut processed = 0;
    let mut successful = 0;
    let mut failed = 0;
    let mut sim_times = Vec::new();

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
            
            

            // Get raw transaction first
            let raw_tx = match get_raw_tx(&tx.hash).await {
                Ok(raw) => raw,
                Err(e) => {
                    warn!("Failed to get raw transaction: {}", e);
                    failed += 1;
                    writeln!(log_file, "[{}] ERROR: Failed to get raw tx for {}", 
                        Local::now().format("%H:%M:%S"), tx.hash)?;
                    continue;
                }
            };
            
            // Decode the transaction
            let hex_str = raw_tx.strip_prefix("0x").unwrap_or(&raw_tx);
            let raw_bytes = match hex::decode(hex_str) {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!("Failed to decode hex: {}", e);
                    failed += 1;
                    continue;
                }
            };
            
            let signed_tx = match TransactionSigned::decode(&mut raw_bytes.as_slice()) {
                Ok(tx) => tx,
                Err(e) => {
                    warn!("Failed to decode transaction: {}", e);
                    failed += 1;
                    continue;
                }
            };
            
            // Simulate with timeout (50ms for fast timeout)
            let sim_start = Instant::now();
            let result = timeout(
                Duration::from_millis(50),
                simulator.simulate_signed_transaction(&signed_tx)
            ).await;
            
            match result {
                Ok(Ok(sim_result)) => {
                    let sim_time = sim_start.elapsed();
                    sim_times.push(sim_time);
                    successful += 1;

                    println!("  ✅ Simulated in {:?}", sim_time);
                    println!("     Gas used: {}", sim_result.gas_used);
                    println!("     Success: {}", sim_result.success);

                    // Log details
                    writeln!(log_file, "[{}] SUCCESS", Local::now().format("%H:%M:%S"))?;
                    writeln!(log_file, "  Hash: {}", tx.hash)?;
                    writeln!(log_file, "  Detection latency: {} µs", tx.latency_ns / 1000)?;
                    writeln!(log_file, "  Simulation time: {:?}", sim_time)?;
                    writeln!(log_file, "  Gas used: {}", sim_result.gas_used)?;
                    writeln!(log_file, "  Success: {}", sim_result.success)?;
                    if let Some(reason) = &sim_result.revert_reason {
                        writeln!(log_file, "  Revert reason: {}", reason)?;
                    }
                    writeln!(log_file)?;
                }
                Ok(Err(e)) => {
                    failed += 1;
                    warn!("Simulation error: {}", e);
                    writeln!(log_file, "[{}] FAILED: {}", Local::now().format("%H:%M:%S"), e)?;
                }
                Err(_) => {
                    failed += 1;
                    warn!("Simulation timed out after 50ms");
                    writeln!(log_file, "[{}] TIMEOUT after 50ms", Local::now().format("%H:%M:%S"))?;
                }
            }

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
        
        println!("\nSimulation times:");
        println!("  Average: {:?}", avg);
        println!("  Min: {:?}", min);
        println!("  Max: {:?}", max);
        println!("  Throughput: {:.0} tx/sec", 1_000_000.0 / avg.as_micros() as f64);

        writeln!(log_file, "\nSUMMARY")?;
        writeln!(log_file, "Total: {}, Success: {}, Failed: {}", processed, successful, failed)?;
        writeln!(log_file, "Avg simulation time: {:?}", avg)?;
        writeln!(log_file, "Throughput: {:.0} tx/sec", 1_000_000.0 / avg.as_micros() as f64)?;
    }

    println!("\n✅ Complete! Log saved to: {}", log_path);
    Ok(())
}

// Helper function to get raw transaction via RPC
async fn get_raw_tx(hash: &str) -> Result<String> {
    use jsonrpsee::http_client::HttpClientBuilder;
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::rpc_params;
    
    let client = HttpClientBuilder::default()
        .build("http://127.0.0.1:8545")?;
    
    let raw_tx: String = client
        .request("eth_getRawTransactionByHash", rpc_params![hash])
        .await?;
    
    Ok(raw_tx)
}

