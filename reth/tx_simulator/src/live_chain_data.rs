//! Re-exports for live chain data components.
//!
//! This module serves as an aggregation point for types and functionalities
//! related to live chain state and data caching, primarily re-exporting
//! components from the `crate::block_context` module. It simplifies access
//! to the live chain cache and data registry within the transaction simulator.
pub mod live_chain_cache {
    pub use crate::block_context::live_chain_cache::*;
}

pub mod live_data_registry {
    pub use crate::block_context::live_data_registry::*;
}

pub use live_data_registry::{keys, ChainStateSnapshot};
