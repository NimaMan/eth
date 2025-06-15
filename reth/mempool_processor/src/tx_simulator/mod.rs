/*
 * Ethereum Transaction Simulator Module
 * 
 * This module provides multiple approaches for simulating Ethereum transactions:
 * 
 * Simulators:
 * - REVM-based: Full EVM execution locally (comprehensive, ~40-50ms)
 * - debug_traceCall: Fast RPC-based simulation (production choice, ~5ms)
 * 
 * Caching:
 * - state_cache: Production cache with metadata and aggregation
 * 
 * The module tracks state changes, detects suspicious activities,
 * and provides efficient caching for real-time mempool analysis.
 */

// Core simulation components
pub mod simulator;                              // REVM-based full EVM simulator
pub mod debug_tracecall_simulator;             // Fast RPC-based simulator
pub mod simulator_wrapper;                      // Runtime switching between simulators

// State tracking and analysis
pub mod state_diff;                            // Core state change types
pub mod debug_tracecall_state_diff_calculator; // State diff calculator with WETH=ETH logic

// Caching implementation
pub mod state_cache;                           // Production cache with full metadata

// Utilities
pub mod conversions;                           // Type conversions between formats

// Test modules
#[cfg(test)]
pub mod tests;

// Re-export key types and functions
pub use simulator::TransactionSimulator;
pub use state_diff::{StateDiffTracker, StateChange, MempoolStateDiff};
pub use state_cache::{StateCache, AggregatedStateChange};
pub use conversions::transaction_view_to_revm_tx_env;
pub use debug_tracecall_simulator::DebugTraceCallSimulator;
pub use debug_tracecall_state_diff_calculator::DebugTraceCallStateDiffCalculator;
pub use simulator_wrapper::SimulatorWrapper; 