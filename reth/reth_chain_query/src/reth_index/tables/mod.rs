/// Table implementations for the analytics database
/// 
/// Each table module provides specialized storage and retrieval
/// for different types of analytics data.

pub mod address_index;
pub mod trades;
pub mod address_metrics;
pub mod tokens;
pub mod pools;

// Re-export table interfaces
pub use address_index::AddressIndex;
pub use trades::TradesTable;
pub use address_metrics::AddressMetricsTable;
pub use tokens::TokensTable;
pub use pools::PoolsTable;