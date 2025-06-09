use serde::{Serialize, Deserialize};
use revm_primitives::{B256, Address, U256};
use std::collections::HashMap;

// Import Reth types for raw transaction storage
// use reth_ethereum::{TransactionSigned, Receipt}; // Disabled - causes REVM version conflicts

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    // Basic transaction info
    pub hash: String,
    pub block_number: u64,
    pub transaction_index: u32,
    pub from_address: String,
    pub to_address: Option<String>,
    pub value: String,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub gas_price: Option<String>,
    pub status: bool,
    pub nonce: u64,
    pub input_data: String,
    
    // Timing metrics
    pub fetch_time_ms: f64,
    pub simulation_time_ms: f64,
    pub total_time_ms: f64,
    
    // Analysis results
    pub transaction_type: String,
    pub is_contract_call: bool,
    pub is_contract_creation: bool,
    pub has_value_transfer: bool,
    pub logs_count: usize,
    
    // Optional fields for advanced processing (disabled due to reth_ethereum dependency conflicts)
    // #[serde(skip)]
    // pub raw_signed_transaction: Option<TransactionSigned>,
    // #[serde(skip)]
    // pub raw_receipt: Option<Receipt>,
    
    // Optional processed data (can be filled by REVM simulation)
    pub internal_transfers: Vec<InternalTransfer>,
    pub state_changes: HashMap<String, StateChange>,
    pub processed_logs: Vec<ProcessedLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalTransfer {
    pub from: String,
    pub to: String,
    pub value: String,
    pub call_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub balance_change: String,
    pub nonce_change: i64,
    pub storage_changes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

impl Default for ProcessedTransaction {
    fn default() -> Self {
        Self {
            hash: String::new(),
            block_number: 0,
            transaction_index: 0,
            from_address: String::new(),
            to_address: None,
            value: "0".to_string(),
            gas_used: 0,
            gas_limit: 0,
            gas_price: None,
            status: false,
            nonce: 0,
            input_data: String::new(),
            fetch_time_ms: 0.0,
            simulation_time_ms: 0.0,
            total_time_ms: 0.0,
            transaction_type: String::new(),
            is_contract_call: false,
            is_contract_creation: false,
            has_value_transfer: false,
            logs_count: 0,
            // raw_signed_transaction: None,
            // raw_receipt: None,
            internal_transfers: Vec::new(),
            state_changes: HashMap::new(),
            processed_logs: Vec::new(),
        }
    }
}