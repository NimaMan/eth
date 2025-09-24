/// Shared data types for the provider module
///
/// This module contains all the data structures used across different
/// provider operations including transactions, blocks, and traces.
use alloy_primitives::{Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

/// Transaction metadata (the transaction parameters, not execution results)
/// This is what was submitted to the network, not what happened when it executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    pub hash: B256,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub tx_number: u64, // Sequential transaction ID in Reth
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub input: Bytes, // The calldata sent with the transaction
    pub gas_price: U256,
    pub gas_limit: u64,
    pub nonce: u64,
    pub transaction_type: u8,
}

// Alias for backwards compatibility during migration
pub type TransactionData = TransactionMetadata;

/// Transaction receipt with logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub tx_hash: B256,
    pub status: bool,
    pub gas_used: u64,
    pub logs: Vec<Log>,
    pub cumulative_gas_used: u64,
    pub effective_gas_price: U256,
}

/// Event log from transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Bytes,
    pub log_index: u64,
    pub transaction_index: u64,
    pub block_number: u64,
}

/// Transaction with optional trace data
#[derive(Debug, Clone)]
pub struct TransactionWithTrace {
    pub transaction: TransactionData,
    pub receipt: TransactionReceipt,
    pub trace: Option<TransactionTrace>,
}

/// Transaction trace from simulation or RPC
#[derive(Debug, Clone)]
pub struct TransactionTrace {
    pub call_frame: CallFrame,
    pub gas_used: u64,
    pub output: Bytes,
    pub error: Option<String>,
}

/// Call frame representing execution trace
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub input: Bytes,
    pub output: Bytes,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub depth: u32,
    pub call_type: CallType,
    pub subcalls: Vec<CallFrame>,
}

/// Block with transactions (for get_block_with_txs compatibility)
#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<reth_primitives::TransactionSignedEcRecovered>,
}

/// Type of call in trace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallType {
    Call,
    DelegateCall,
    StaticCall,
    Create,
    Create2,
}

impl std::fmt::Display for CallType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallType::Call => write!(f, "CALL"),
            CallType::DelegateCall => write!(f, "DELEGATECALL"),
            CallType::StaticCall => write!(f, "STATICCALL"),
            CallType::Create => write!(f, "CREATE"),
            CallType::Create2 => write!(f, "CREATE2"),
        }
    }
}

/// State changes from transaction execution
#[derive(Debug, Clone)]
pub struct StateChanges {
    pub balance_changes: Vec<BalanceChange>,
    pub storage_changes: Vec<StorageChange>,
    pub nonce_changes: Vec<NonceChange>,
}

/// Balance change for an address
#[derive(Debug, Clone)]
pub struct BalanceChange {
    pub address: Address,
    pub before: U256,
    pub after: U256,
}

/// Storage slot change
#[derive(Debug, Clone)]
pub struct StorageChange {
    pub contract: Address,
    pub slot: B256,
    pub before: U256,
    pub after: U256,
}

/// Nonce change for an address
#[derive(Debug, Clone)]
pub struct NonceChange {
    pub address: Address,
    pub before: u64,
    pub after: u64,
}

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

/// Raw block data fetched from the database (optionally including traces)
#[derive(Debug, Clone)]
pub struct RawBlockData {
    pub header: BlockHeader,
    pub transactions: Vec<TransactionMetadata>,
    pub receipts: Vec<TransactionReceipt>,
    pub traces: Option<Vec<TransactionTrace>>,
}

/// Full transaction data including metadata, receipt, and optional trace
#[derive(Debug, Clone)]
pub struct FullTransactionData {
    /// Transaction metadata (from database)
    pub tx_metadata: TransactionMetadata,
    /// Transaction receipt with logs (from database)
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
#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub number: u64,
    pub hash: B256,
    pub parent_hash: B256,
    pub timestamp: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub base_fee_per_gas: Option<u64>,
}
