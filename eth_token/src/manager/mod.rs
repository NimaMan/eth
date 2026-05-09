//! Compatibility facade for the old `eth_token::manager` path.
//!
//! New code should import token tracking types from `eth_token::tracking`.

pub use crate::tracking::*;

pub mod block_processor {
    pub use crate::tracking::block_processor::*;
}

pub mod index {
    pub use crate::tracking::tracked_token_index::*;
}

pub mod retention {
    pub use crate::tracking::live_token_retention::*;
}

pub mod token_builder {
    pub use crate::tracking::builders::*;
}

pub mod update_router {
    pub use crate::tracking::token_update_router::*;
}

#[allow(unused_imports)]
pub(crate) mod replay_context {
    pub(crate) use crate::tracking::replay_context::*;
}
