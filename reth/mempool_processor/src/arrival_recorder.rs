use alloy_primitives::B256;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
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

/// Records first-seen times in ms for mempool tx hashes and writes when included
pub struct MempoolArrivalRecorder {
    pending: Arc<Mutex<HashMap<B256, u64>>>,
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
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();
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
                    map.retain(|_, ts| {
                        let age_ms = u128::from(now_ms.saturating_sub(*ts));
                        age_ms <= max_age_ms
                    });

                    for (hash, ts) in map.iter() {
                        batch.push((*hash, *ts));
                        if batch.len() == batch_size {
                            break;
                        }
                    }

                    if batch.is_empty() {
                        continue;
                    }

                    for (hash, _) in &batch {
                        map.remove(hash);
                    }
                }

                let mut to_requeue: Vec<(B256, u64)> = Vec::new();

                match writer_clone.write_arrivals_by_hashes_ms_return_resolved(&batch) {
                    Ok(resolved) => {
                        if resolved.len() < batch.len() {
                            let resolved_set: HashSet<B256> = resolved.into_iter().collect();
                            to_requeue = batch
                                .iter()
                                .filter(|(hash, _)| !resolved_set.contains(hash))
                                .cloned()
                                .collect();
                        }
                    }
                    Err(e) => {
                        tracing::warn!("MempoolArrivalRecorder flush error: {}", e);
                        to_requeue = batch.clone();
                    }
                }

                if !to_requeue.is_empty() {
                    let mut map = pending_clone.lock();
                    for (hash, ts) in to_requeue {
                        map.entry(hash)
                            .and_modify(|existing| {
                                if ts < *existing {
                                    *existing = ts;
                                }
                            })
                            .or_insert(ts);
                    }
                }
            }
        });
        Self {
            pending,
            _db: db,
            _writer: writer,
            _flush_task: handle,
        }
    }

    /// Record first seen in ms for a tx hash string (with or without 0x)
    pub fn record_hash_hex_ms(&self, hash_hex: &str) {
        let h = hash_hex.strip_prefix("0x").unwrap_or(hash_hex);
        if let Ok(bytes) = hex::decode(h) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                let hash = B256::from(arr);
                let ts_ms = Utc::now().timestamp_millis() as u64;
                let mut map = self.pending.lock();
                map.entry(hash)
                    .and_modify(|v| {
                        if ts_ms < *v {
                            *v = ts_ms;
                        }
                    })
                    .or_insert(ts_ms);
            }
        }
    }
}
