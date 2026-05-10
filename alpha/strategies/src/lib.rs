//! Strategy implementations for the alpha engine.

mod baseline;
mod market_tracker;
mod shared_rules;

pub use baseline::snipe_all::{SnipeAllConfig, SnipeAllStrategy};
pub use market_tracker::{MarketTrackerConfig, MarketTrackerStrategy};
