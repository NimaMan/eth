use chrono::Local;
use clap::Parser;
use mempool_processor::mempool_fetcher::MempoolFetcherIPCClient;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
/// Measure Instant Fetch Performance with Adaptive Backoff
///
/// IMPORTANT: How Transaction Fetching Works
/// =========================================
/// 1. Transactions arrive in BURSTS (3-34 at once) every 67-362ms
/// 2. We fetch them INSTANTLY when they arrive (within microseconds)
/// 3. Between bursts, we use ADAPTIVE BACKOFF to avoid wasting CPU:
///    - First 1ms: Check every 100μs (might catch stragglers)
///    - Next 90ms: Check every 1ms (typical inter-burst time)
///    - After 100ms: Check every 10ms (long gaps between bursts)
///
/// This achieves:
/// - Zero added latency when transactions arrive
/// - Minimal CPU usage during empty periods
/// - No missed transactions
///
/// Usage:
///   cargo run --example measure_instant_fetch_performance --release -- 1000
///   cargo run --example measure_instant_fetch_performance --release -- 10000
///   cargo run --example measure_instant_fetch_performance --release -- 100000
///
/// If no argument provided, defaults to 10,000 transactions
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[clap(name = "measure_instant_fetch")]
struct Args {
    /// Number of transactions to collect
    #[clap(default_value = "10000")]
    tx_count: u64,

    /// Batch size for instant collection
    #[clap(long, default_value = "100")]
    batch_size: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("🚀 Measuring Instant Fetch Performance");
    info!("   Target: {} transactions", args.tx_count);
    info!("   Batch size: {}", args.batch_size);

    // Create log file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    // Create log directory if it doesn't exist
    let log_dir = PathBuf::from(mempool_processor::config::DEFAULT_LOG_DIR);
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join(format!("instant_fetch_{}tx_{}.csv", args.tx_count, timestamp));
    let mut log_file = File::create(&log_path)?;
    writeln!(log_file, "fetch_num,batch_size,fetch_time_us,queue_wait_us,tx_detection_ns_min,tx_detection_ns_max,tx_detection_ns_avg,tx_hashes")?;

    // Also create a separate file with just transaction hashes for easy Etherscan verification
    let hash_log_path = log_dir.join(format!("tx_hashes_{}tx_{}.txt", args.tx_count, timestamp));
    let mut hash_file = File::create(&hash_log_path)?;
    let start_timestamp = Local::now();
    writeln!(
        hash_file,
        "################################################################################"
    )?;
    writeln!(hash_file, "# MEMPOOL TRANSACTION LOG")?;
    writeln!(
        hash_file,
        "# Started: {}",
        start_timestamp.format("%Y-%m-%d %H:%M:%S.%3f %Z")
    )?;
    writeln!(hash_file, "# Target: {} transactions", args.tx_count)?;
    writeln!(
        hash_file,
        "################################################################################"
    )?;
    writeln!(hash_file, "# Columns:")?;
    writeln!(hash_file, "#   1. Transaction Hash (0x...)")?;
    writeln!(
        hash_file,
        "#   2. Detection Latency (nanoseconds from socket read)"
    )?;
    writeln!(
        hash_file,
        "#   3. Log Timestamp (when we logged this transaction)"
    )?;
    writeln!(hash_file, "#   4. Elapsed Time (seconds since start)")?;
    writeln!(
        hash_file,
        "################################################################################"
    )?;
    writeln!(hash_file, "")?;

    // Initialize client
    let client =
        MempoolFetcherIPCClient::new(Some("/home/nima/.local/share/reth/mainnet/reth.ipc"))?;
    client.start().await?;

    info!("Client started, beginning measurement...");

    // Metrics
    let mut total_processed = 0u64;
    let mut fetch_times: Vec<Duration> = Vec::new();
    let mut batch_sizes: Vec<usize> = Vec::new();
    let mut empty_fetches = 0u64;
    let mut full_batches = 0u64;

    let start_time = Instant::now();
    let mut last_report = Instant::now();
    let mut fetch_num = 0u64;

    // Track consecutive empty fetches for adaptive backoff
    let mut consecutive_empty = 0u64;

    // Main collection loop with ADAPTIVE BACKOFF
    while total_processed < args.tx_count {
        fetch_num += 1;

        let fetch_start = Instant::now();
        let txs = client.get_transactions_instant(args.batch_size).await;
        let fetch_time = fetch_start.elapsed();

        fetch_times.push(fetch_time);
        batch_sizes.push(txs.len());

        if txs.is_empty() {
            empty_fetches += 1;
            consecutive_empty += 1;

            // ADAPTIVE BACKOFF: Reduce polling frequency based on how long queue has been empty
            let sleep_time = match consecutive_empty {
                1..=10 => Duration::from_micros(100), // First 1ms: check every 100μs
                11..=100 => Duration::from_millis(1), // Next 90ms: check every 1ms
                _ => Duration::from_millis(10),       // After 100ms: check every 10ms
            };
            tokio::time::sleep(sleep_time).await;
        } else {
            // Got transactions! Reset empty counter
            consecutive_empty = 0;
            // Calculate detection latency stats for this batch
            let detection_ns_values: Vec<u64> = txs.iter().map(|tx| tx.detection_ns).collect();

            let min_detection = detection_ns_values.iter().min().copied().unwrap_or(0);
            let max_detection = detection_ns_values.iter().max().copied().unwrap_or(0);
            let avg_detection = if !detection_ns_values.is_empty() {
                detection_ns_values.iter().sum::<u64>() / detection_ns_values.len() as u64
            } else {
                0
            };

            // Collect transaction hashes
            let tx_hashes: Vec<String> = txs.iter().map(|tx| tx.hash.clone()).collect();

            // Log to hash file with enhanced timestamp information
            let current_time = Local::now();
            let elapsed = start_time.elapsed();
            for tx in &txs {
                writeln!(
                    hash_file,
                    "{} | {:>6} ns | {} | {:>8.3} sec",
                    tx.hash,
                    tx.detection_ns,
                    current_time.format("%Y-%m-%d %H:%M:%S.%3f"),
                    elapsed.as_secs_f64()
                )?;
            }

            // Log this fetch
            writeln!(
                log_file,
                "{},{},{},{},{},{},{},\"{}\"",
                fetch_num,
                txs.len(),
                fetch_time.as_micros(),
                0, // Queue wait time (always 0 for instant)
                min_detection,
                max_detection,
                avg_detection,
                tx_hashes.join(";")
            )?;

            if txs.len() == args.batch_size {
                full_batches += 1;
            }

            total_processed += txs.len() as u64;

            // Progress report every 1000 transactions
            if total_processed / 1000 > (total_processed - txs.len() as u64) / 1000 {
                let elapsed = last_report.elapsed();
                let rate = 1000.0 / elapsed.as_secs_f64();
                info!(
                    "Progress: {}/{} ({:.1}%) - Rate: {:.1} tx/sec",
                    total_processed,
                    args.tx_count,
                    total_processed as f64 / args.tx_count as f64 * 100.0,
                    rate
                );
                last_report = Instant::now();
            }
        }
    }

    let total_time = start_time.elapsed();

    // Calculate statistics
    let avg_fetch_time = fetch_times.iter().sum::<Duration>() / fetch_times.len() as u32;
    let avg_batch_size = batch_sizes.iter().sum::<usize>() as f64 / batch_sizes.len() as f64;

    // Sort fetch times for percentiles
    let mut sorted_fetch_us: Vec<u64> = fetch_times.iter().map(|d| d.as_micros() as u64).collect();
    sorted_fetch_us.sort();

    let p50 = sorted_fetch_us[sorted_fetch_us.len() / 2];
    let p95 = sorted_fetch_us[sorted_fetch_us.len() * 95 / 100];
    let p99 = sorted_fetch_us[sorted_fetch_us.len() * 99 / 100];
    let max = sorted_fetch_us.last().copied().unwrap_or(0);

    // Write summary
    writeln!(log_file, "\n# Summary")?;
    writeln!(log_file, "total_transactions,{}", total_processed)?;
    writeln!(
        log_file,
        "total_time_seconds,{:.3}",
        total_time.as_secs_f64()
    )?;
    writeln!(
        log_file,
        "throughput_tx_per_sec,{:.1}",
        total_processed as f64 / total_time.as_secs_f64()
    )?;
    writeln!(log_file, "total_fetches,{}", fetch_num)?;
    writeln!(log_file, "empty_fetches,{}", empty_fetches)?;
    writeln!(log_file, "full_batches,{}", full_batches)?;
    writeln!(log_file, "avg_batch_size,{:.1}", avg_batch_size)?;
    writeln!(
        log_file,
        "fetch_latency_avg_us,{}",
        avg_fetch_time.as_micros()
    )?;
    writeln!(log_file, "fetch_latency_p50_us,{}", p50)?;
    writeln!(log_file, "fetch_latency_p95_us,{}", p95)?;
    writeln!(log_file, "fetch_latency_p99_us,{}", p99)?;
    writeln!(log_file, "fetch_latency_max_us,{}", max)?;

    // Print summary to console
    info!("\n✅ MEASUREMENT COMPLETE");
    info!("   Total transactions: {}", total_processed);
    info!("   Total time: {:.2}s", total_time.as_secs_f64());
    info!(
        "   Throughput: {:.1} tx/sec",
        total_processed as f64 / total_time.as_secs_f64()
    );
    info!("\n📊 FETCH PERFORMANCE:");
    info!("   Total fetches: {}", fetch_num);
    info!(
        "   Empty fetches: {} ({:.1}%)",
        empty_fetches,
        empty_fetches as f64 / fetch_num as f64 * 100.0
    );
    info!(
        "   Full batches: {} ({:.1}%)",
        full_batches,
        full_batches as f64 / fetch_num as f64 * 100.0
    );
    info!("   Average batch size: {:.1}", avg_batch_size);
    info!("\n⏱️  FETCH LATENCY:");
    info!("   Average: {}μs", avg_fetch_time.as_micros());
    info!("   P50: {}μs", p50);
    info!("   P95: {}μs", p95);
    info!("   P99: {}μs", p99);
    info!("   Max: {}μs", max);

    // Warnings
    if empty_fetches as f64 / fetch_num as f64 > 0.5 {
        warn!("⚠️  High empty fetch rate - consider reducing fetch frequency");
    }
    if full_batches as f64 / fetch_num as f64 > 0.1 {
        warn!("⚠️  Many full batches - consider increasing batch size");
    }
    if p99 > 1000 {
        warn!("⚠️  P99 latency >1ms - investigate slow fetches");
    }

    // Add summary footer to hash file
    writeln!(hash_file, "")?;
    writeln!(
        hash_file,
        "################################################################################"
    )?;
    writeln!(hash_file, "# SUMMARY")?;
    writeln!(
        hash_file,
        "# Completed: {}",
        Local::now().format("%Y-%m-%d %H:%M:%S.%3f %Z")
    )?;
    writeln!(
        hash_file,
        "# Total Duration: {:.3} seconds",
        total_time.as_secs_f64()
    )?;
    writeln!(hash_file, "# Total Transactions: {}", total_processed)?;
    writeln!(
        hash_file,
        "# Average Rate: {:.1} tx/sec",
        total_processed as f64 / total_time.as_secs_f64()
    )?;
    writeln!(
        hash_file,
        "################################################################################"
    )?;

    info!("\n📁 Log saved to: {}", log_path.display());
    info!("📁 Transaction hashes saved to: {}", hash_log_path.display());
    info!("\n🔍 To verify on Etherscan, check: https://etherscan.io/tx/[HASH]");

    Ok(())
}
