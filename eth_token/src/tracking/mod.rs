//! Token tracking over processed Rust transactions and blocks.

pub(crate) mod address;
pub mod block_processor;
pub mod builders;
pub mod index;
pub mod registry;
pub(crate) mod replay_context;
pub mod reports;
pub mod retention;
pub mod transaction_applier;

#[cfg(test)]
mod tests;

pub(crate) use address::{
    address_string, hash_string, normalize_address, normalize_address_string, parse_address_lossy,
    same_address_str,
};
pub use block_processor::{BlockTokenProcessor, DEFAULT_TRACKED_TOKEN_INDEX_SIZE};
pub use builders::TokenStateBuilder;
pub use index::{
    TrackedTokenIndex, TrackedTokenIndexEntry, TrackedTokenIndexUpdate, TrackedTokenStatus,
};
pub use registry::TokenRegistry;
pub use reports::{TokenBlockUpdateReport, TokenStateUpdateReport, TokenTransactionUpdateError};
pub use retention::{
    LivePoolDenomClass, LivePoolRetentionDecision, LiveTokenRetentionDecision,
    LiveTokenRetentionPolicy, LiveTokenRetentionReport, PoolDropReason, TokenDropReason,
};
pub use transaction_applier::{ProcessedTokenUpdateRouter, TokenTransactionApplier};
