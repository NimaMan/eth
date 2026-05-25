//! Alpha11 production strategy family.
//!
//! Alpha11 owns deployable strategy defaults and live presets. It composes the
//! reusable `baseline::snipe_all` implementation instead of storing product
//! defaults in the baseline module.

mod config;
pub mod live;
pub mod presets;
mod strategy;

pub use config::{
    initial_entry_bankroll_wei, Alpha11Config, BUY_WEI, ENTRY_INIT_MAX_AGE_BLOCKS,
    ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL, HOLD15_STRATEGY_NAME, HOLD16_ALL_POOLS_STRATEGY_NAME,
    HOLD16_STRATEGY_NAME, HOLD3_VALIDATION_STRATEGY_NAME, HOLD_SWEEP_SET_NAME,
    INITIAL_ENTRY_BANKROLL_ETH, LIVE_VALIDATION_ENTRY_BANKROLL_ETH,
    LIVE_VALIDATION_MAX_ENTRY_POOLS, LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS,
    MIN_LIQUIDITY_ETH, MIN_LIQUIDITY_USD, STRATEGY_IMPL,
};
pub use live::{LiveAlpha11Config, LiveAlpha11Strategy};
pub use strategy::Alpha11Strategy;
