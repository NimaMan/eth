use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use mempool_processor::{
    arrival_recorder::MempoolArrivalRecorder,
    mempool_fetcher::{IngressStatsSnapshot, MempoolIngressObserver},
};
use tokio::sync::mpsc;
use tracing::warn;

const ARRIVAL_RECORD_CHANNEL_CAPACITY: usize = 100_000;

pub(crate) fn create_arrival_recording_ingress_observer(
    arrival_recorder: Option<Arc<MempoolArrivalRecorder>>,
) -> Arc<dyn MempoolIngressObserver> {
    let sender = arrival_recorder.map(|recorder| {
        let (sender, mut receiver) =
            mpsc::channel::<ArrivalRecord>(ARRIVAL_RECORD_CHANNEL_CAPACITY);
        tokio::spawn(async move {
            while let Some(record) = receiver.recv().await {
                recorder.record_hash_hex_ms_at(&record.hash, record.first_seen_ms);
            }
            warn!("arrival recording ingress worker stopped");
        });
        sender
    });

    Arc::new(ArrivalRecordingIngressObserver {
        sender,
        skipped_records: AtomicU64::new(0),
    })
}

struct ArrivalRecordingIngressObserver {
    sender: Option<mpsc::Sender<ArrivalRecord>>,
    skipped_records: AtomicU64,
}

struct ArrivalRecord {
    hash: String,
    first_seen_ms: u64,
}

impl MempoolIngressObserver for ArrivalRecordingIngressObserver {
    fn tx_received(&self, hash: &str, first_seen_ms: u64, _stats: IngressStatsSnapshot) {
        self.enqueue_arrival(hash, first_seen_ms);
    }

    fn tx_dropped_queue_full(&self, hash: &str, first_seen_ms: u64, _stats: IngressStatsSnapshot) {
        self.enqueue_arrival(hash, first_seen_ms);
    }

    fn queue_depth_updated(&self, _stats: IngressStatsSnapshot) {}
}

impl ArrivalRecordingIngressObserver {
    fn enqueue_arrival(&self, hash: &str, first_seen_ms: u64) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };

        if let Err(err) = sender.try_send(ArrivalRecord {
            hash: hash.to_string(),
            first_seen_ms,
        }) {
            let skipped = self.skipped_records.fetch_add(1, Ordering::Relaxed) + 1;
            if skipped == 1 || skipped % 1000 == 0 {
                warn!(
                    "arrival recording channel full/closed; skipped={} latest_hash={} error={}",
                    skipped, hash, err
                );
            }
        }
    }
}
