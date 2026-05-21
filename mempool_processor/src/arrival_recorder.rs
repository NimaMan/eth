use alloy_primitives::B256;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reth_chain_query::reth_index::database::RethIndexDB;
use reth_chain_query::reth_index::writers::mempool_arrival_writer::MempoolArrivalWriter;

/// Configuration for the mempool arrival recorder
#[derive(Clone, Debug)]
pub struct ArrivalRecorderConfig {
    pub flush_interval: Duration,
    pub batch_size: usize,
    pub max_entry_age: Duration,
}

impl Default for ArrivalRecorderConfig {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_secs(5),
            batch_size: 1000,
            max_entry_age: Duration::from_secs(2 * 24 * 60 * 60),
        }
    }
}

#[derive(Clone, Debug)]
struct PendingArrival {
    first_seen_ms: u64,
    last_attempt_ms: u64,
    attempts: u32,
}

#[derive(Default)]
struct ArrivalRecorderCounters {
    seen: AtomicU64,
    resolved: AtomicU64,
    written: AtomicU64,
    expired: AtomicU64,
    flush_errors: AtomicU64,
}

#[derive(Clone, Debug, Default)]
pub struct ArrivalRecorderStats {
    pub seen: u64,
    pub pending: u64,
    pub resolved: u64,
    pub written: u64,
    pub unresolved: u64,
    pub expired: u64,
    pub flush_errors: u64,
}

/// Records first-seen times in ms for mempool tx hashes and writes when included
pub struct MempoolArrivalRecorder {
    pending: Arc<Mutex<HashMap<B256, PendingArrival>>>,
    counters: Arc<ArrivalRecorderCounters>,
    _db: Arc<RethIndexDB>,
    _writer: Arc<MempoolArrivalWriter>,
    _flush_task: tokio::task::JoinHandle<()>,
}

impl MempoolArrivalRecorder {
    /// Create and start the background flusher
    pub fn new(
        db: Arc<RethIndexDB>,
        writer: Arc<MempoolArrivalWriter>,
        cfg: ArrivalRecorderConfig,
    ) -> Self {
        let pending: Arc<Mutex<HashMap<B256, PendingArrival>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let counters = Arc::new(ArrivalRecorderCounters::default());
        let pending_clone = pending.clone();
        let counters_clone = counters.clone();
        let writer_clone = writer.clone();
        let flush_interval = cfg.flush_interval;
        let batch_size = cfg.batch_size;
        let max_entry_age = cfg.max_entry_age;
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(flush_interval);
            loop {
                ticker.tick().await;
                let now_ms = Utc::now().timestamp_millis() as u64;
                let mut batch: Vec<(B256, u64)> = Vec::new();

                {
                    let mut map = pending_clone.lock();

                    let max_age_ms = max_entry_age.as_millis();
                    let mut expired = 0u64;
                    map.retain(|_, arrival| {
                        let age_ms = u128::from(now_ms.saturating_sub(arrival.first_seen_ms));
                        let keep = age_ms <= max_age_ms;
                        if !keep {
                            expired += 1;
                        }
                        keep
                    });
                    if expired > 0 {
                        counters_clone.expired.fetch_add(expired, Ordering::Relaxed);
                    }

                    let mut candidates: Vec<(B256, PendingArrival)> = map
                        .iter()
                        .map(|(hash, arrival)| (*hash, arrival.clone()))
                        .collect();
                    candidates.sort_by(|(_, a), (_, b)| {
                        a.attempts
                            .cmp(&b.attempts)
                            .then_with(|| a.last_attempt_ms.cmp(&b.last_attempt_ms))
                            .then_with(|| a.first_seen_ms.cmp(&b.first_seen_ms))
                    });

                    for (hash, arrival) in candidates.into_iter().take(batch_size) {
                        batch.push((hash, arrival.first_seen_ms));
                    }

                    if batch.is_empty() {
                        continue;
                    }
                }

                let writer_for_flush = writer_clone.clone();
                let batch_for_flush = batch.clone();
                let write_result = tokio::task::spawn_blocking(move || {
                    writer_for_flush.write_arrivals_by_hashes_ms_return_resolved(&batch_for_flush)
                })
                .await;

                match write_result {
                    Ok(Ok(resolved)) => {
                        let resolved_set: HashSet<B256> = resolved.into_iter().collect();
                        let resolved_count = resolved_set.len() as u64;
                        counters_clone
                            .resolved
                            .fetch_add(resolved_count, Ordering::Relaxed);
                        counters_clone
                            .written
                            .fetch_add(resolved_count, Ordering::Relaxed);

                        let mut map = pending_clone.lock();
                        for hash in &resolved_set {
                            map.remove(hash);
                        }
                        for (hash, _) in &batch {
                            if !resolved_set.contains(hash) {
                                if let Some(arrival) = map.get_mut(hash) {
                                    arrival.attempts = arrival.attempts.saturating_add(1);
                                    arrival.last_attempt_ms = now_ms;
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("MempoolArrivalRecorder flush error: {}", e);
                        counters_clone.flush_errors.fetch_add(1, Ordering::Relaxed);
                        let mut map = pending_clone.lock();
                        for (hash, _) in &batch {
                            if let Some(arrival) = map.get_mut(hash) {
                                arrival.attempts = arrival.attempts.saturating_add(1);
                                arrival.last_attempt_ms = now_ms;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("MempoolArrivalRecorder flush task join error: {}", e);
                        counters_clone.flush_errors.fetch_add(1, Ordering::Relaxed);
                        let mut map = pending_clone.lock();
                        for (hash, _) in &batch {
                            if let Some(arrival) = map.get_mut(hash) {
                                arrival.attempts = arrival.attempts.saturating_add(1);
                                arrival.last_attempt_ms = now_ms;
                            }
                        }
                    }
                }
            }
        });
        Self {
            pending,
            counters,
            _db: db,
            _writer: writer,
            _flush_task: handle,
        }
    }

    /// Record first seen in ms for a tx hash string (with or without 0x)
    pub fn record_hash_hex_ms(&self, hash_hex: &str) {
        let ts_ms = Utc::now().timestamp_millis() as u64;
        self.record_hash_hex_ms_at(hash_hex, ts_ms);
    }

    /// Record first seen in ms for a tx hash string using a caller-supplied timestamp.
    pub fn record_hash_hex_ms_at(&self, hash_hex: &str, ts_ms: u64) {
        let h = hash_hex.strip_prefix("0x").unwrap_or(hash_hex);
        if let Ok(bytes) = hex::decode(h) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                let hash = B256::from(arr);
                self.counters.seen.fetch_add(1, Ordering::Relaxed);
                let mut map = self.pending.lock();
                map.entry(hash)
                    .and_modify(|arrival| {
                        if ts_ms < arrival.first_seen_ms {
                            arrival.first_seen_ms = ts_ms;
                        }
                    })
                    .or_insert(PendingArrival {
                        first_seen_ms: ts_ms,
                        last_attempt_ms: 0,
                        attempts: 0,
                    });
            }
        }
    }

    pub fn stats(&self) -> ArrivalRecorderStats {
        let pending = self.pending.lock();
        ArrivalRecorderStats {
            seen: self.counters.seen.load(Ordering::Relaxed),
            pending: pending.len() as u64,
            resolved: self.counters.resolved.load(Ordering::Relaxed),
            written: self.counters.written.load(Ordering::Relaxed),
            unresolved: pending
                .values()
                .filter(|arrival| arrival.attempts > 0)
                .count() as u64,
            expired: self.counters.expired.load(Ordering::Relaxed),
            flush_errors: self.counters.flush_errors.load(Ordering::Relaxed),
        }
    }
}
