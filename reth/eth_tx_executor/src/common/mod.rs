//! Common Module
//! 
//! Shared types, utilities, and configurations used across all modules.
//! This module provides the foundation for consistent error handling,
//! type definitions, and utility functions throughout the system.

pub mod constants;
pub mod errors;
pub mod types;
pub mod utils;

// Re-export commonly used items
pub use constants::{addresses, limits, timing};
pub use errors::{KartalError, ErrorContext};
pub use types::{
    Action, ChainId, GasPrice, PerformanceMetrics, PoolAddress, Priority, 
    Result, Timestamp, TokenAddress, TokenAmount, TransactionStatus, TxHash,
};
pub use utils::{
    apply_slippage, calculate_price_impact, current_timestamp, format_token_amount,
    format_wei_to_eth, gwei_to_wei, wei_to_gwei,
};