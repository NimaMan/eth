use alloy_primitives::U256;

use crate::baseline::snipe_all::SnipeAllConfig;

pub const STRATEGY_IMPL: &str = "alpha11";
pub const HOLD_SWEEP_SET_NAME: &str = "alpha11-univ2-lp30-pool-update-block-hold-sweep";
pub const HOLD15_STRATEGY_NAME: &str = "alpha11-univ2-lp30-pool-update-block-hold15";
pub const HOLD16_STRATEGY_NAME: &str = "alpha11-univ2-lp30-pool-update-block-hold16";
pub const HOLD16_ALL_POOLS_STRATEGY_NAME: &str = "alpha11-all-pools-lp30-pool-update-block-hold16";
pub const HOLD3_VALIDATION_STRATEGY_NAME: &str =
    "alpha11-univ2-lp30-pool-update-block-hold3-validation";
pub const BUY_WEI: &str = "10000000000000000";
pub const MIN_LIQUIDITY_ETH: &str = "0.5";
pub const MIN_LIQUIDITY_USD: &str = "1000";
pub const ENTRY_INIT_MAX_AGE_BLOCKS: u64 = 5;
pub const ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL: &str = "2";
pub const LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS: u64 = 2;
pub const INITIAL_ENTRY_BANKROLL_ETH: &str = "0.555";
pub const LIVE_VALIDATION_ENTRY_BANKROLL_ETH: &str = "0.01";
pub const LIVE_VALIDATION_MAX_ENTRY_POOLS: usize = 1;

const INITIAL_ENTRY_BANKROLL_WEI: u64 = 555_000_000_000_000_000;

#[derive(Clone, Debug)]
pub struct Alpha11Config {
    pub snipe_all: SnipeAllConfig,
}

impl Alpha11Config {
    pub fn new(snipe_all: SnipeAllConfig) -> Self {
        Self { snipe_all }
    }
}

pub fn initial_entry_bankroll_wei() -> U256 {
    U256::from(INITIAL_ENTRY_BANKROLL_WEI)
}
