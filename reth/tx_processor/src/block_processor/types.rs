use crate::tx_processor::data_models::ProcessedTransaction;
use reth_chain_query::provider::{
    BlockHeader, TransactionData, TransactionReceipt, TransactionTrace,
};

/// Result of processing an entire block worth of transactions.
#[derive(Debug, Clone)]
pub struct ProcessedBlock {
    pub header: BlockHeader,
    pub transactions: Vec<ProcessedBlockTransactions>,
}

/// Per-transaction payload emitted by [`ProcessedBlock`].
#[derive(Debug, Clone)]
pub struct ProcessedBlockTransactions {
    pub metadata: TransactionData,
    pub receipt: TransactionReceipt,
    pub processed: ProcessedTransaction,
    pub trace: Option<TransactionTrace>,
}

/// Controls batch block processing behaviour.
#[derive(Debug, Clone, Copy)]
pub struct BlockBatchOptions {
    pub include_traces: bool,
    pub max_concurrency: usize,
}

impl Default for BlockBatchOptions {
    fn default() -> Self {
        Self {
            include_traces: true,
            max_concurrency: 10,
        }
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
}
