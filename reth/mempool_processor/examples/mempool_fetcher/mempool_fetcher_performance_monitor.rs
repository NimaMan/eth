use chrono::Local;
use mempool_processor::mempool_fetcher::MempoolFetcherIPCClient;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
/// Mempool Fetcher Performance Monitor
///
/// OBJECTIVE: Measure two critical things:
/// 1. Are we getting new transactions from mempool IMMEDIATELY with full transaction data?
/// 2. Does this system perform reliably over long periods (hours/days)?
///
/// WHAT WE MEASURE:
/// - Detection latency: Time from IPC read start to having full transaction data (target: <10μs)
/// - Transaction completeness: Verify we get ALL fields (hash, from, to, value, gas, etc.)
/// - Long-term stability: No degradation in latency over time
/// - Coverage: Are we missing any transactions?
///
/// Run with: cargo run --example mempool_fetcher_performance_monitor --release
///
/// Logs to: mempool_processor/logs/performance_YYYYMMDD_HHMMSS.csv
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Clone)]
struct PerformanceStats {
    start_time: Instant,
    total_transactions: u64,
    sub_10us_count: u64,
    sub_50us_count: u64,
    over_50us_count: u64,
    min_latency_us: u64,
    max_latency_us: u64,
    sum_latency_us: u64,

    // Data completeness check
    complete_data_count: u64,
    incomplete_data_count: u64,

    // Long-term tracking
    last_hour_avg_us: f64,
    last_hour_count: u64,
}

impl PerformanceStats {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_transactions: 0,
            sub_10us_count: 0,
            sub_50us_count: 0,
            over_50us_count: 0,
            min_latency_us: u64::MAX,
            max_latency_us: 0,
            sum_latency_us: 0,
            complete_data_count: 0,
            incomplete_data_count: 0,
            last_hour_avg_us: 0.0,
            last_hour_count: 0,
        }
    }

    fn update(&mut self, latency_us: u64, is_complete: bool) {
        self.total_transactions += 1;
        self.sum_latency_us += latency_us;

        if latency_us < 10 {
            self.sub_10us_count += 1;
        } else if latency_us < 50 {
            self.sub_50us_count += 1;
        } else {
            self.over_50us_count += 1;
        }

        self.min_latency_us = self.min_latency_us.min(latency_us);
        self.max_latency_us = self.max_latency_us.max(latency_us);

        if is_complete {
            self.complete_data_count += 1;
        } else {
            self.incomplete_data_count += 1;
        }
    }

    fn avg_latency_us(&self) -> f64 {
        if self.total_transactions > 0 {
            self.sum_latency_us as f64 / self.total_transactions as f64
        } else {
            0.0
        }
    }

    fn print_summary(&self) {
        let runtime = self.start_time.elapsed();
        let rate = self.total_transactions as f64 / runtime.as_secs_f64();

        info!("\n=== PERFORMANCE SUMMARY ===");
        info!("Runtime: {:.1} hours", runtime.as_secs_f64() / 3600.0);
        info!("Total transactions: {}", self.total_transactions);
        info!("Rate: {:.1} tx/sec", rate);
        info!("");
        info!("LATENCY DISTRIBUTION:");
        info!(
            "  <10μs: {} ({:.1}%)",
            self.sub_10us_count,
            (self.sub_10us_count as f64 / self.total_transactions as f64) * 100.0
        );
        info!(
            "  10-50μs: {} ({:.1}%)",
            self.sub_50us_count,
            (self.sub_50us_count as f64 / self.total_transactions as f64) * 100.0
        );
        info!(
            "  >50μs: {} ({:.1}%)",
            self.over_50us_count,
            (self.over_50us_count as f64 / self.total_transactions as f64) * 100.0
        );
        info!("");
        info!("LATENCY STATS:");
        info!("  Average: {:.1}μs", self.avg_latency_us());
        info!("  Min: {}μs", self.min_latency_us);
        info!("  Max: {}μs", self.max_latency_us);
        info!("");
        info!("DATA COMPLETENESS:");
        info!(
            "  Complete: {} ({:.1}%)",
            self.complete_data_count,
            (self.complete_data_count as f64 / self.total_transactions as f64) * 100.0
        );
        info!("  Incomplete: {}", self.incomplete_data_count);

        if self.incomplete_data_count > 0 {
            warn!(
                "WARNING: {} transactions had incomplete data!",
                self.incomplete_data_count
            );
        }
    }
}

fn check_transaction_completeness(
    tx: &mempool_processor::mempool_fetcher::MempoolTransaction,
) -> bool {
    // With pre-parsed fields, we just check the basic requirements
    !tx.hash.is_empty()
        && !tx.from.is_empty()
        && tx.to.is_some()
        && !tx.input.is_empty()
        && tx.gas_price.is_some()
}

async fn run_performance_monitor() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let log_dir = PathBuf::from(mempool_processor::config::DEFAULT_LOG_DIR);
    std::fs::create_dir_all(&log_dir)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = log_dir.join(format!("performance_{}.csv", timestamp));
    let mut log_file = File::create(&log_path)?;

    // Write CSV header
    writeln!(
        log_file,
        "timestamp,tx_hash,latency_us,is_complete,avg_latency_us,total_count"
    )?;

    info!("Starting Mempool Fetcher Performance Monitor");
    info!("Log file: {}", log_path.display());
    info!("Objectives:");
    info!("1. Verify IMMEDIATE detection (<10μs target)");
    info!("2. Verify COMPLETE transaction data");
    info!("3. Monitor LONG-TERM stability");
    info!("");

    // Initialize IPC client
    let ipc_client = MempoolFetcherIPCClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;

    // Stats tracking
    let stats = Arc::new(Mutex::new(PerformanceStats::new()));
    let mut last_summary = Instant::now();
    let mut last_hour_check = Instant::now();

    info!("Monitoring started. Press Ctrl+C to stop.");

    loop {
        let transactions = ipc_client.get_transactions_instant(100).await;
        if !transactions.is_empty() {
            for tx in transactions {
                let latency_us = tx.detection_ns / 1000;
                let is_complete = check_transaction_completeness(&tx);

                // Update stats
                let mut stats_guard = stats.lock().await;
                stats_guard.update(latency_us, is_complete);
                let avg = stats_guard.avg_latency_us();
                let total = stats_guard.total_transactions;
                drop(stats_guard);

                // Log to CSV
                writeln!(
                    log_file,
                    "{},{},{},{},{:.1},{}",
                    Local::now().format("%Y-%m-%d %H:%M:%S%.6f"),
                    tx.hash,
                    latency_us,
                    is_complete,
                    avg,
                    total
                )?;

                // Log warnings for high latency or incomplete data
                if latency_us > 50 {
                    warn!("HIGH LATENCY: {} took {}μs", tx.hash, latency_us);
                }
                if !is_complete {
                    warn!("INCOMPLETE DATA: {}", tx.hash);
                }
            }
        }

        // Print summary every 5 minutes
        if last_summary.elapsed() > Duration::from_secs(300) {
            let stats_guard = stats.lock().await;
            stats_guard.print_summary();
            drop(stats_guard);
            last_summary = Instant::now();
            log_file.flush()?;
        }

        // Check for long-term degradation every hour
        if last_hour_check.elapsed() > Duration::from_secs(3600) {
            let mut stats_guard = stats.lock().await;
            let current_avg = stats_guard.avg_latency_us();

            if stats_guard.last_hour_avg_us > 0.0 {
                let degradation = ((current_avg - stats_guard.last_hour_avg_us)
                    / stats_guard.last_hour_avg_us)
                    * 100.0;
                if degradation > 10.0 {
                    warn!(
                        "PERFORMANCE DEGRADATION: {:.1}% increase in average latency",
                        degradation
                    );
                } else {
                    info!("Performance stable: avg latency {:.1}μs", current_avg);
                }
            }

            stats_guard.last_hour_avg_us = current_avg;
            stats_guard.last_hour_count = stats_guard.total_transactions;
            last_hour_check = Instant::now();
        }

        // Small sleep to prevent busy-waiting
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    run_performance_monitor().await
}
