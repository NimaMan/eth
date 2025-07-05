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
// pub mod reth_direct_simulator;                 // Ultra-fast direct Reth integration (removed)
pub mod reth_simulator_engine;                 // Direct Reth engine bypassing RPC

// State tracking and analysis
pub mod state_diff_types;                      // Core state change type definitions only

// Re-export key types and functions
pub use debug_tracecall_simulator::DebugTraceCallSimulator;
pub use debug_tracecall_state_diff_calculator::DebugTraceCallStateDiffCalculator;
// pub use reth_direct_simulator::RethDirectTxSimulator;
pub use state_diff_types::{StateDiffTracker, StateChange, MempoolStateDiff}; 