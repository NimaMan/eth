use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolEventKind {
    Sync,
    Swap,
    Mint,
    Burn,
    Transfer,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolEventObservation {
    pub kind: PoolEventKind,
    pub pool_address: String,
    pub block_number: u64,
    pub tx_hash: String,
    pub log_index: Option<u64>,
    pub actor: Option<String>,
    pub token_delta: Option<f64>,
    pub denom_delta: Option<f64>,
    pub evidence: EvidenceRef,
}

impl PoolEventObservation {
    pub fn new(
        kind: PoolEventKind,
        pool_address: impl Into<String>,
        block_number: u64,
        tx_hash: impl Into<String>,
    ) -> Self {
        let tx_hash = tx_hash.into();
        Self {
            kind,
            pool_address: pool_address.into(),
            block_number,
            tx_hash: tx_hash.clone(),
            log_index: None,
            actor: None,
            token_delta: None,
            denom_delta: None,
            evidence: EvidenceRef::new(EvidenceSourceKind::PoolEvent, EvidenceConfidence::High)
                .at_block(Some(block_number))
                .with_tx(Some(tx_hash)),
        }
    }
}
