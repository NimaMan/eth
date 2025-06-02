/*
 * Ethereum Transaction Simulator Module
 * 
 * This module provides functionality to:
 * 1. Simulate Ethereum transactions using REVM
 * 2. Track state changes (balance/storage) from transactions
 * 3. Cache simulation results for efficient access
 * 4. Detect suspicious activities like liquidity removal
 */

// Our component modules
pub mod simulator;
pub mod state_diff;
pub mod state_cache;
pub mod mempool_state_cache;
pub mod comprehensive_state_diff;
pub mod conversions;

// Re-export key types and functions
pub use simulator::TransactionSimulator;
pub use state_diff::{StateDiffTracker, StateChange};
pub use state_cache::{StateCache, AggregatedStateChange};
pub use mempool_state_cache::*; 
pub use comprehensive_state_diff::*; 
pub use conversions::transaction_view_to_revm_tx_env; 