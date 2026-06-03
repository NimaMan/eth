use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::state::evidence::EvidenceRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketFamily {
    ConstantProductV2,
    ConcentratedLiquidityV3,
    SingletonV4,
    BalancerWeighted,
    CurveStable,
    Unknown,
}

impl Default for MarketFamily {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenOrientation {
    Token0IsDenom,
    Token1IsDenom,
    Unknown,
}

impl Default for TokenOrientation {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MarketStructureTrack {
    pub protocol: String,
    pub family: MarketFamily,
    pub token_orientation: TokenOrientation,
    pub fee_tier: Option<u32>,
    pub tick_spacing: Option<i32>,
    pub v4_pool_key: Option<String>,
    pub hooks: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

impl MarketStructureTrack {
    pub fn from_base_pool(base: &BasePool) -> Self {
        Self {
            protocol: base.identity.protocol.clone(),
            family: family_for_protocol(&base.identity.protocol),
            token_orientation: match base.config.token1_is_denom {
                Some(true) => TokenOrientation::Token1IsDenom,
                Some(false) => TokenOrientation::Token0IsDenom,
                None => TokenOrientation::Unknown,
            },
            fee_tier: None,
            tick_spacing: None,
            v4_pool_key: None,
            hooks: None,
            evidence: vec![EvidenceRef::base_projection()],
        }
    }
}

fn family_for_protocol(protocol: &str) -> MarketFamily {
    let normalized = protocol.trim().to_ascii_lowercase();
    if normalized.contains("v4") {
        MarketFamily::SingletonV4
    } else if normalized.contains("v3") {
        MarketFamily::ConcentratedLiquidityV3
    } else if normalized.contains("balancer") {
        MarketFamily::BalancerWeighted
    } else if normalized.contains("curve") {
        MarketFamily::CurveStable
    } else if normalized.contains("v2")
        || normalized.contains("uniswap")
        || normalized.contains("sushiswap")
        || normalized.contains("pancakeswap")
    {
        MarketFamily::ConstantProductV2
    } else {
        MarketFamily::Unknown
    }
}
