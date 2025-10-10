pub mod cache;
/// Fundamental Block-Time Conversion System
///
/// Core functionality for bidirectional block ↔ time conversion,
/// time period aggregation, and consistent time handling across
/// Rust and Python interfaces.
///
/// This module provides:
/// - Block number → Timestamp conversion
/// - Timestamp → Block number conversion  
/// - Time period boundaries in blocks
/// - Caching for performance
/// - Integration with both Reth DB and PostgreSQL
pub mod converter;
pub mod periods;

pub use cache::TimestampCache;
pub use converter::{BlockTimeConverter, BlockTimestamp};
pub use periods::{PeriodBoundary, PeriodType, TimePeriod};

use chrono::{DateTime, Utc};

/// Average block time on Ethereum (seconds)
pub const AVERAGE_BLOCK_TIME: u64 = 12;

/// Ethereum mainnet genesis timestamp
pub const ETHEREUM_GENESIS_TIMESTAMP: i64 = 1438269973; // July 30, 2015

/// Estimate timestamp for a block number (fallback when DB unavailable)
pub fn estimate_timestamp(block_number: u64) -> DateTime<Utc> {
    let seconds_since_genesis = block_number * AVERAGE_BLOCK_TIME;
    DateTime::from_timestamp(ETHEREUM_GENESIS_TIMESTAMP + seconds_since_genesis as i64, 0)
        .unwrap_or_else(|| Utc::now())
}

/// Estimate block number for a timestamp (fallback when DB unavailable)
pub fn estimate_block_number(timestamp: DateTime<Utc>) -> u64 {
    let seconds_since_genesis = (timestamp.timestamp() - ETHEREUM_GENESIS_TIMESTAMP).max(0);
    (seconds_since_genesis as u64) / AVERAGE_BLOCK_TIME
}
