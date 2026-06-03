use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
use crate::pools::state::tracks::SellRestrictionSignal;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SellRestrictionObservation {
    pub signal: SellRestrictionSignal,
    pub present: bool,
    pub limit_tokens: Option<f64>,
    pub limit_denom: Option<f64>,
    pub limit_bps_of_supply: Option<f64>,
    pub cooldown_seconds: Option<u64>,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl SellRestrictionObservation {
    pub fn synthetic(signal: SellRestrictionSignal, note: impl Into<String>) -> Self {
        Self {
            signal,
            present: true,
            limit_tokens: None,
            limit_denom: None,
            limit_bps_of_supply: None,
            cooldown_seconds: None,
            actor: None,
            subject: None,
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
