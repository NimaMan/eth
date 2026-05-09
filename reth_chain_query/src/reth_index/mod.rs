/// RethIndex: High-Performance Indexing Layer for Reth
///
/// RethIndex provides a complementary MDBX database that enables fast
/// entity-centric queries and pre-computed aggregations. While Reth stores
/// data optimized for blockchain operation (sequential), RethIndex provides:
/// - Address → block participation indexes
/// - Trading history and PnL tracking
/// - Pre-computed address metrics
/// - Token and pool metadata caching
/// - Mempool timing analytics
///
/// See README.md for comprehensive architecture documentation.
pub mod database;
pub mod models;
pub mod reader;
pub mod tables;
pub mod writers;

// Re-export main types
pub use database::RethIndexDB;
pub use models::{AddressMetrics, PoolData, TokenMetadata, TradeData};
pub use reader::RethIndexReader;
pub use writers::{
    address_block_participation_writer::{AddressBlockParticipationWriter, AddressParticipation},
    mempool_arrival_writer::MempoolArrivalWriter,
};

// Re-export table interfaces
pub use tables::{
    address_block_participation::{AddressBlockParticipationIndex, ParticipationBlockNumber},
    address_metrics::AddressMetricsTable,
    pools::PoolsTable,
    tokens::TokensTable,
    trades::TradesTable,
};

// Re-export result type
pub use eyre::Result;
