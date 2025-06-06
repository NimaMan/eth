/*
 * Ethereum Mempool Processor Library
 * 
 * This library provides components for processing Ethereum mempool transactions:
 * - Fetching transactions from an Ethereum node
 * - Filtering transactions based on criteria (value, watched addresses)
 * - Alerting on interesting transactions
 * - Publishing alerts via ZeroMQ
 * - Simulating transactions to detect state changes and suspicious activity
 * - Subscribing to pool level updates from Python
 * - Detecting potential scam transactions by analyzing their effects on pools
 * - Validation testing infrastructure for comparing Rust and Python implementations
 */

// Common utilities
pub mod common;

pub mod mempool_processor; 
pub mod tx_simulator;

// Adding the new pool_subscriber module
pub mod pool_subscriber;

// Adding the scam detection module
pub mod scam_detection;

// Adding the validation testing module
pub mod validation_testing;

// Re-export key components if necessary, or keep them encapsulated for now
// pub use mempool_processor::*;
// pub use tx_simulator::*; 