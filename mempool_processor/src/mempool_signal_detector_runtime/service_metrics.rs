use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

/// Service-level counters and latency summaries for the live detector process.
pub(crate) struct ServiceMetrics {
    pub(crate) total_processed: AtomicU64,
    pub(crate) contract_creations: AtomicU64,
    pub(crate) creator_actions: AtomicU64,
    pub(crate) dex_interactions: AtomicU64,
    pub(crate) regular_txs: AtomicU64,
    pub(crate) simulations_submitted: AtomicU64,
    pub(crate) simulations_completed: AtomicU64,
    pub(crate) simulation_errors: AtomicU64,
    pub(crate) trading_enabled_signals: Arc<AtomicU64>,
    pub(crate) liquidity_removal_signals: Arc<AtomicU64>,
    pub(crate) sell_blocked_signals: Arc<AtomicU64>,
    pub(crate) tax_change_signals: Arc<AtomicU64>,
    detection_latencies: Arc<Mutex<Vec<Duration>>>,
    simulation_times: Arc<Mutex<Vec<Duration>>>,
}

impl ServiceMetrics {
    pub(crate) fn new() -> Self {
        Self {
            total_processed: AtomicU64::new(0),
            contract_creations: AtomicU64::new(0),
            creator_actions: AtomicU64::new(0),
            dex_interactions: AtomicU64::new(0),
            regular_txs: AtomicU64::new(0),
            simulations_submitted: AtomicU64::new(0),
            simulations_completed: AtomicU64::new(0),
            simulation_errors: AtomicU64::new(0),
            trading_enabled_signals: Arc::new(AtomicU64::new(0)),
            liquidity_removal_signals: Arc::new(AtomicU64::new(0)),
            sell_blocked_signals: Arc::new(AtomicU64::new(0)),
            tax_change_signals: Arc::new(AtomicU64::new(0)),
            detection_latencies: Arc::new(Mutex::new(Vec::with_capacity(10000))),
            simulation_times: Arc::new(Mutex::new(Vec::with_capacity(1000))),
        }
    }

    pub(crate) async fn add_detection_latency(&self, latency: Duration) {
        let mut latencies = self.detection_latencies.lock().await;
        if latencies.len() >= 10000 {
            latencies.drain(0..5000);
        }
        latencies.push(latency);
    }

    pub(crate) async fn add_simulation_time(&self, time: Duration) {
        let mut times = self.simulation_times.lock().await;
        if times.len() >= 1000 {
            times.drain(0..500);
        }
        times.push(time);
    }

    pub(crate) async fn report(&self, elapsed: Duration) -> String {
        let detection_latencies = self.detection_latencies.lock().await;
        let (avg_detect, max_detect, _p99_detect) = calculate_latency_stats(&detection_latencies);
        drop(detection_latencies);

        let simulation_times = self.simulation_times.lock().await;
        let (avg_sim, max_sim, _p99_sim) = calculate_latency_stats(&simulation_times);
        drop(simulation_times);

        let total = self.total_processed.load(Ordering::Relaxed);
        let creations = self.contract_creations.load(Ordering::Relaxed);
        let creator_actions = self.creator_actions.load(Ordering::Relaxed);
        let _dex = self.dex_interactions.load(Ordering::Relaxed);
        let _regular = self.regular_txs.load(Ordering::Relaxed);
        let sims_completed = self.simulations_completed.load(Ordering::Relaxed);
        let sim_errors = self.simulation_errors.load(Ordering::Relaxed);
        let rate = total as f64 / elapsed.as_secs_f64();

        format!(
            "TX: {} ({:.1}/s) | Detect: {}us/{}us | Sim: {:.1}ms/{:.1}ms | CC:{} CA:{} | Sims done/actionable_err:{}/{} | Signals: TE:{} LR:{} SELL_BLOCKED:{} TC:{}",
            total,
            rate,
            avg_detect.as_micros(),
            max_detect.as_micros(),
            avg_sim.as_secs_f64() * 1000.0,
            max_sim.as_secs_f64() * 1000.0,
            creations,
            creator_actions,
            sims_completed,
            sim_errors,
            self.trading_enabled_signals.load(Ordering::Relaxed),
            self.liquidity_removal_signals.load(Ordering::Relaxed),
            self.sell_blocked_signals.load(Ordering::Relaxed),
            self.tax_change_signals.load(Ordering::Relaxed),
        )
    }
}

fn calculate_latency_stats(latencies: &[Duration]) -> (Duration, Duration, Duration) {
    if latencies.is_empty() {
        return (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    }

    let mut sorted = latencies.to_vec();
    sorted.sort();

    let sum: Duration = sorted.iter().sum();
    let avg = sum / sorted.len() as u32;
    let max = sorted.last().copied().unwrap_or(Duration::ZERO);
    let p99 = sorted.get(sorted.len() * 99 / 100).copied().unwrap_or(max);

    (avg, max, p99)
}
