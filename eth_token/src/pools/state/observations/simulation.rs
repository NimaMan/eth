use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationDirection {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationResult {
    Success,
    Failure,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TradeSimulationObservation {
    pub direction: SimulationDirection,
    pub result: SimulationResult,
    pub raw_can_execute: bool,
    pub effective_can_execute: bool,
    pub tax_bps: Option<f64>,
    pub failure_reason: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl TradeSimulationObservation {
    pub fn new(direction: SimulationDirection, result: SimulationResult) -> Self {
        let can_execute = result == SimulationResult::Success;
        Self {
            direction,
            result,
            raw_can_execute: can_execute,
            effective_can_execute: can_execute,
            tax_bps: None,
            failure_reason: None,
            block_number: None,
            tx_hash: None,
            evidence: EvidenceRef::new(EvidenceSourceKind::Simulation, EvidenceConfidence::High),
        }
    }
}
