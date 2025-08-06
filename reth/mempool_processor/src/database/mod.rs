/// Database Module
/// 
/// This module provides database connectivity and utilities for the mempool processor.
/// It includes connection management, query execution, and specialized logging functionality.

pub mod scam_prediction_writer;
pub mod trading_event_writer;
pub mod mempool_timestamp_tracker;

pub use scam_prediction_writer::ScamPredictionWriter;
pub use trading_event_writer::{
    TradingEventWriter, 
    TradingEnabledEvent, 
    CreatorActionEvent,
    MempoolSignal
};
pub use mempool_timestamp_tracker::{MempoolTimestampTracker, TrackerConfig};

/// Default database connection parameters
pub const DEFAULT_DB_HOST: &str = "localhost";
pub const DEFAULT_DB_PORT: u16 = 5432;
pub const DEFAULT_DB_NAME: &str = "eth_db";
pub const DEFAULT_DB_USER: &str = "postgres";
pub const DEFAULT_DB_PASSWORD: &str = "postgres";

/// Get default database URL for sqlx connections
pub fn get_default_database_url() -> String {
    format!(
        "postgresql://{}:{}@{}:{}/{}",
        DEFAULT_DB_USER,
        DEFAULT_DB_PASSWORD,
        DEFAULT_DB_HOST,
        DEFAULT_DB_PORT,
        DEFAULT_DB_NAME
    )
}