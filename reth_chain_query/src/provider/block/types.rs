use alloy_primitives::B256;

use crate::provider::{StateChanges, TransactionMetadata, TransactionReceipt, TransactionTrace};
use reth_ethereum_primitives::TransactionSigned;
use reth_primitives_traits::Recovered;

/// Complete block with all transactions
#[derive(Debug, Clone)]
pub struct BlockTransactions {
    pub block_number: u64,
    pub block_hash: B256,
    pub timestamp: u64,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee_per_gas: Option<u64>,
    pub transactions: Vec<FullTransactionData>,
}

/// Raw block data fetched from storage or RPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawBlockData {
    pub header: BlockHeader,
    pub transactions: Vec<TransactionMetadata>,
    pub receipts: Vec<TransactionReceipt>,
    pub traces: Option<Vec<TransactionTrace>>,
}

/// Full transaction data including metadata, receipt, and optional trace
#[derive(Debug, Clone)]
pub struct FullTransactionData {
    /// Transaction metadata (from database or RPC)
    pub tx_metadata: TransactionMetadata,
    /// Transaction receipt with logs
    pub tx_receipt: TransactionReceipt,

    /// Transaction trace from RPC or simulation (optional)
    pub tx_trace: Option<TransactionTrace>,

    /// State changes computed from trace
    pub state_changes: Option<StateChanges>,
}

/// Options for fetching block transactions
#[derive(Debug, Clone, Default)]
pub struct BlockTransactionOptions {
    /// Include trace data (from RPC or simulation)
    pub include_traces: bool,
    /// Include state changes
    pub include_state_changes: bool,
}

/// Block header information from Headers table
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
    pub number: u64,
    pub hash: B256,
    pub parent_hash: B256,
    pub timestamp: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub base_fee_per_gas: Option<u64>,
    pub withdrawals_root: Option<B256>,
    pub blob_gas_used: Option<u64>,
    pub excess_blob_gas: Option<u64>,
    pub parent_beacon_block_root: Option<B256>,
    pub requests_hash: Option<B256>,
    pub block_access_list_hash: Option<B256>,
    pub slot_number: Option<u64>,
}

/// Block with transactions (for get_block_with_txs compatibility)
#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Recovered<TransactionSigned>>,
}
