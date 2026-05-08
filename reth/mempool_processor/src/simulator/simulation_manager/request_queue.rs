use std::{future::Future, sync::Arc};

use futures::future::join_all;
use tokio::sync::Mutex;

use super::{
    types::{SimulationResult, TxSimulationJob},
    QueueStats, SimulationQueue,
};

#[derive(Debug, Default, Clone)]
pub struct ManagerStats {
    pub total_requests: u64,
    pub successful_simulations: u64,
    pub failed_simulations: u64,
    pub buy_sell_tests: u64,
    pub avg_simulation_time_ms: f64,
    pub max_simulation_time_ms: f64,
    pub queue_current_size: usize,
    pub queue_total_enqueued: u64,
    pub queue_total_processed: u64,
    pub queue_total_dropped: u64,
}

#[derive(Clone)]
pub struct RequestQueue {
    queue: Arc<Mutex<SimulationQueue>>,
    stats: Arc<Mutex<ManagerStats>>,
    max_batch: usize,
}

impl RequestQueue {
    pub fn new(max_batch: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(SimulationQueue::new())),
            stats: Arc::new(Mutex::new(ManagerStats::default())),
            max_batch,
        }
    }

    pub async fn submit(&self, request: TxSimulationJob) -> Result<(), String> {
        let mut queue = self.queue.lock().await;
        queue.push(request)?;

        let mut stats = self.stats.lock().await;
        stats.total_requests += 1;

        Ok(())
    }

    pub async fn process<F, Fut>(&self, mut runner: F) -> Vec<SimulationResult>
    where
        F: FnMut(TxSimulationJob) -> Fut,
        Fut: Future<Output = SimulationResult>,
    {
        let requests = {
            let mut queue = self.queue.lock().await;
            queue.pop_batch(self.max_batch)
        };

        if requests.is_empty() {
            return Vec::new();
        }

        let mut futures = Vec::with_capacity(requests.len());
        for request in requests {
            futures.push(runner(request));
        }

        let batch_results = join_all(futures).await;
        self.record_results(&batch_results).await;
        batch_results
    }

    pub async fn pop_one(&self) -> Option<TxSimulationJob> {
        let mut queue = self.queue.lock().await;
        queue.pop_one()
    }

    pub async fn queue_stats(&self) -> QueueStats {
        let queue = self.queue.lock().await;
        queue.get_stats()
    }

    pub async fn stats(&self) -> ManagerStats {
        let queue_stats = self.queue_stats().await;
        let mut stats = self.stats.lock().await.clone();
        stats.queue_current_size = queue_stats.current_size;
        stats.queue_total_enqueued = queue_stats.total_enqueued;
        stats.queue_total_processed = queue_stats.total_processed;
        stats.queue_total_dropped = queue_stats.total_dropped;
        stats
    }

    pub async fn record_result(&self, result: &SimulationResult) {
        self.record_results(std::slice::from_ref(result)).await;
    }

    async fn record_results(&self, batch_results: &[SimulationResult]) {
        let mut stats = self.stats.lock().await;
        for result in batch_results {
            if result.error.is_none() {
                stats.successful_simulations += 1;
            } else {
                stats.failed_simulations += 1;
            }

            if result.buy_sell_result().is_some() {
                stats.buy_sell_tests += 1;
            }

            let total = stats.successful_simulations + stats.failed_simulations;
            stats.avg_simulation_time_ms = (stats.avg_simulation_time_ms * (total - 1) as f64
                + result.simulation_time_ms)
                / total as f64;
            stats.max_simulation_time_ms =
                stats.max_simulation_time_ms.max(result.simulation_time_ms);
        }
    }
}
