/// Type definitions for the transaction simulator
/// 
/// This module contains all the public types used throughout the tx_simulator library.
/// These types represent simulation results, internal transactions, and configuration options.

use alloy_primitives::{Address, U256, Bytes};
pub use alloy_rpc_types_trace::geth::CallFrame;
use std::collections::HashMap;

/// Basic simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
}

/// Extended simulation result
#[derive(Debug, Clone)]
pub struct DetailedSimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
}


/// Full simulation result with call trace
#[derive(Debug, Clone)]
pub struct FullSimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub call_trace: CallFrame,
}

/// Result of a single transaction in a sequence
#[derive(Debug, Clone)]
pub struct SequentialTransactionResult {
    pub transaction_index: usize,
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    /// Cumulative gas used up to this point in the sequence
    pub cumulative_gas_used: u64,
    /// Nonces after this transaction (for tracking state)
    pub updated_nonces: HashMap<Address, u64>,
}

/// Complete result of simulating a sequence of transactions
#[derive(Debug, Clone)]
pub struct SequentialSimulationResult {
    pub total_transactions: usize,
    pub successful_transactions: usize,
    pub failed_transactions: usize,
    pub total_gas_used: u64,
    pub results: Vec<SequentialTransactionResult>,
    /// Whether the entire sequence was successful (all transactions succeeded)
    pub sequence_success: bool,
}

/// Options for sequential simulation
#[derive(Debug, Clone)]
pub struct SequentialSimulationOptions {
    /// Block number to simulate at (None = latest)
    pub at_block: Option<u64>,
    /// Whether to stop simulation on first failure (default: true)
    pub stop_on_failure: bool,
    /// Whether to auto-increment nonces for repeated senders (default: true)
    pub auto_increment_nonces: bool,
    /// Gas limit per transaction (None = use transaction's gas limit)
    pub gas_limit_per_tx: Option<u64>,
}

impl Default for SequentialSimulationOptions {
    fn default() -> Self {
        Self {
            at_block: None,
            stop_on_failure: true,
            auto_increment_nonces: true,
            gas_limit_per_tx: None,
        }
    }
}

/// View function result
#[derive(Debug, Clone)]
pub struct ViewFunctionResult {
    pub success: bool,
    pub output: Bytes,
    pub gas_used: u64,
}

impl ViewFunctionResult {
    /// Decode the output as a U256 value
    pub fn decode_uint256(&self) -> U256 {
        crate::view_function_simulator::decode_uint256_result(&self.output)
    }
    
    /// Decode the output as a uint8 value
    pub fn decode_uint8(&self) -> u8 {
        crate::view_function_simulator::decode_uint8_result(&self.output)
    }
    
    /// Decode the output as a string
    pub fn decode_string(&self) -> String {
        crate::view_function_simulator::decode_string_result(&self.output)
    }
}

/// Batch simulation results
#[derive(Debug)]
pub struct BatchSimulationResult {
    /// Total number of transactions in the batch
    pub total: usize,
    /// Number of successfully simulated transactions
    pub successful: usize,
    /// Number of failed simulations
    pub failed: usize,
    /// Number of timed out simulations
    pub timed_out: usize,
    /// Individual results for each transaction (tx_hash, result)
    pub results: Vec<(String, eyre::Result<SimulationResult>)>,
    /// Total duration of batch processing
    pub duration: std::time::Duration,
    /// Average time per transaction
    pub avg_time_per_tx: std::time::Duration,
}

