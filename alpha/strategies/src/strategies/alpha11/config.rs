//! Alpha11 strategy constants.
//!
//! Alpha11 is a config preset over the core engine: a fixed entry/exit policy
//! (init-age 5, price-ratio 2, LP gate 30%, lp-approval-exit defer 2, bankroll
//! 0.555 ETH, Uniswap V2 filter, all exit rules on, defer-buy-confirm). The
//! sub-strategy variants differ only in hold horizon (and the all-pools /
//! validation tweaks); see `variants.rs` and `factory.rs`.

pub const STRATEGY_IMPL: &str = "alpha11";
pub const HOLD_SWEEP_SET_NAME: &str = "alpha11-univ2-lp30-pool-update-block-hold-sweep";
pub const HOLD15_STRATEGY_NAME: &str = "alpha11-univ2-lp30-pool-update-block-hold15";
pub const HOLD16_STRATEGY_NAME: &str = "alpha11-univ2-lp30-pool-update-block-hold16";
pub const HOLD16_ALL_POOLS_STRATEGY_NAME: &str = "alpha11-all-pools-lp30-pool-update-block-hold16";
pub const HOLD3_VALIDATION_STRATEGY_NAME: &str =
    "alpha11-univ2-lp30-pool-update-block-hold3-validation";
pub const BUY_WEI: &str = "5000000000000000";
pub const MIN_LIQUIDITY_ETH: &str = "0.5";
pub const MIN_LIQUIDITY_USD: &str = "1000";
pub const ENTRY_INIT_MAX_AGE_BLOCKS: u64 = 5;
pub const ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL: &str = "2";
pub const LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS: u64 = 2;
pub const INITIAL_ENTRY_BANKROLL_ETH: &str = "0.555";
pub const LIVE_VALIDATION_ENTRY_BANKROLL_ETH: &str = "0.01";
pub const LIVE_VALIDATION_MAX_ENTRY_POOLS: usize = 1;
