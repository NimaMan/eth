//! Shared live-state contracts for Ethereum alpha components.
//!
//! This crate owns snapshot schemas, retention policy, and reader/writer
//! traits. It intentionally does not own block processing, token mutation,
//! mempool processing, cache clients, or trading runtime orchestration.

pub mod block;
pub mod chain_state;
pub mod error;
pub mod retention;
pub mod store;
pub mod token;

pub type BlockNumber = u64;
pub type ChainId = u64;
pub type TimestampUnixSecs = u64;

pub use block::BlockReadyNotification;
pub use chain_state::{ChainStateCodec, ChainStateSnapshotStats, EncodedChainStateSnapshot};
pub use error::{LiveStateError, Result};
pub use retention::RetentionPolicy;
pub use store::{InMemoryLiveStateStore, LiveStateReader, LiveStateWriter, SnapshotWriteOptions};
pub use token::{
    PoolSnapshot, TokenControl, TokenCreation, TokenLatestBlock, TokenLifecycle, TokenMetadata,
    TokenSnapshot, TokenSnapshotIndexEntry, TokenStatus, TokenTransferSummary,
};
