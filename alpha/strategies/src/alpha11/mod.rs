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
    initial_entry_bankroll_wei, Alpha11Config, HOLD15_STRATEGY_NAME, HOLD_SWEEP_SET_NAME,
    INITIAL_ENTRY_BANKROLL_ETH, MAX_ENTRY_PRICE_RATIO_TO_INITIAL, STRATEGY_IMPL,
};
pub use live::{LiveAlpha11Config, LiveAlpha11Strategy};
pub use strategy::Alpha11Strategy;
