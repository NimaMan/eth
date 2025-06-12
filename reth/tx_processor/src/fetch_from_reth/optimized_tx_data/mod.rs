//! Optimized Transaction Data Retrieval
//! 
//! This module provides performance-optimized transaction data retrieval by intelligently
//! choosing between database-only access (fast) and simulation (complete but slower).
//!
//! ## Performance Characteristics
//! - **Database Only**: ~0.004ms (4,215x faster with provider reuse)
//! - **Database + Simulation**: ~80-800ms (complete data)
//! - **Provider Creation**: ~18ms (one-time cost)
//!
//! ## Usage Examples
//! ```bash
//! # Basic transaction data (fast)
//! cargo run --bin optimized_tx_basic <tx_hash>
//! 
//! # Smart auto-detection
//! cargo run --bin optimized_tx_smart <tx_hash>
//! 
//! # Performance comparison
//! cargo run --bin optimized_tx_comparison <tx_hash>
//! 
//! # Database connection optimization demonstration
//! cargo run --bin optimized_tx_db_connection <tx_hash>
//! 
//! # Comprehensive provider optimization (all modes)
//! cargo run --bin optimized_tx_comprehensive <tx_hash>
//! ```
//!
//! ## Performance Optimization Guide
//!
//! ### For Single Queries
//! Use the standard API functions:
//! ```rust,no_run
//! let basic_data = get_basic_transaction_data(tx_hash, options).await?;
//! let smart_data = get_smart_transaction_data(tx_hash, options).await?;
//! let complete_data = get_full_transaction_analysis(tx_hash, options).await?;
//! ```
//!
//! ### For Batch Processing (High Performance)
//! Create provider once and reuse:
//! ```rust,no_run
//! use revm_tx_simulator_lib::fetch_from_reth::RethDatabaseProvider;
//! 
//! // Create provider once (750ms cost)
//! let provider = RethDatabaseProvider::new(datadir)?;
//! 
//! // Use for many queries (0.03ms each)
//! for tx_hash in transaction_hashes {
//!     let data = get_basic_transaction_data_with_provider(
//!         tx_hash, options.clone(), Some(&provider)
//!     ).await?;
//! }
//! ```

pub mod api;
pub mod heuristics;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export main API
pub use api::{
    get_basic_transaction_data,
    get_basic_transaction_data_with_provider,
    get_smart_transaction_data,
    get_smart_transaction_data_with_provider,
    get_full_transaction_analysis,
    get_full_transaction_analysis_with_provider,
    get_transaction_optimization_advice,
};

pub use types::{
    BasicTxData,
    SmartTxData, 
    FullTxData,
    DataLevel,
    TransactionType,
    PerformanceMetrics,
    TransactionDataOptions,
};

pub use heuristics::{
    detect_transaction_type,
    needs_internal_transfers,
    should_use_simulation,
};