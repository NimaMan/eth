use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
use crate::pools::state::tracks::TransferPolicySignal;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferPolicyObservation {
    pub signal: TransferPolicySignal,
    pub present: bool,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub function: Option<String>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl TransferPolicyObservation {
    pub fn synthetic(signal: TransferPolicySignal, note: impl Into<String>) -> Self {
        Self {
            signal,
            present: true,
            actor: None,
            subject: None,
            function: None,
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

    pub fn absent(signal: TransferPolicySignal, note: impl Into<String>) -> Self {
        Self {
            present: false,
            evidence: EvidenceRef::new(EvidenceSourceKind::Classification, EvidenceConfidence::Low)
                .with_note(note),
            ..Self::synthetic(signal, "synthetic absence")
        }
    }
}
