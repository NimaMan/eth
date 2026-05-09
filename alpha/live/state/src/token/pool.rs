use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

use crate::{BlockNumber, TimestampUnixSecs};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolSnapshot {
    pub pool_address: Address,
    pub protocol: String,
    pub denom_address: Option<Address>,
    pub denom_symbol: Option<String>,
    pub token_reserve: Option<String>,
    pub denom_reserve: Option<String>,
    pub price_denom_per_token: Option<String>,
    pub total_liquidity: Option<String>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<BlockNumber>,
    pub trading_enabled_tx: Option<B256>,
    pub pool_id: Option<String>,
    pub fee_bps: Option<u32>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub latest_block_number: Option<BlockNumber>,
    pub last_update_unix_secs: Option<TimestampUnixSecs>,
}

impl PoolSnapshot {
    pub fn new(pool_address: Address, protocol: impl Into<String>) -> Self {
        Self {
            pool_address,
            protocol: protocol.into(),
            denom_address: None,
            denom_symbol: None,
            token_reserve: None,
            denom_reserve: None,
            price_denom_per_token: None,
            total_liquidity: None,
            can_buy: false,
            can_sell: false,
            trading_enabled: false,
            trading_enabled_block: None,
            trading_enabled_tx: None,
            pool_id: None,
            fee_bps: None,
            is_scam: false,
            scam_label: None,
            latest_block_number: None,
            last_update_unix_secs: None,
        }
    }
}
