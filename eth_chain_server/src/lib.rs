#![recursion_limit = "512"]

pub mod app;
pub mod error;
pub mod http;
pub mod live;
pub mod live_frames;
pub mod live_simulation;
pub mod memory;
pub mod prices;
pub mod ranges;
pub mod read_models;
pub mod recent_blocks;
pub mod simulation;
pub mod stores;
pub mod token_analytics;

// Compatibility modules for existing examples/tests and any downstream callers.
pub mod alpha_trading {
    pub use crate::stores::alpha_trading::*;
}

pub mod config {
    pub use crate::app::config::*;
}

pub mod mempool_signals {
    pub use crate::stores::mempool_signals::*;
}

pub mod range_indexer {
    pub use crate::ranges::*;
    pub use crate::ranges::{manager, pipeline, progress, types};
}

pub mod server {
    pub use crate::http::*;

    pub mod state {
        pub use crate::app::state::*;
    }
}

pub mod views {
    pub use crate::read_models::{
        activity, cache, error, gas_rank, live, ops, pool, price, risk_atlas, run,
        scammer_analytics, simulation, strategy, surface, token, token_analytics,
    };
}

pub use app::config::ChainServerConfig;
pub use live::LiveTracker;
pub use ranges::RangeIndexManager;
pub use tx_processor::ProcessedBlockDiskCacheStore;
