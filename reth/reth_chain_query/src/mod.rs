/// ChainQuery module for direct blockchain database queries
/// 
/// Provides efficient access to blockchain state without RPC calls,
/// reading directly from Reth's database.

pub mod query_engine;

pub use query_engine::ChainQuery;