use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Thread-safe metrics for transaction processing
#[derive(Debug)]
pub struct TransactionMetrics {
    // Counters
    pub total_transactions: AtomicU64,
    
    // Latency tracking (in microseconds)
    pub total_latency_us: AtomicU64,
    pub min_latency_us: AtomicU64,
    pub max_latency_us: AtomicU64,
    
    // Timing
    start_time: Instant,
}

impl TransactionMetrics {
    pub fn new() -> Self {
        Self {
            total_transactions: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            min_latency_us: AtomicU64::new(u64::MAX),
            max_latency_us: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Record a transaction with its processing latency
    pub fn record_transaction(&self, latency_us: u64) {
        self.total_transactions.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        
        // Update min latency
        self.min_latency_us.fetch_min(latency_us, Ordering::Relaxed);
        
        // Update max latency
        self.max_latency_us.fetch_max(latency_us, Ordering::Relaxed);
    }

    /// Get average latency in microseconds
    pub fn average_latency_us(&self) -> f64 {
        let total = self.total_transactions.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        self.total_latency_us.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Get transactions per second
    pub fn tps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.total_transactions.load(Ordering::Relaxed) as f64 / elapsed
    }

    /// Generate a performance report
    pub fn report(&self) -> PerformanceReport {
        let total = self.total_transactions.load(Ordering::Relaxed);
        let min = self.min_latency_us.load(Ordering::Relaxed);
        
        PerformanceReport {
            total_transactions: total,
            average_latency_us: self.average_latency_us(),
            min_latency_us: if min == u64::MAX { 0 } else { min },
            max_latency_us: self.max_latency_us.load(Ordering::Relaxed),
            tps: self.tps(),
            elapsed_secs: self.start_time.elapsed().as_secs_f64(),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.total_transactions.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.min_latency_us.store(u64::MAX, Ordering::Relaxed);
        self.max_latency_us.store(0, Ordering::Relaxed);
    }
}

/// Performance report snapshot
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub total_transactions: u64,
    pub average_latency_us: f64,
    pub min_latency_us: u64,
    pub max_latency_us: u64,
    pub tps: f64,
    pub elapsed_secs: f64,
}

impl std::fmt::Display for PerformanceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Performance Report ===")?;
        writeln!(f, "Total transactions: {}", self.total_transactions)?;
        writeln!(f, "Elapsed time: {:.1}s", self.elapsed_secs)?;
        writeln!(f, "Throughput: {:.1} TPS", self.tps)?;
        writeln!(f, "Average latency: {:.1} µs", self.average_latency_us)?;
        writeln!(f, "Min latency: {} µs", self.min_latency_us)?;
        writeln!(f, "Max latency: {} µs", self.max_latency_us)?;
        write!(f, "========================")
    }
}