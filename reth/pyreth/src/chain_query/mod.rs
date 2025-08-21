/// ChainQuery module for direct blockchain database queries
/// 
/// Provides efficient access to blockchain state without RPC calls,
/// reading directly from Reth's database.

pub mod query_engine;
pub mod account;
pub mod token;
pub mod storage;
pub mod block;
// pub mod block_fetcher; // Temporarily disabled for testing

pub use query_engine::ChainQuery;

// Re-export commonly used types
pub use account::{AccountInfo, AccountQuery};
pub use token::{TokenInfo, TokenQuery};
pub use storage::StorageQuery;
pub use block::{BlockInfo, BlockQuery};
// pub use block_fetcher::{BlockDataFetcher, ProcessedBlock, ProcessedBlockTransaction};