use chrono::Local;
use eyre::Result;
/// Live Mempool Transaction Simulation
///
/// Fetches live transactions from mempool using MempoolFetcherIPCClient
/// and simulates them with MempoolSimulator for automatic nonce retry.
use mempool_processor::canonical_head_cache::CanonicalHeadCache;
use mempool_processor::mempool_fetcher::MempoolFetcherIPCClient;
use mempool_processor::simulator::MempoolSimulator;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n🚀 Live Mempool Transaction Simulation with Auto-Nonce Retry");
    println!("=============================================================\n");

    // Initialize MempoolSimulator with automatic nonce retry
    let start = Instant::now();
    let head_cache = Arc::new(CanonicalHeadCache::new());
    let mempool_simulator =
        MempoolSimulator::new("/home/nima/.local/share/reth/mainnet", head_cache.clone())?;
    info!("✅ MempoolSimulator initialized in {:?}", start.elapsed());

    // Start canonical head listener so simulations have the latest context
    let header_task =
        head_cache.spawn_head_listener("/home/nima/.local/share/reth/mainnet/reth.ipc");
    head_cache
        .wait_for_latest_header(Duration::from_secs(10))
        .await?;

    // Connect to mempool
    info!("📡 Connecting to mempool via IPC...");
    let mempool_client =
        MempoolFetcherIPCClient::new(Some("/home/nima/.local/share/reth/mainnet/reth.ipc"))?;
    mempool_client.start().await?;
    info!("✅ Mempool monitoring started\n");

    // Create log file
    let log_dir = PathBuf::from(mempool_processor::config::DEFAULT_LOG_DIR).join("simulation");
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join(format!(
        "live_simulation_{}.log",
        Local::now().format("%Y%m%d_%H%M%S")
    ));
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
        let transactions = mempool_client.get_transactions_instant(5).await;

        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        for tx in transactions {
            processed += 1;

            println!("Transaction {}/{}: {}", processed, target_txs, tx.hash);

            // Simulate with MempoolSimulator (includes automatic nonce retry)
            let sim_start = Instant::now();
            let result = timeout(
                Duration::from_millis(50),
                mempool_simulator.simulate_mempool_tx(&tx),
            )
            .await;

            match result {
                Ok(Ok(sim_result)) => {
                    let sim_time = sim_start.elapsed();
                    sim_times.push(sim_time);
                    successful += 1;

                    println!("  ✅ Simulated in {:?} (with auto-nonce retry)", sim_time);
                    println!("     Gas used: {}", sim_result.gas_used);
                    println!("     Success: {}", sim_result.success);

                    // Log details
                    writeln!(log_file, "[{}] SUCCESS", Local::now().format("%H:%M:%S"))?;
                    writeln!(log_file, "  Hash: {}", tx.hash)?;
                    writeln!(
                        log_file,
                        "  Detection latency: {} µs",
                        tx.detection_ns / 1000
                    )?;
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
                    warn!("Simulation error (after nonce retry): {}", e);
                    writeln!(
                        log_file,
                        "[{}] FAILED: {}",
                        Local::now().format("%H:%M:%S"),
                        e
                    )?;
                }
                Err(_) => {
                    failed += 1;
                    warn!("Simulation timed out after 50ms");
                    writeln!(
                        log_file,
                        "[{}] TIMEOUT after 50ms",
                        Local::now().format("%H:%M:%S")
                    )?;
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
        println!(
            "  Throughput: {:.0} tx/sec",
            1_000_000.0 / avg.as_micros() as f64
        );

        writeln!(log_file, "\nSUMMARY")?;
        writeln!(
            log_file,
            "Total: {}, Success: {}, Failed: {}",
            processed, successful, failed
        )?;
        writeln!(log_file, "Avg simulation time: {:?}", avg)?;
        writeln!(
            log_file,
            "Throughput: {:.0} tx/sec",
            1_000_000.0 / avg.as_micros() as f64
        )?;
    }

    println!("\n✅ Complete! Log saved to: {}", log_path.display());
    header_task.abort();
    Ok(())
}

// Note: No longer need get_raw_tx function since MempoolSimulator
// handles transaction processing directly from MempoolTransaction
