//! Alpha11 production strategy.
//!
//! Alpha11 is a config preset over the core [`crate::core`] engine — it adds no
//! trait-level logic. It is `config.rs` (constants), `variants.rs` (the
//! sub-strategies as complete resolved specs), and `factory.rs` (assemble the
//! variant set). Live and backtest both resolve alpha11 through the factory, so
//! the variant set is identical in every mode.

pub mod config;
pub mod factory;
pub mod variants;

pub use config::{
    BUY_WEI, ENTRY_INIT_MAX_AGE_BLOCKS, ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL,
    HOLD15_STRATEGY_NAME, HOLD16_ALL_POOLS_STRATEGY_NAME, HOLD16_STRATEGY_NAME,
    HOLD3_VALIDATION_STRATEGY_NAME, HOLD_SWEEP_SET_NAME, INITIAL_ENTRY_BANKROLL_ETH,
    LIVE_VALIDATION_ENTRY_BANKROLL_ETH, LIVE_VALIDATION_MAX_ENTRY_POOLS,
    LP_APPROVAL_EXIT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS, MIN_LIQUIDITY_ETH, MIN_LIQUIDITY_USD,
    STRATEGY_IMPL,
};
