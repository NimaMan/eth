/// Table implementations for the analytics database
///
/// Each table module provides specialized storage and retrieval
/// for different types of analytics data.
pub mod address_index;
pub mod address_metrics;
pub mod mempool_tx_arrivals;
pub mod pools;
pub mod tokens;
pub mod trades;

// Re-export table interfaces
pub use address_index::AddressIndex;
pub use address_metrics::AddressMetricsTable;
pub use mempool_tx_arrivals::MempoolTxArrivalTable;
pub use pools::PoolsTable;
pub use tokens::TokensTable;
pub use trades::TradesTable;
