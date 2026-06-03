use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
use crate::pools::state::tracks::BehavioralOutcomeSignal;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BehavioralOutcomeObservation {
    pub signal: BehavioralOutcomeSignal,
    pub present: bool,
    pub rate: Option<f64>,
    pub sample_size: Option<u64>,
    pub buy_count: Option<u64>,
    pub sell_count: Option<u64>,
    pub pattern_id: Option<String>,
    pub signature: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl BehavioralOutcomeObservation {
    pub fn synthetic(signal: BehavioralOutcomeSignal, note: impl Into<String>) -> Self {
        Self {
            signal,
            present: true,
            rate: None,
            sample_size: None,
            buy_count: None,
            sell_count: None,
            pattern_id: None,
            signature: None,
            block_number: None,
            tx_hash: None,
            evidence: EvidenceRef::new(
                EvidenceSourceKind::Classification,
                EvidenceConfidence::Medium,
            )
            .with_note(note),
        }
    }
}
