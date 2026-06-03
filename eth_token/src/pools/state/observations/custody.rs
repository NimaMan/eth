use serde::{Deserialize, Serialize};

use crate::custody::{CustodyCapability, CustodyFinding, CustodyState};
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustodyObservation {
    pub capability: CustodyCapability,
    pub state: CustodyState,
    pub actor: Option<String>,
    pub affected_holder: Option<String>,
    pub amount: Option<f64>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl CustodyObservation {
    pub fn from_finding(finding: &CustodyFinding) -> Self {
        Self {
            capability: finding.capability,
            state: finding.state,
            actor: None,
            affected_holder: None,
            amount: None,
            block_number: finding.block_number,
            tx_hash: None,
            evidence: EvidenceRef::new(
                EvidenceSourceKind::CustodyFinding,
                match finding.state {
                    CustodyState::Latent => EvidenceConfidence::Medium,
                    CustodyState::Realized => EvidenceConfidence::Confirmed,
                },
            )
            .at_block(finding.block_number),
        }
    }
}
