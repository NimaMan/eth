pub mod current;
pub mod transaction;

use serde::{Deserialize, Serialize};

pub use current::{
    ObservationBlockActivity, ObservationPoolTradingState, TokenPoolCurrentObservation,
};
pub use transaction::{ObservationTransactionSummary, ObservationTransactionType};

pub const ACTIVE_OBSERVATION_TARGET_HORIZONS: [u16; 12] =
    [1, 2, 3, 5, 10, 15, 20, 30, 50, 100, 250, 500];

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenPoolObservationKey {
    pub chain: Option<String>,
    pub token_address: String,
    pub pool_address: String,
    pub denom_address: String,
    pub protocol: String,
}

impl TokenPoolObservationKey {
    pub fn new(
        token_address: impl Into<String>,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        protocol: impl Into<String>,
    ) -> Self {
        Self {
            chain: None,
            token_address: normalize_key(token_address),
            pool_address: normalize_key(pool_address),
            denom_address: normalize_key(denom_address),
            protocol: protocol.into(),
        }
    }

    pub fn with_chain(mut self, chain: impl Into<String>) -> Self {
        self.chain = Some(chain.into());
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenPoolObservationContext {
    pub active_observation_index: u64,
    pub block_number: u64,
    pub timestamp: Option<u64>,
    pub active_reasons: Vec<ActiveObservationReason>,
}

impl TokenPoolObservationContext {
    pub fn new(active_observation_index: u64, block_number: u64, timestamp: Option<u64>) -> Self {
        Self {
            active_observation_index,
            block_number,
            timestamp,
            active_reasons: Vec::new(),
        }
    }

    pub fn with_reason(mut self, reason: ActiveObservationReason) -> Self {
        self.active_reasons.push(reason);
        self
    }

    pub fn with_reasons(
        mut self,
        reasons: impl IntoIterator<Item = ActiveObservationReason>,
    ) -> Self {
        self.active_reasons.extend(reasons);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveObservationReason {
    PoolSwap,
    PoolMint,
    PoolBurn,
    PoolSync,
    PoolLiquidityChange,
    PoolPriceChange,
    TradingStatusChange,
    ScamStatusChange,
    LpApproval,
    LpTransfer,
    TokenTransfer,
    DenomTransfer,
    BuyVolume,
    SellVolume,
    Bribe,
    ControlAddressActivity,
    NetworkActivity,
    Manual,
}

fn normalize_key(value: impl Into<String>) -> String {
    value.into().trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_key_normalizes_addresses() {
        let key = TokenPoolObservationKey::new(" 0xABC ", "0xDEF", "0xC02A", "UNISWAP-V2");

        assert_eq!(key.token_address, "0xabc");
        assert_eq!(key.pool_address, "0xdef");
        assert_eq!(key.denom_address, "0xc02a");
        assert_eq!(key.protocol, "UNISWAP-V2");
    }

    #[test]
    fn active_horizons_keep_near_future_detail() {
        assert_eq!(
            ACTIVE_OBSERVATION_TARGET_HORIZONS,
            [1, 2, 3, 5, 10, 15, 20, 30, 50, 100, 250, 500]
        );
    }
}
