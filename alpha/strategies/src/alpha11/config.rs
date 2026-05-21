use alloy_primitives::U256;

use crate::baseline::snipe_all::SnipeAllConfig;

pub const STRATEGY_IMPL: &str = "alpha11";
pub const HOLD_SWEEP_SET_NAME: &str = "alpha11-live-univ2-lp30-pool-update-block-hold-sweep";
pub const HOLD15_STRATEGY_NAME: &str = "alpha11-live-univ2-lp30-pool-update-block-hold15";
pub const DEPLOY_HOLD_SWEEP_SET_NAME: &str =
    "alpha11-live-univ2-lp30-price-to-initial-lte1p5-pool-update-block-hold-sweep";
pub const DEPLOY_HOLD15_STRATEGY_NAME: &str =
    "alpha11-live-univ2-lp30-price-to-initial-lte1p5-pool-update-block-hold15";
pub const INITIAL_ENTRY_BANKROLL_ETH: &str = "0.225";
pub const MAX_ENTRY_PRICE_RATIO_TO_INITIAL: &str = "1.5";

const INITIAL_ENTRY_BANKROLL_WEI: u64 = 225_000_000_000_000_000;

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
