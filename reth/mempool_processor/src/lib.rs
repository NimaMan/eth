/// Mempool Processor Library
/// 
/// High-performance Ethereum mempool monitoring and transaction analysis system.
/// Provides real-time transaction simulation, state change detection, and market event detection.

pub mod mempool_fetcher;
pub mod tx_simulator;
pub mod pool_subscriber;
pub mod signal_engine;
pub mod validation_testing;
pub mod common;

// Additional modules
pub mod mempool_scam_detector;