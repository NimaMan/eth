/// Type definitions for the transaction simulator
///
/// This module contains all the public types used throughout the tx_simulator library.
/// These types represent simulation results, internal transactions, and configuration options.
use crate::tx_fee_parameters::TxGasParameters;
use alloy_primitives::{Address, Bytes, Log as AlloyLog, U256};
pub use alloy_rpc_types_trace::geth::{CallFrame, StructLog};
use std::collections::HashMap;

/// Additional context to help explain reverts when raw error data is missing.
#[derive(Debug, Clone)]
pub struct RevertContext {
    pub target: Address,
    pub has_code: bool,
    pub calldata_len: usize,
}

/// Basic simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub revert_context: Option<RevertContext>,
}

/// Full simulation result with call trace (mirrors reth `/debug/trace_*` responses)
#[derive(Debug, Clone)]
pub struct FullSimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub revert_context: Option<RevertContext>,
    /// Geth-style call frame produced by the call tracer (mirrors `/debug/trace_*`).
    pub call_trace: CallFrame,
    /// Optional per-opcode logs from geth's default tracer (`structLogs` in RPC responses).
    /// Present when the simulation was executed with step recording enabled (full trace helpers).
    pub struct_logs: Option<Vec<StructLog>>,
    /// Raw EVM logs emitted during execution (matches transaction receipt logs).
    pub logs: Vec<AlloyLog>,
}

/// Result of a single transaction in a sequence
#[derive(Debug, Clone)]
pub struct SequentialTransactionResult {
    pub transaction_index: usize,
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub revert_context: Option<RevertContext>,
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
        crate::contract_simulation::decode_uint256_from_contract_output(&self.output)
    }

    /// Decode the output as a uint8 value
    pub fn decode_uint8(&self) -> u8 {
        crate::contract_simulation::decode_uint8_from_contract_output(&self.output)
    }

    /// Decode the output as a string
    pub fn decode_string(&self) -> String {
        crate::contract_simulation::decode_string_from_contract_output(&self.output)
    }
}

/// Parallel transaction simulation results
#[derive(Debug)]
pub struct ParallelTxSimulationResult {
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

/// Default fee handling when transactions omit explicit gas parameters
#[derive(Debug, Clone)]
pub struct FeeDefaults {
    pub chain_id: Option<u64>,
    pub max_fee_per_blob_gas: u128,
}

impl Default for FeeDefaults {
    fn default() -> Self {
        Self {
            chain_id: Some(1),
            max_fee_per_blob_gas: 0,
        }
    }
}

/// Default parameters for constructing view calls
#[derive(Debug, Clone)]
pub struct ViewCallDefaults {
    pub from: Address,
    pub gas_limit: u64,
}

impl Default for ViewCallDefaults {
    fn default() -> Self {
        Self {
            from: Address::ZERO,
            gas_limit: 3_000_000,
        }
    }
}

/// Optional per-call overrides when building view transactions
#[derive(Debug, Clone, Default)]
pub struct ViewCallOverrides {
    pub from: Option<Address>,
    pub gas_limit: Option<u64>,
}

impl ViewCallOverrides {
    pub fn resolve(&self, defaults: &ViewCallDefaults) -> ViewCallDefaults {
        ViewCallDefaults {
            from: self.from.unwrap_or(defaults.from),
            gas_limit: self.gas_limit.unwrap_or(defaults.gas_limit),
        }
    }
}

/// Aggregated simulator defaults
#[derive(Debug, Clone)]
pub struct SimulationDefaults {
    pub fee: FeeDefaults,
    pub view_call: ViewCallDefaults,
    pub tx_gas: TxGasParameters,
}

impl Default for SimulationDefaults {
    fn default() -> Self {
        Self {
            fee: FeeDefaults::default(),
            view_call: ViewCallDefaults::default(),
            tx_gas: TxGasParameters::default(),
        }
    }
}
