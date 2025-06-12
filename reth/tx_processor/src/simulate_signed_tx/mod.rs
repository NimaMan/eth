//! Transaction simulation module using REVM
//! 
//! This module provides comprehensive transaction simulation capabilities
//! including internal transfer extraction, call tracing, and state changes.
//!
//! ## Examples
//!
//! ### Basic Usage
//! ```bash
//! # Simulate transaction fetched via RPC
//! cargo run --bin simulate_basic_usage
//! cargo run --bin simulate_by_hash <tx_hash>
//! ```
//!
//! ### Database Integration  
//! ```bash
//! # Simulate transaction loaded from Reth database (hybrid approach)
//! cargo run --bin simulate_from_db [tx_hash]
//! ```
//!
//! ### Advanced Usage
//! ```bash
//! # Call tracing and internal transfers
//! cargo run --bin advanced_tracing
//! cargo run --bin extract_internal_transfers
//! 
//! # Raw transaction bytes
//! cargo run --bin simulate_raw_bytes
//! ```

pub mod simulation_core;
pub mod call_tracer;
pub mod internal_transfer_tracker;
pub mod spec_utils;
// pub mod signed_tx_simulator;  // Temporarily disabled due to compilation errors
// pub mod simple_signed_tx_simulator;  // Temporarily disabled due to compilation errors
pub mod lib;

// Re-export main types and functions
pub use simulation_core::{
    simulate_transaction, SimCacheDB, ExecutionResultType,
};

// Re-export the main entry point for signed transaction simulation
// pub use signed_tx_simulator::{
//     simulate_signed_transaction,
//     simulate_signed_transaction_with_tracer,
// };

// Re-export the simple signed transaction simulator
// pub use simple_signed_tx_simulator::{
//     simulate_signed_tx_by_hash,
// };

// Re-export the main clean API (from lib.rs)
pub use self::lib::{
    simulate_signed_tx,
    simulate_signed_tx_bytes,
};

// Placeholder types that need to be implemented
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub trace_calls: bool,
    pub calculate_state_diff: bool,
    pub gas_limit: u64,
    pub timeout: std::time::Duration,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            trace_calls: true,
            calculate_state_diff: true,
            gas_limit: 30_000_000,
            timeout: std::time::Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockEnv {
    pub number: u64,
    pub timestamp: u64,
    pub basefee: alloy_primitives::U256,
    pub gas_limit: u64,
    pub coinbase: alloy_primitives::Address,
}

#[derive(Debug)]
pub struct SimulationOutput {
    pub success: bool,
    pub gas_used: u64,
    pub return_data: alloy_primitives::Bytes,
    pub revert_reason: Option<String>,
    pub internal_transfers: Vec<InternalTransfer>,
    pub call_trace: Option<CallTrace>,
    pub state_changes: std::collections::HashMap<alloy_primitives::Address, StateChange>,
    pub logs: Vec<alloy_rpc_types::Log>,
}

#[derive(Debug)]
pub struct StateChange {
    pub balance_change: I256,
    pub nonce_change: i32,
    pub storage_changes: std::collections::HashMap<alloy_primitives::U256, StorageChange>,
    pub code_change: Option<alloy_primitives::Bytes>,
}

#[derive(Debug)]
pub struct StorageChange {
    pub before: alloy_primitives::U256,
    pub after: alloy_primitives::U256,
}

#[derive(Debug)]
pub enum SimulationError {
    Reverted { reason: String },
    OutOfGas,
    InvalidTransaction(String),
    Timeout,
    StateNotAvailable(u64),
}

impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimulationError::Reverted { reason } => write!(f, "Transaction reverted: {}", reason),
            SimulationError::OutOfGas => write!(f, "Insufficient gas"),
            SimulationError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            SimulationError::Timeout => write!(f, "Simulation timeout"),
            SimulationError::StateNotAvailable(block) => write!(f, "State not available for block {}", block),
        }
    }
}

impl std::error::Error for SimulationError {}

// Placeholder for i256 type
pub type I256 = i128;

// Re-export from internal modules
pub use call_tracer::{
    CallTracer, CallTrace, CallType, InternalTransfer,
};

pub use internal_transfer_tracker::{
    InternalTransferTracker,
};

// TODO: Implement simulate_transaction_with_config
pub async fn simulate_transaction_with_config(
    _tx: alloy_rpc_types::Transaction,
    _block_env: BlockEnv,
    _provider: std::sync::Arc<dyn alloy_provider::Provider>,
    _config: SimulationConfig,
) -> Result<SimulationOutput, SimulationError> {
    todo!("Implement simulate_transaction_with_config")
}

// Test module
#[cfg(test)]
mod tests;

// Run examples
#[cfg(test)]
mod run_examples;