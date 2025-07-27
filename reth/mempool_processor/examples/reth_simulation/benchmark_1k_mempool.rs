/// Benchmark 1000 Mempool Transactions with Direct Reth
/// 
/// This example demonstrates the high-throughput capability of Direct Reth
/// by processing 1000 mempool transactions and measuring performance.

use mempool_processor::mempool_fetcher::{
    full_transaction_ipc_client::FullTransactionIpcClient,
};
use reth_tx_simulator::DirectTxSimulator;
use reth_primitives::TransactionSigned;
use alloy_rlp::Decodable;
use eyre::Result;
use tracing::info;
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n🚀 Benchmark: 1000 Mempool Transactions");
    println!("======================================\n");

    // Initialize
    let start = Instant::now();
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let mempool_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    mempool_client.start_monitoring().await?;
    info!("✅ Initialized in {:?}\n", start.elapsed());

    // Create benchmark log
    std::fs::create_dir_all("/home/nima/code/crypto/logs/mempool")?;
    let log_path = format!("/home/nima/code/crypto/logs/mempool/benchmark_1k_{}.log", 
        Local::now().format("%Y%m%d_%H%M%S"));
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    
    writeln!(log_file, "1000 Transaction Benchmark")?;
    writeln!(log_file, "Started: {}", Local::now())?;
    writeln!(log_file, "========================\n")?;

    // Statistics
    struct Stats {
        detection_times: Vec<u64>,
        simulation_times: Vec<Duration>,
        gas_used: Vec<u64>,
        successful: usize,
        failed: usize,
        reverted: usize,
    }
    
    let mut stats = Stats {
        detection_times: Vec::new(),
        simulation_times: Vec::new(),
        gas_used: Vec::new(),
        successful: 0,
        failed: 0,
        reverted: 0,
    };

    let target = 1000;
    let mut processed = 0;
    let batch_size = 50;
    let overall_start = Instant::now();

    println!("📊 Processing {} transactions in batches of {}...\n", target, batch_size);

    while processed < target {
        let batch_start = Instant::now();
        let transactions = mempool_client.get_full_transactions(batch_size).await?;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(50)).await;
            continue;
        }

        let batch_count = transactions.len();
        
        for tx in transactions {
            processed += 1;
            stats.detection_times.push(tx.latency_ns);

            // Get raw transaction
            let raw_tx = match get_raw_tx(&tx.hash).await {
                Ok(raw) => raw,
                Err(_) => {
                    stats.failed += 1;
                    continue;
                }
            };

            // Decode
            let signed_tx = match decode_transaction(&raw_tx) {
                Ok(tx) => tx,
                Err(_) => {
                    stats.failed += 1;
                    continue;
                }
            };

            // Simulate
            let sim_start = Instant::now();
            match simulator.simulate_signed_transaction(&signed_tx).await {
                Ok(result) => {
                    let sim_time = sim_start.elapsed();
                    stats.simulation_times.push(sim_time);
                    
                    if result.success {
                        stats.successful += 1;
                        stats.gas_used.push(result.gas_used);
                    } else {
                        stats.reverted += 1;
                    }

                    // Log every 100th transaction
                    if processed % 100 == 0 {
                        writeln!(log_file, "[{}] Milestone: {} transactions", 
                            Local::now().format("%H:%M:%S"), processed)?;
                        writeln!(log_file, "  Success: {}, Failed: {}, Reverted: {}", 
                            stats.successful, stats.failed, stats.reverted)?;
                        writeln!(log_file, "  Avg sim time: {:?}", 
                            stats.simulation_times.iter().sum::<Duration>() / stats.simulation_times.len() as u32)?;
                    }
                }
                Err(_) => {
                    stats.failed += 1;
                }
            }
        }

        let batch_time = batch_start.elapsed();
        let batch_throughput = batch_count as f64 / batch_time.as_secs_f64();
        
        println!("Batch {}: {} tx in {:?} ({:.0} tx/sec)", 
            processed / batch_size, batch_count, batch_time, batch_throughput);

        if processed >= target {
            break;
        }
    }

    let overall_time = overall_start.elapsed();

    // Calculate final statistics
    println!("\n📊 BENCHMARK RESULTS");
    println!("===================");
    println!("Total time:      {:?}", overall_time);
    println!("Transactions:    {}", processed);
    println!("Overall rate:    {:.0} tx/sec", processed as f64 / overall_time.as_secs_f64());
    println!("\nBreakdown:");
    println!("  Successful:    {} ({:.1}%)", stats.successful, 
        (stats.successful as f64 / processed as f64) * 100.0);
    println!("  Reverted:      {} ({:.1}%)", stats.reverted,
        (stats.reverted as f64 / processed as f64) * 100.0);
    println!("  Failed:        {} ({:.1}%)", stats.failed,
        (stats.failed as f64 / processed as f64) * 100.0);

    if !stats.simulation_times.is_empty() {
        stats.simulation_times.sort();
        let sum: Duration = stats.simulation_times.iter().sum();
        let avg = sum / stats.simulation_times.len() as u32;
        let p50 = &stats.simulation_times[stats.simulation_times.len() / 2];
        let p95 = &stats.simulation_times[stats.simulation_times.len() * 95 / 100];
        let p99 = &stats.simulation_times[stats.simulation_times.len() * 99 / 100];

        println!("\nSimulation times:");
        println!("  Average:       {:?}", avg);
        println!("  P50:           {:?}", p50);
        println!("  P95:           {:?}", p95);
        println!("  P99:           {:?}", p99);
        println!("  Theoretical:   {:.0} tx/sec", 1_000_000.0 / avg.as_micros() as f64);

        // Detection latency
        let avg_detection = stats.detection_times.iter().sum::<u64>() / stats.detection_times.len() as u64;
        println!("\nDetection latency:");
        println!("  Average:       {} µs", avg_detection / 1000);

        // Gas usage
        if !stats.gas_used.is_empty() {
            let avg_gas = stats.gas_used.iter().sum::<u64>() / stats.gas_used.len() as u64;
            println!("\nGas usage:");
            println!("  Average:       {}", avg_gas);
        }

        // Write summary to log
        writeln!(log_file, "\n=== FINAL RESULTS ===")?;
        writeln!(log_file, "Total transactions: {}", processed)?;
        writeln!(log_file, "Total time: {:?}", overall_time)?;
        writeln!(log_file, "Overall throughput: {:.0} tx/sec", processed as f64 / overall_time.as_secs_f64())?;
        writeln!(log_file, "Average simulation: {:?}", avg)?;
        writeln!(log_file, "Theoretical max: {:.0} tx/sec", 1_000_000.0 / avg.as_micros() as f64)?;
        writeln!(log_file, "Completed: {}", Local::now())?;
    }

    println!("\n✅ Benchmark complete! Log: {}", log_path);
    
    // Compare with RPC baseline
    println!("\n🔄 Performance vs RPC:");
    println!("  RPC typical:   10-20 tx/sec");
    println!("  Direct Reth:   {:.0} tx/sec", processed as f64 / overall_time.as_secs_f64());
    println!("  Speedup:       {:.0}x", 
        (processed as f64 / overall_time.as_secs_f64()) / 15.0);

    Ok(())
}

fn decode_transaction(raw_tx: &str) -> Result<TransactionSigned> {
    let hex_str = raw_tx.strip_prefix("0x").unwrap_or(raw_tx);
    let raw_bytes = hex::decode(hex_str)?;
    Ok(TransactionSigned::decode(&mut raw_bytes.as_slice())?)
}

async fn get_raw_tx(hash: &str) -> Result<String> {
    use jsonrpsee::http_client::{HttpClientBuilder, HttpClient};
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::rpc_params;
    
    let client: HttpClient = HttpClientBuilder::default()
        .build("http://127.0.0.1:8545")?;
    
    let raw_tx: String = client.request(
        "eth_getRawTransactionByHash",
        rpc_params![hash]
    ).await?;
    
    Ok(raw_tx)
}