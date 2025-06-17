use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Performance metrics for transaction processing
#[derive(Debug, Clone)]
pub struct TransactionTiming {
    /// Transaction hash
    pub tx_hash: String,
    
    /// When the transaction arrived at our Reth node (from mempool)
    pub reth_arrival_time: Instant,
    
    /// When we started processing the transaction
    pub processing_start_time: Instant,
    
    /// When we completed processing the transaction
    pub processing_end_time: Option<Instant>,
    
    /// Unix timestamp for logging
    pub unix_timestamp: u64,
}

impl TransactionTiming {
    pub fn new(tx_hash: String) -> Self {
        let now = Instant::now();
        Self {
            tx_hash,
            reth_arrival_time: now,
            processing_start_time: now,
            processing_end_time: None,
            unix_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
    
    /// Create with a specific arrival time (e.g., from WebSocket)
    pub fn with_arrival_time(tx_hash: String, arrival_time: Instant) -> Self {
        let now = Instant::now();
        Self {
            tx_hash,
            reth_arrival_time: arrival_time,
            processing_start_time: now,
            processing_end_time: None,
            unix_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
    
    /// Mark the start of processing
    pub fn mark_processing_start(&mut self) {
        self.processing_start_time = Instant::now();
    }
    
    /// Mark the end of processing
    pub fn mark_processing_end(&mut self) {
        self.processing_end_time = Some(Instant::now());
    }
    
    /// Get the time from Reth arrival to processing start (queue time)
    pub fn queue_time_ms(&self) -> f64 {
        self.processing_start_time
            .duration_since(self.reth_arrival_time)
            .as_secs_f64() * 1000.0
    }
    
    /// Get the processing duration
    pub fn processing_time_ms(&self) -> Option<f64> {
        self.processing_end_time.map(|end| {
            end.duration_since(self.processing_start_time)
                .as_secs_f64() * 1000.0
        })
    }
    
    /// Get total time from arrival to completion
    pub fn total_time_ms(&self) -> Option<f64> {
        self.processing_end_time.map(|end| {
            end.duration_since(self.reth_arrival_time)
                .as_secs_f64() * 1000.0
        })
    }
}

/// Tracks performance metrics for transaction processing
pub struct PerformanceTracker {
    /// Recent transaction timings (circular buffer)
    recent_timings: Arc<RwLock<VecDeque<TransactionTiming>>>,
    
    /// Maximum number of timings to keep
    max_timings: usize,
    
    /// Counter for processed transactions
    tx_counter: Arc<RwLock<usize>>,
    
    /// Frequency of logging (every N transactions)
    log_frequency: usize,
}

impl PerformanceTracker {
    pub fn new(max_timings: usize, log_frequency: usize) -> Self {
        Self {
            recent_timings: Arc::new(RwLock::new(VecDeque::with_capacity(max_timings))),
            max_timings,
            tx_counter: Arc::new(RwLock::new(0)),
            log_frequency,
        }
    }
    
    /// Start tracking a new transaction
    pub async fn start_transaction(&self, tx_hash: String) -> Arc<RwLock<TransactionTiming>> {
        let timing = Arc::new(RwLock::new(TransactionTiming::new(tx_hash)));
        
        // Add to recent timings
        let mut timings = self.recent_timings.write().await;
        if timings.len() >= self.max_timings {
            timings.pop_front();
        }
        timings.push_back(timing.read().await.clone());
        
        timing
    }
    
    /// Start tracking a new transaction with specific arrival time
    pub async fn start_transaction_with_arrival(&self, tx_hash: String, arrival_time: Instant) -> Arc<RwLock<TransactionTiming>> {
        let timing = Arc::new(RwLock::new(TransactionTiming::with_arrival_time(tx_hash, arrival_time)));
        
        // Add to recent timings
        let mut timings = self.recent_timings.write().await;
        if timings.len() >= self.max_timings {
            timings.pop_front();
        }
        timings.push_back(timing.read().await.clone());
        
        timing
    }
    
    /// Complete tracking for a transaction
    pub async fn complete_transaction(&self, timing: Arc<RwLock<TransactionTiming>>) {
        // Mark completion time
        timing.write().await.mark_processing_end();
        
        // Update the timing in our collection
        let completed_timing = timing.read().await.clone();
        let mut timings = self.recent_timings.write().await;
        
        // Find and update the timing entry
        for stored_timing in timings.iter_mut() {
            if stored_timing.tx_hash == completed_timing.tx_hash {
                *stored_timing = completed_timing;
                break;
            }
        }
        
        // Update counter
        let mut counter = self.tx_counter.write().await;
        *counter += 1;
        
        // Check if we should log metrics
        if *counter % self.log_frequency == 0 {
            self.log_performance_metrics().await;
        }
    }
    
    /// Log performance metrics
    async fn log_performance_metrics(&self) {
        let timings = self.recent_timings.read().await;
        
        // Filter completed transactions
        let completed: Vec<&TransactionTiming> = timings
            .iter()
            .filter(|t| t.processing_end_time.is_some())
            .collect();
        
        if completed.is_empty() {
            return;
        }
        
        // Calculate queue times
        let queue_times: Vec<f64> = completed
            .iter()
            .map(|t| t.queue_time_ms())
            .collect();
        
        // Calculate processing times
        let processing_times: Vec<f64> = completed
            .iter()
            .filter_map(|t| t.processing_time_ms())
            .collect();
        
        // Calculate total times
        let total_times: Vec<f64> = completed
            .iter()
            .filter_map(|t| t.total_time_ms())
            .collect();
        
        // Calculate statistics
        let queue_mean = queue_times.iter().sum::<f64>() / queue_times.len() as f64;
        let queue_max = queue_times.iter().cloned().fold(f64::MIN, f64::max);
        
        let processing_mean = processing_times.iter().sum::<f64>() / processing_times.len() as f64;
        let processing_max = processing_times.iter().cloned().fold(f64::MIN, f64::max);
        
        let total_mean = total_times.iter().sum::<f64>() / total_times.len() as f64;
        let total_max = total_times.iter().cloned().fold(f64::MIN, f64::max);
        
        let tx_count = *self.tx_counter.read().await;
        
        // Log performance metrics
        info!("🚀 PERFORMANCE METRICS (last {} transactions)", completed.len());
        info!("📊 Total transactions processed: {}", tx_count);
        info!("⏱️  Queue Time (Reth arrival → Processing start):");
        info!("    Mean: {:.3}ms, Max: {:.3}ms", queue_mean, queue_max);
        info!("⚡ Processing Time (Start → End):");
        info!("    Mean: {:.3}ms, Max: {:.3}ms", processing_mean, processing_max);
        info!("📈 Total Time (Reth arrival → Processing end):");
        info!("    Mean: {:.3}ms, Max: {:.3}ms", total_mean, total_max);
        
        // Log detailed timing for debugging if verbose
        debug!("Detailed timings for last 5 transactions:");
        for (i, timing) in completed.iter().rev().take(5).enumerate() {
            debug!("  [{}] TX {}: Queue: {:.3}ms, Process: {:.3}ms, Total: {:.3}ms",
                i + 1,
                &timing.tx_hash[..8],
                timing.queue_time_ms(),
                timing.processing_time_ms().unwrap_or(0.0),
                timing.total_time_ms().unwrap_or(0.0)
            );
        }
    }
    
    /// Get current performance statistics
    pub async fn get_statistics(&self) -> PerformanceStatistics {
        let timings = self.recent_timings.read().await;
        
        let completed: Vec<&TransactionTiming> = timings
            .iter()
            .filter(|t| t.processing_end_time.is_some())
            .collect();
        
        if completed.is_empty() {
            return PerformanceStatistics::default();
        }
        
        let queue_times: Vec<f64> = completed.iter().map(|t| t.queue_time_ms()).collect();
        let processing_times: Vec<f64> = completed.iter().filter_map(|t| t.processing_time_ms()).collect();
        let total_times: Vec<f64> = completed.iter().filter_map(|t| t.total_time_ms()).collect();
        
        PerformanceStatistics {
            total_processed: *self.tx_counter.read().await,
            queue_time_mean_ms: queue_times.iter().sum::<f64>() / queue_times.len() as f64,
            queue_time_max_ms: queue_times.iter().cloned().fold(f64::MIN, f64::max),
            processing_time_mean_ms: processing_times.iter().sum::<f64>() / processing_times.len() as f64,
            processing_time_max_ms: processing_times.iter().cloned().fold(f64::MIN, f64::max),
            total_time_mean_ms: total_times.iter().sum::<f64>() / total_times.len() as f64,
            total_time_max_ms: total_times.iter().cloned().fold(f64::MIN, f64::max),
            sample_size: completed.len(),
        }
    }
}

/// Performance statistics summary
#[derive(Debug, Clone, Default)]
pub struct PerformanceStatistics {
    pub total_processed: usize,
    pub queue_time_mean_ms: f64,
    pub queue_time_max_ms: f64,
    pub processing_time_mean_ms: f64,
    pub processing_time_max_ms: f64,
    pub total_time_mean_ms: f64,
    pub total_time_max_ms: f64,
    pub sample_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_transaction_timing() {
        let mut timing = TransactionTiming::new("0x123".to_string());
        
        // Simulate some delay
        tokio::time::sleep(Duration::from_millis(5)).await;
        timing.mark_processing_start();
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        timing.mark_processing_end();
        
        assert!(timing.queue_time_ms() >= 5.0);
        assert!(timing.processing_time_ms().unwrap() >= 10.0);
        assert!(timing.total_time_ms().unwrap() >= 15.0);
    }
    
    #[tokio::test]
    async fn test_performance_tracker() {
        let tracker = PerformanceTracker::new(100, 5);
        
        // Process some transactions
        for i in 0..5 {
            let tx_hash = format!("0x{:x}", i);
            let timing = tracker.start_transaction(tx_hash).await;
            
            tokio::time::sleep(Duration::from_millis(2)).await;
            timing.write().await.mark_processing_start();
            
            tokio::time::sleep(Duration::from_millis(5)).await;
            tracker.complete_transaction(timing).await;
        }
        
        let stats = tracker.get_statistics().await;
        assert_eq!(stats.total_processed, 5);
        assert!(stats.processing_time_mean_ms >= 5.0);
    }
}

