use alloy_primitives::B256;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reth_chain_query::reth_index::database::RethIndexDB;
use reth_chain_query::reth_index::writers::mempool_arrival_writer::MempoolArrivalWriter;

/// Configuration for the mempool arrival recorder
#[derive(Clone, Debug)]
pub struct ArrivalRecorderConfig {
    pub flush_interval: Duration,
    pub batch_size: usize,
}

impl Default for ArrivalRecorderConfig {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_secs(5),
            batch_size: 1000,
        }
    }
}

/// Records first-seen times in ms for mempool tx hashes and writes when included
pub struct MempoolArrivalRecorder {
    pending: Arc<Mutex<HashMap<B256, u64>>>,
    _db: Arc<RethIndexDB>,
    writer: Arc<MempoolArrivalWriter>,
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
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(cfg.flush_interval);
            loop {
                ticker.tick().await;
                let mut batch: Vec<(B256, u64)> = {
                    let map = pending_clone.lock();
                    map.iter().map(|(h, ts)| (*h, *ts)).collect()
                };
                if batch.is_empty() {
                    continue;
                }
                if batch.len() > cfg.batch_size {
                    batch.truncate(cfg.batch_size);
                }
                match writer_clone.write_arrivals_by_hashes_ms_return_resolved(&batch) {
                    Ok(resolved) => {
                        if !resolved.is_empty() {
                            let mut map = pending_clone.lock();
                            for h in resolved {
                                map.remove(&h);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("MempoolArrivalRecorder flush error: {}", e);
                    }
                }
            }
        });
        Self {
            pending,
            _db: db,
            writer,
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
