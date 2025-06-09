use serde::{Serialize, Deserialize};
use revm_primitives::{B256, Address, U256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    pub hash: B256,
    pub block_number: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_used: u64,
    pub success: bool,
    pub internal_transfers: Vec<InternalTransfer>,
    pub state_changes: HashMap<Address, StateChange>,
    pub logs: Vec<Log>,
    pub metrics: ProcessingMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalTransfer {
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub call_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub balance_change: i128,
    pub nonce_change: i64,
    pub storage_changes: HashMap<B256, B256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetrics {
    pub fetch_time_ms: f64,
    pub simulation_time_ms: f64,
    pub total_time_ms: f64,
}