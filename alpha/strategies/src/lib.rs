//! Strategy implementations for the alpha engine.

mod baseline;
mod market_tracker;
pub mod shared_rules;

pub use baseline::snipe_all::{
    LiveSnipeAllConfig, LiveSnipeAllStrategy, SnipeAllConfig, SnipeAllStrategy,
};
pub use market_tracker::{MarketTrackerConfig, MarketTrackerStrategy};
