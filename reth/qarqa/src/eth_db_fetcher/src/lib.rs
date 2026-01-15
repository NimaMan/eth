//! ETH DB Fetcher - PostgreSQL database access for QARQA
//! 
//! This module provides efficient access to the eth_db PostgreSQL database,
//! which contains indexed blockchain data including:
//! - Address to transaction mappings
//! - Pre-computed address metrics
//! - Token metadata and scam labels
//! - Trading relationships between addresses

pub mod models;
pub mod connection;
pub mod address_fetcher;
pub mod transaction_fetcher;

// Re-export main types
pub use models::*;
pub use connection::{create_pool, DbConfig};
pub use address_fetcher::AddressFetcher;
pub use transaction_fetcher::TransactionFetcher;