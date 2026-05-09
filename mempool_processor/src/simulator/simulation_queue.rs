/// Priority-based Simulation Queue
///
/// Manages simulation requests with priority ordering
use super::TxSimulationJob;
use crate::tx_router::SimulationPriority;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use tracing::debug;

/// Wrapper for priority ordering
#[derive(Debug, Clone)]
struct PrioritizedRequest {
    request: TxSimulationJob,
    enqueued_at: std::time::Instant,
}

impl PartialEq for PrioritizedRequest {
    fn eq(&self, other: &Self) -> bool {
        self.request.priority == other.request.priority
    }
}

impl Eq for PrioritizedRequest {}

impl PartialOrd for PrioritizedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower priority value = higher priority (Critical=0, High=1, etc)
        // So we reverse the comparison
        other
            .request
            .priority
            .cmp(&self.request.priority)
            .then_with(|| self.enqueued_at.cmp(&other.enqueued_at))
    }
}

/// Priority queue for simulation requests
pub struct SimulationQueue {
    queue: BinaryHeap<PrioritizedRequest>,
    max_size: usize,

    // Statistics
    total_enqueued: u64,
    total_processed: u64,
    total_dropped: u64,
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub current_size: usize,
    pub total_enqueued: u64,
    pub total_processed: u64,
    pub total_dropped: u64,
    pub by_priority: std::collections::HashMap<String, u64>,
}

impl SimulationQueue {
    /// Create new simulation queue
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            max_size: 10000,
            total_enqueued: 0,
            total_processed: 0,
            total_dropped: 0,
        }
    }

    /// Push a request to the queue
    pub fn push(&mut self, request: TxSimulationJob) -> Result<(), String> {
        if self.queue.len() >= self.max_size {
            // Drop lowest priority items if queue is full
            if request.priority <= SimulationPriority::Normal {
                self.total_dropped += 1;
                return Err("Queue full, dropping low priority request".to_string());
            }

            // Make room for high priority request
            self.drop_lowest_priority();
        }

        debug!("Enqueuing {:?} priority request", request.priority);

        self.queue.push(PrioritizedRequest {
            request,
            enqueued_at: std::time::Instant::now(),
        });

        self.total_enqueued += 1;
        Ok(())
    }

    /// Pop a batch of requests
    pub fn pop_batch(&mut self, max_batch: usize) -> Vec<TxSimulationJob> {
        let mut batch = Vec::with_capacity(max_batch.min(self.queue.len()));

        while batch.len() < max_batch && !self.queue.is_empty() {
            if let Some(prioritized) = self.queue.pop() {
                batch.push(prioritized.request);
                self.total_processed += 1;
            }
        }

        if !batch.is_empty() {
            debug!("Popped batch of {} requests", batch.len());
        }

        batch
    }

    /// Pop a single request from the queue.
    pub fn pop_one(&mut self) -> Option<TxSimulationJob> {
        let request = self.queue.pop().map(|prioritized| prioritized.request);
        if request.is_some() {
            self.total_processed += 1;
        }
        request
    }

    /// Drop the lowest priority item
    fn drop_lowest_priority(&mut self) {
        // This is inefficient but simple - for production, use a different data structure
        let mut items: Vec<_> = self.queue.drain().collect();
        items.sort_by(|a, b| b.cmp(a)); // Reverse sort to get lowest priority last

        if let Some(_dropped) = items.pop() {
            self.total_dropped += 1;
            debug!("Dropped lowest priority request to make room");
        }

        self.queue = items.into_iter().collect();
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> QueueStats {
        let mut by_priority = std::collections::HashMap::new();

        for item in &self.queue {
            let priority_str = format!("{:?}", item.request.priority);
            *by_priority.entry(priority_str).or_insert(0) += 1;
        }

        QueueStats {
            current_size: self.queue.len(),
            total_enqueued: self.total_enqueued,
            total_processed: self.total_processed,
            total_dropped: self.total_dropped,
            by_priority,
        }
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}
