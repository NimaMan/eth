/// Mempool Processor Library
/// 
/// High-performance Ethereum mempool monitoring and transaction analysis system.
/// Provides real-time transaction simulation, state change detection, and market event detection.

pub mod mempool_fetcher;
// pub mod tx_simulator; // Temporarily disabled for migration
pub mod token_tracking;
// pub mod signal_engine; // Temporarily disabled for migration
pub mod common;
pub mod config;
pub mod database;
pub mod token_parameter_extraction;
pub mod publishers;

// Re-export commonly used types
pub use mempool_fetcher::{FullTransactionIpcClient, FullTransaction, IpcClientStats};

