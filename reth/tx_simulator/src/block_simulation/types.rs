/// Types for block-level simulation
use alloy_primitives::{B256, U256};
use alloy_rpc_types_trace::geth::TraceResult;
use serde::{Deserialize, Serialize};

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