use serde::{Deserialize, Serialize};

use crate::pools::PoolLifecycle;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolMarketFeatures {
    pub pool_creation_block: Option<u64>,
    pub trading_enabled_block: Option<u64>,
    pub pool_age_blocks: Option<u64>,
    pub trading_age_blocks: Option<u64>,
    pub lifecycle: Option<PoolLifecycle>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub economic_sellable: Option<bool>,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tax_bucket: Option<String>,
    pub first_observed_buy_block: Option<u64>,
    pub first_observed_sell_block: Option<u64>,
    pub first_failed_sell_block: Option<u64>,
    pub last_trading_failure_class: Option<String>,
    pub liquidity_removed_as_of: bool,
    pub liquidity_removal_block_as_of: Option<u64>,
    pub scam_mechanism_as_of: Option<String>,
    pub scam_label_as_of: Option<String>,
}

impl PoolMarketFeatures {
    pub fn with_ages(
        pool_creation_block: Option<u64>,
        trading_enabled_block: Option<u64>,
        as_of_block: u64,
    ) -> Self {
        Self {
            pool_creation_block,
            trading_enabled_block,
            pool_age_blocks: pool_creation_block.map(|block| as_of_block.saturating_sub(block)),
            trading_age_blocks: trading_enabled_block
                .map(|block| as_of_block.saturating_sub(block)),
            ..Default::default()
        }
    }
}
