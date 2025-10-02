pub mod canonical_head_cache;
/// Mempool Processor Library
///
/// High-performance Ethereum mempool monitoring and transaction analysis system.
/// Provides real-time transaction simulation, state change detection, and market event detection.
pub mod common;
pub mod config;
pub mod db_writers;
pub mod mempool_fetcher;
pub mod token_tracking;

// Signal processing modules
pub mod arrival_recorder;
pub mod function_detector;
pub mod signal_detector;
pub mod signal_publisher;
pub mod simulator;
pub mod tx_router;
