/// Database Module
/// 
/// This module provides database connectivity and utilities for the mempool processor.
/// It includes connection management, query execution, and specialized logging functionality.

pub mod mempool_timestamp_tracker;
pub mod trading_signal_writer;
pub mod liquidity_removal_signal_writer;
pub mod lp_approval_signal_writer;
pub mod tax_signal_writer;
pub mod scam_signal_writer;
pub mod unified_signal_writer;

pub use mempool_timestamp_tracker::{MempoolTimestampTracker, TrackerConfig};
pub use trading_signal_writer::{TradingSignalWriter, TradingSignalRecord, WriterConfig as SignalWriterConfig};
pub use liquidity_removal_signal_writer::{LiquidityRemovalSignalWriter, LiquidityRemovalSignalRecord};
pub use lp_approval_signal_writer::{LpApprovalSignalWriter, LpApprovalSignalRecord};
pub use tax_signal_writer::{TaxSignalWriter, TaxSignalRecord};
pub use scam_signal_writer::{ScamSignalWriter, ScamSignalRecord, ScamType};
pub use unified_signal_writer::UnifiedSignalWriter;

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
