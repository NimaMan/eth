/// PostgreSQL Database Query Module
/// 
/// Provides efficient queries for aggregated Ethereum trading data stored in PostgreSQL.
/// This module complements the blockchain queries by accessing pre-processed analytics data.
/// 
/// # Architecture
/// 
/// - `models`: Rust structs matching the PostgreSQL schema
/// - `queries`: Specialized query modules for different data types
/// - `connection`: Database connection pool management
/// 
/// # Performance
/// 
/// Uses sqlx for zero-copy deserialization and prepared statements.
/// Connection pooling ensures efficient resource usage for concurrent queries.

pub mod models;
pub mod queries;
pub mod connection;

// Re-export main types
pub use connection::PostgresDB;
pub use models::{
    AddressMetrics, Trade, Token, Pool, 
    Transaction, TxParticipant, AddressTransaction, TransactionWithParticipants
};

use eyre::Result;

/// Main PostgreSQL query client
pub struct PostgresQuery {
    db: PostgresDB,
}

impl PostgresQuery {
    /// Create new PostgreSQL query client with connection string
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = PostgresDB::new(database_url).await?;
        Ok(Self { db })
    }
    
    /// Get reference to database connection pool
    pub fn db(&self) -> &PostgresDB {
        &self.db
    }
}