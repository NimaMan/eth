/// RethIndex: High-Performance Indexing Layer for Reth
/// 
/// RethIndex provides a complementary MDBX database that enables fast
/// entity-centric queries and pre-computed aggregations. While Reth stores
/// data optimized for blockchain operation (sequential), RethIndex provides:
/// - Address → Transaction reverse indexes
/// - Trading history and PnL tracking
/// - Pre-computed address metrics
/// - Token and pool metadata caching
/// - Mempool timing analytics
///
/// See README.md for comprehensive architecture documentation.

pub mod database;
pub mod models;
pub mod reader;
pub mod writer;
pub mod tables;

// Re-export main types
pub use database::RethIndexDB;
pub use models::{TradeData, AddressMetrics, TokenMetadata, PoolData};
pub use reader::RethIndexReader;
pub use writer::RethIndexWriter;

// Re-export table interfaces
pub use tables::{
    address_index::AddressIndex,
    trades::TradesTable,
    address_metrics::AddressMetricsTable,
    tokens::TokensTable,
    pools::PoolsTable,
};

// Re-export result type
pub use eyre::Result;