use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::B256;
use mempool_processor::{
    arrival_recorder::MempoolArrivalRecorder,
    mempool_fetcher::{IngressStatsSnapshot, MempoolIngressObserver},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct LocalTimeFormatter;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimeFormatter {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = chrono::Local::now();
        write!(w, "[{}]", now.format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

pub(crate) fn parse_tx_hash_or_zero(hash: &str) -> B256 {
    let trimmed = hash.trim();
    let normalized = if trimmed.starts_with("0x") {
        trimmed.to_string()
    } else {
        format!("0x{trimmed}")
    };
    B256::from_str(&normalized).unwrap_or(B256::ZERO)
}

pub(crate) fn arrival_recording_ingress_observer(
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
