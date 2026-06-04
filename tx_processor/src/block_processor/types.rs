use crate::tx_processor::data_models::ProcessedTransaction;
use reth_chain_query::provider::{
    BlockHeader, TransactionData, TransactionReceipt, TransactionTrace,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tx_simulator::block_simulation::BlockTraceEngine;

/// Result of processing an entire block worth of transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedBlock {
    pub header: BlockHeader,
    pub transactions: Vec<ProcessedBlockTransactions>,
}

/// Per-transaction payload emitted by [`ProcessedBlock`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedBlockTransactions {
    pub metadata: TransactionData,
    pub receipt: TransactionReceipt,
    pub processed: ProcessedTransaction,
    pub trace: Option<TransactionTrace>,
    pub processing_error: Option<String>,
}

/// Controls batch block processing behaviour.
#[derive(Debug, Clone, Copy)]
pub struct BlockBatchOptions {
    /// Capture call traces for every block in the batch. MUST stay `true` on any path
    /// whose `ProcessedTransaction`s feed net-flow accounting (PnL / token-state /
    /// risk-atlas): event-less internal transfers (e.g. a custody `transferFrom` that
    /// emits no `Transfer` log) are captured ONLY from traces, so `false` silently drops
    /// them and breaks the transfer-capture invariant (see `data_models/README.md`). The
    /// default is `true` and is regression-pinned by `accounting_default_traces_every_block`.
    pub include_traces: bool,
    pub max_concurrency: usize,
    pub trace_engine: BlockTraceEngine,
}

/// Timing breakdown for `BlockProcessor::process_raw_block_profiled`.
#[derive(Debug, Clone, Default)]
pub struct ProcessRawBlockProfile {
    pub total: Duration,
    pub tx_processing: Duration,
    pub trace_conversion: Duration,
    pub internal_extraction: Duration,
    pub balance_calculation: Duration,
    pub contract_creation: Duration,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PersistentProcessedBlockCacheMode {
    ReadWrite,
    ReadOnly,
    Refresh,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProcessedBlockSource {
    LiveDirect,
    Cache,
    Processed,
}

impl ProcessedBlockSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LiveDirect => "live_block_update",
            Self::Cache => "processed_block_disk_cache",
            Self::Processed => "processed_block",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachedProcessedBlock {
    pub block: ProcessedBlock,
    pub cache_hit: bool,
    pub cache_read: Duration,
    pub cache_write: Duration,
    pub source: ProcessedBlockSource,
}

impl Default for BlockBatchOptions {
    fn default() -> Self {
        Self {
            include_traces: true,
            max_concurrency: 10,
            trace_engine: BlockTraceEngine::default(),
        }
    }
}

#[cfg(test)]
mod block_batch_options_tests {
    use super::BlockBatchOptions;

    /// Transfer-capture invariant guard. The chain-server range cache-fill
    /// (`load_processed_block_range_with_options`) and the live block processor both build
    /// on `BlockBatchOptions::default()` to trace every accounted block. If this default
    /// ever flips to `false`, event-less internal transfers (custody/holder-balance
    /// drains) silently vanish from net-flow accounting — see the transfer-capture
    /// invariant in `data_models/README.md`. Keep it `true`.
    #[test]
    fn accounting_default_traces_every_block() {
        assert!(
            BlockBatchOptions::default().include_traces,
            "BlockBatchOptions::default().include_traces must stay true: net-flow accounting \
             capture of event-less transfers depends on traces"
        );
    }
}

impl BlockBatchOptions {
    pub fn with_max_concurrency(mut self, value: usize) -> Self {
        if value == 0 {
            return self;
        }
        self.max_concurrency = value;
        self
    }

    pub fn with_traces(mut self, include: bool) -> Self {
        self.include_traces = include;
        self
    }

    pub fn with_trace_engine(mut self, trace_engine: BlockTraceEngine) -> Self {
        self.trace_engine = trace_engine;
        self
    }
}
