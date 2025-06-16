/*
 * Ethereum Transaction Simulator Module
 * 
 * This module provides fast transaction simulation using debug_traceCall RPC.
 * 
 * Primary Components:
 * - debug_traceCall: Fast RPC-based simulation (production choice, ~5ms)
 * - State diff calculation: Tracks ETH and token balance changes
 * 
 * The module tracks state changes for scam detection and MEV analysis.
 */

// Core simulation components
pub mod debug_tracecall_simulator;             // Fast RPC-based simulator
pub mod debug_tracecall_state_diff_calculator; // State diff calculator with WETH=ETH logic

// State tracking and analysis
pub mod state_diff_types;                      // Core state change type definitions only

// Legacy components (to be removed)
pub mod simulator;                              // REVM-based full EVM simulator (DEPRECATED)
pub mod simulator_wrapper;                      // Runtime switching between simulators (DEPRECATED)
pub mod state_cache;                           // Production cache with full metadata (DEPRECATED)
pub mod conversions;                           // Type conversions between formats (DEPRECATED)

// Test modules
#[cfg(test)]
pub mod tests;

// Re-export key types and functions
pub use debug_tracecall_simulator::DebugTraceCallSimulator;
pub use debug_tracecall_state_diff_calculator::DebugTraceCallStateDiffCalculator;
pub use state_diff_types::{StateDiffTracker, StateChange, MempoolStateDiff};

// Legacy exports (to be removed)
pub use simulator::TransactionSimulator;
pub use state_cache::{StateCache, AggregatedStateChange};
pub use conversions::transaction_view_to_revm_tx_env;
pub use simulator_wrapper::SimulatorWrapper; 