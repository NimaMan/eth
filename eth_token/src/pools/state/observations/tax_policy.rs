use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
use crate::pools::state::tracks::TaxPolicySignal;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaxPolicyObservation {
    pub signal: TaxPolicySignal,
    pub present: bool,
    pub buy_tax_percent: Option<f64>,
    pub sell_tax_percent: Option<f64>,
    pub gas_used: Option<u64>,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl TaxPolicyObservation {
    pub fn synthetic(signal: TaxPolicySignal, note: impl Into<String>) -> Self {
        Self {
            signal,
            present: true,
            buy_tax_percent: None,
            sell_tax_percent: None,
            gas_used: None,
            actor: None,
            subject: None,
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
