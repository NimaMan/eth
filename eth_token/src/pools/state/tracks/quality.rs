use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EvidenceQualityTrack {
    pub evidence_count: usize,
    pub confidence: EvidenceConfidence,
    pub event_backed: bool,
    pub simulation_backed: bool,
    pub trace_backed: bool,
    pub state_read_backed: bool,
    pub custody_backed: bool,
    pub evidence: Vec<EvidenceRef>,
}

impl EvidenceQualityTrack {
    pub fn from_evidence_refs(evidence: Vec<EvidenceRef>) -> Self {
        let mut confidence = EvidenceConfidence::Unknown;
        let mut event_backed = false;
        let mut simulation_backed = false;
        let mut trace_backed = false;
        let mut state_read_backed = false;
        let mut custody_backed = false;

        for item in &evidence {
            confidence = confidence.max(item.confidence);
            match item.source {
                EvidenceSourceKind::PoolEvent | EvidenceSourceKind::ReserveTracker => {
                    event_backed = true;
                }
                EvidenceSourceKind::Simulation => simulation_backed = true,
                EvidenceSourceKind::Trace => trace_backed = true,
                EvidenceSourceKind::StateRead => state_read_backed = true,
                EvidenceSourceKind::CustodyFinding => custody_backed = true,
                EvidenceSourceKind::BasePoolProjection
                | EvidenceSourceKind::PoolStateFlags
                | EvidenceSourceKind::Classification
                | EvidenceSourceKind::Fixture => {}
            }
        }

        Self {
            evidence_count: evidence.len(),
            confidence,
            event_backed,
            simulation_backed,
            trace_backed,
            state_read_backed,
            custody_backed,
            evidence,
        }
    }
}
