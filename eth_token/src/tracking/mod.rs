//! Token tracking over processed Rust transactions and blocks.

pub(crate) mod address;
pub mod block_processor;
pub mod builders;
pub mod live_token_retention;
pub mod registry;
pub(crate) mod replay_context;
pub mod reports;
pub mod token_update_router;
pub mod tracked_token_index;

#[cfg(test)]
mod state_replay_tests;
#[cfg(test)]
mod tests;

pub(crate) use address::{
    address_string, hash_string, normalize_address, normalize_address_string, parse_address_lossy,
    same_address_str,
};
pub use block_processor::{BlockTokenProcessor, DEFAULT_TRACKED_TOKEN_INDEX_SIZE};
pub use builders::TokenStateBuilder;
pub use live_token_retention::{
    LivePoolDenomClass, LivePoolRetentionDecision, LiveTokenRetentionDecision,
    LiveTokenRetentionPolicy, LiveTokenRetentionReport, PoolDropReason, TokenDropReason,
};
pub use registry::TokenRegistry;
pub use reports::{TokenBlockUpdateReport, TokenStateUpdateReport, TokenTransactionUpdateError};
pub use token_update_router::ProcessedTokenUpdateRouter;
pub use tracked_token_index::{
    TrackedTokenIndex, TrackedTokenIndexEntry, TrackedTokenIndexUpdate, TrackedTokenStatus,
};
