use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
use crate::pools::state::tracks::SupplyControlSignal;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SupplyControlObservation {
    pub signal: SupplyControlSignal,
    pub present: bool,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub amount: Option<f64>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl SupplyControlObservation {
    pub fn synthetic(signal: SupplyControlSignal, note: impl Into<String>) -> Self {
        Self {
            signal,
            present: true,
            actor: None,
            subject: None,
            amount: None,
            selector: None,
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
