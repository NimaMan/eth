use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityObservationKind {
    ReserveUpdate,
    LiquidityDeposit,
    LiquidityRemoval,
    Dust,
    Drain,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiquidityObservation {
    pub kind: LiquidityObservationKind,
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub total_liquidity: f64,
    pub price_denom_per_token: Option<f64>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: EvidenceRef,
}

impl LiquidityObservation {
    pub fn reserve_update(
        token_reserve: f64,
        denom_reserve: f64,
        block_number: Option<u64>,
        tx_hash: Option<String>,
    ) -> Self {
        Self {
            kind: LiquidityObservationKind::ReserveUpdate,
            token_reserve,
            denom_reserve,
            total_liquidity: denom_reserve.max(0.0),
            price_denom_per_token: positive_price(denom_reserve, token_reserve),
            block_number,
            tx_hash: tx_hash.clone(),
            evidence: EvidenceRef::new(
                EvidenceSourceKind::ReserveTracker,
                EvidenceConfidence::High,
            )
            .at_block(block_number)
            .with_tx(tx_hash),
        }
    }
}

fn positive_price(denom_reserve: f64, token_reserve: f64) -> Option<f64> {
    if denom_reserve.is_finite()
        && token_reserve.is_finite()
        && denom_reserve > 0.0
        && token_reserve > 0.0
    {
        Some(denom_reserve / token_reserve)
    } else {
        None
    }
}
