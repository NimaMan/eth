/// Types for block-level simulation
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types_trace::geth::TraceResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Result of tracing an entire block - matches debug_traceBlockByNumber output
#[derive(Debug, Clone)]
pub struct BlockTraceResult {
    /// Block number that was traced
    pub block_number: u64,
    /// Block hash
    pub block_hash: B256,
    /// Traces for each transaction in the block (matches Reth's Vec<TraceResult>)
    pub traces: Vec<TraceResult>,
    /// Total gas used by all transactions
    pub total_gas_used: u64,
    /// Base fee for the block
    pub base_fee: Option<U256>,
    /// Timestamp of the block
    pub timestamp: u64,
}

/// Internal trace result for a single transaction
#[derive(Debug, Clone)]
pub struct TransactionTraceResult {
    /// Transaction hash
    pub tx_hash: B256,
    /// Transaction index in block
    pub tx_index: u64,
    /// The trace result (matches Reth's format)
    pub trace_result: TraceResult,
    /// Gas used by this transaction
    pub gas_used: u64,
}

/// Options for block simulation
#[derive(Debug, Clone, Default)]
pub struct BlockSimulationOptions {
    /// Whether to include full call traces (more expensive)
    pub include_traces: bool,
    /// Whether to include logs in traces
    pub include_logs: bool,
    /// Timeout for simulating each transaction (in milliseconds)
    pub timeout_per_tx: Option<u64>,
    /// Maximum number of transactions to process (for testing)
    pub max_transactions: Option<usize>,
    /// Tracer configuration (e.g., "callTracer" config)
    pub tracer_config: Option<TracerConfig>,
}

/// Tracer configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerConfig {
    /// Only trace top-level calls
    pub only_top_call: bool,
    /// Include storage changes
    pub with_storage: bool,
    /// Include memory snapshots
    pub with_memory: bool,
    /// Include stack snapshots
    pub with_stack: bool,
}

/// Traces plus timing and state-read measurements for one block replay.
#[derive(Debug, Clone)]
pub struct ProfiledBlockTrace {
    pub traces: Vec<TraceResult>,
    pub profile: BlockReplayProfile,
}

/// Controls optional replay profiling behavior.
#[derive(Debug, Clone, Default)]
pub struct ReplayProfileConfig {
    pub record_keys: bool,
    pub prewarm_keys: Option<StateAccessKeys>,
}

/// Cold replay timing breakdown for one block.
#[derive(Debug, Clone, Default)]
pub struct BlockReplayProfile {
    pub engine: &'static str,
    pub block_number: u64,
    pub block_hash: B256,
    pub total_ms: f64,
    pub block_hash_lookup_ms: f64,
    pub block_load_ms: f64,
    pub state_open_ms: f64,
    pub sender_recovery_ms: f64,
    pub evm_env_ms: f64,
    pub tx_env_ms: f64,
    pub inspector_build_ms: f64,
    pub evm_exec_ms: f64,
    pub trace_build_ms: f64,
    pub db_commit_ms: f64,
    pub tx_count: usize,
    pub gas_used: u64,
    pub trace_node_count: usize,
    pub errors: usize,
    pub state_reads: StateReadProfile,
    pub state_keys: StateAccessKeys,
    pub preload_ms: f64,
    pub exec_after_prewarm_ms: f64,
    pub tx_profiles: Vec<TransactionReplayProfile>,
}

/// Backing provider misses observed below `CacheDB`.
#[derive(Debug, Clone, Default)]
pub struct StateReadProfile {
    pub account_reads: u64,
    pub storage_reads: u64,
    pub code_reads: u64,
    pub block_hash_reads: u64,
    pub provider_read_ms: f64,
}

impl StateReadProfile {
    pub fn delta_since(&self, earlier: &Self) -> Self {
        Self {
            account_reads: self.account_reads.saturating_sub(earlier.account_reads),
            storage_reads: self.storage_reads.saturating_sub(earlier.storage_reads),
            code_reads: self.code_reads.saturating_sub(earlier.code_reads),
            block_hash_reads: self
                .block_hash_reads
                .saturating_sub(earlier.block_hash_reads),
            provider_read_ms: self.provider_read_ms - earlier.provider_read_ms,
        }
    }
}

/// Optional provider-miss keys observed below `CacheDB`.
#[derive(Debug, Clone, Default)]
pub struct StateAccessKeys {
    pub accounts: BTreeSet<Address>,
    pub storage: BTreeSet<StorageAccessKey>,
    pub code_hashes: BTreeSet<B256>,
    pub block_hashes: BTreeSet<u64>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct StorageAccessKey {
    pub address: Address,
    pub key: U256,
}

/// Optional per-transaction replay timing row.
#[derive(Debug, Clone, Default)]
pub struct TransactionReplayProfile {
    pub tx_index: usize,
    pub tx_hash: B256,
    pub gas_used: u64,
    pub trace_nodes: usize,
    pub sender_recovery_ms: f64,
    pub evm_env_ms: f64,
    pub tx_env_ms: f64,
    pub inspector_build_ms: f64,
    pub evm_exec_ms: f64,
    pub trace_build_ms: f64,
    pub db_commit_ms: f64,
    pub state_reads: StateReadProfile,
    pub errors: usize,
}
