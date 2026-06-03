use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
use crate::pools::state::tracks::ContractPostureSignal;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContractPostureObservation {
    pub signal: ContractPostureSignal,
    pub present: bool,
    pub contract_address: Option<String>,
    pub implementation_address: Option<String>,
    pub owner_address: Option<String>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl ContractPostureObservation {
    pub fn synthetic(signal: ContractPostureSignal, note: impl Into<String>) -> Self {
        Self {
            signal,
            present: true,
            contract_address: None,
            implementation_address: None,
            owner_address: None,
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
