use serde::{Deserialize, Serialize};

use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityClass {
    Unknown,
    Empty,
    Dust,
    Meaningful,
    Removed,
}

impl Default for LiquidityClass {
    fn default() -> Self {
        Self::Unknown
    }
}

impl LiquidityClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Empty => "empty",
            Self::Dust => "dust",
            Self::Meaningful => "meaningful",
            Self::Removed => "removed",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LiquidityTrack {
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub total_liquidity: f64,
    pub price_denom_per_token: f64,
    pub has_reserves: bool,
    pub has_effective_liquidity: bool,
    pub legacy_liquidity_removal_flag: bool,
    pub reserve_liquidity_removed: bool,
    pub class: LiquidityClass,
    pub latest_block_number: Option<u64>,
    pub evidence: Vec<EvidenceRef>,
}

impl LiquidityTrack {
    pub fn from_flags(flags: &PoolStateFlags) -> Self {
        let class = if flags.liquidity.reserve_liquidity_removed {
            LiquidityClass::Removed
        } else if !flags.liquidity.has_reserves {
            LiquidityClass::Empty
        } else if flags.liquidity.has_effective_liquidity {
            LiquidityClass::Meaningful
        } else {
            LiquidityClass::Dust
        };

        Self {
            token_reserve: flags.liquidity.token_reserve,
            denom_reserve: flags.liquidity.denom_reserve,
            total_liquidity: flags.liquidity.total_liquidity,
            price_denom_per_token: flags.liquidity.price_denom_per_token,
            has_reserves: flags.liquidity.has_reserves,
            has_effective_liquidity: flags.liquidity.has_effective_liquidity,
            legacy_liquidity_removal_flag: flags.liquidity.legacy_liquidity_removal_flag,
            reserve_liquidity_removed: flags.liquidity.reserve_liquidity_removed,
            class,
            latest_block_number: flags.latest_block_number,
            evidence: vec![EvidenceRef::new(
                EvidenceSourceKind::ReserveTracker,
                EvidenceConfidence::High,
            )
            .at_block(flags.latest_block_number)],
        }
    }
}
