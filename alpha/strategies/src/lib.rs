//! Strategy implementations for the alpha engine.

pub mod alpha11;
mod baseline;
mod market_tracker;
pub mod shared_rules;

pub use alpha11::{
    Alpha11Config, Alpha11Strategy, LiveAlpha11Config, LiveAlpha11Strategy,
    HOLD15_STRATEGY_NAME as ALPHA11_HOLD15_STRATEGY_NAME,
    HOLD3_VALIDATION_STRATEGY_NAME as ALPHA11_HOLD3_VALIDATION_STRATEGY_NAME,
    HOLD_SWEEP_SET_NAME as ALPHA11_HOLD_SWEEP_SET_NAME, STRATEGY_IMPL as ALPHA11_STRATEGY_IMPL,
};
pub use baseline::snipe_all::{
    LiveSnipeAllConfig, LiveSnipeAllStrategy, SnipeAllConfig, SnipeAllStrategy,
};
pub use market_tracker::{MarketTrackerConfig, MarketTrackerStrategy};
