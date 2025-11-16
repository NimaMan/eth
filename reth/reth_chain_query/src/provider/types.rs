/// Shared data types for the provider module
///
/// This module contains all the data structures used across different
/// provider operations including transactions, blocks, and traces.
use alloy_eips::{eip2930::AccessListItem, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

/// Transaction metadata (the transaction parameters, not execution results)
/// This is what was submitted to the network, not what happened when it executed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(default)]
    pub access_list: Vec<AccessListItem>,
    #[serde(default)]
    pub blob_versioned_hashes: Vec<B256>,
    pub max_fee_per_blob_gas: Option<U256>,
    #[serde(default)]
    pub signed_authorizations: Vec<SignedAuthorization>,
}

// Alias for backwards compatibility during migration
pub type TransactionData = TransactionMetadata;

/// Transaction receipt with logs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionReceipt {
    pub tx_hash: B256,
    pub status: bool,
    pub gas_used: u64,
    pub logs: Vec<Log>,
    pub cumulative_gas_used: u64,
    pub effective_gas_price: U256,
    pub contract_address: Option<Address>,
}

/// Event log from transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionTrace {
    pub call_frame: CallFrame,
    pub gas_used: u64,
    pub output: Bytes,
    pub error: Option<String>,
}

/// Call frame representing execution trace
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub balance_changes: Vec<StateBalanceChange>,
    pub storage_changes: Vec<StorageChange>,
    pub nonce_changes: Vec<NonceChange>,
}

/// Balance change for an address
#[derive(Debug, Clone)]
pub struct StateBalanceChange {
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
