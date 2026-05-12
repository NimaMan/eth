use std::sync::Arc;

use mempool_processor::{
    arrival_recorder::MempoolArrivalRecorder,
    mempool_fetcher::{IngressStatsSnapshot, MempoolIngressObserver},
};

pub(crate) fn create_arrival_recording_ingress_observer(
    arrival_recorder: Option<Arc<MempoolArrivalRecorder>>,
) -> Arc<dyn MempoolIngressObserver> {
    Arc::new(ArrivalRecordingIngressObserver { arrival_recorder })
}

struct ArrivalRecordingIngressObserver {
    arrival_recorder: Option<Arc<MempoolArrivalRecorder>>,
}

impl MempoolIngressObserver for ArrivalRecordingIngressObserver {
    fn tx_received(&self, hash: &str, first_seen_ms: u64, _stats: IngressStatsSnapshot) {
        if let Some(recorder) = self.arrival_recorder.as_ref() {
            recorder.record_hash_hex_ms_at(hash, first_seen_ms);
        }
    }

    fn tx_dropped_queue_full(&self, hash: &str, first_seen_ms: u64, _stats: IngressStatsSnapshot) {
        if let Some(recorder) = self.arrival_recorder.as_ref() {
            recorder.record_hash_hex_ms_at(hash, first_seen_ms);
        }
    }

    fn queue_depth_updated(&self, _stats: IngressStatsSnapshot) {}
}
