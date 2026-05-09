pub mod address;
pub mod cache;
/// Processed Transaction Provider
///
/// This module provides different ways to obtain ProcessedTransaction objects:
/// 1. From CallData (simulation)
/// 2. From transaction hash (load from DB)
///
/// Similar to Python's different entry points for getting processed transactions
pub mod core;
pub mod provider;
pub mod token;

pub use address::AddressProcessedTxProvider;
pub use cache::{
    processed_block_trace_config_hash, ProcessedBlockCacheKey, ProcessedBlockCacheStore,
};
pub use provider::ProcessedTxProvider;
pub use token::TokenProcessedTxProvider;
