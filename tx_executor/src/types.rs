use ethers_core::types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Received,
    Rejected,
    Signed,
    DryRun,
    Broadcast,
    BroadcastError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedDirectRawTransaction {
    pub attempt_id: String,
    pub chain_id: u64,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub data: Vec<u8>,
    pub gas_limit: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub nonce: Option<U256>,
    pub simulation: Option<crate::request::SimulationReference>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub tx_hash: H256,
    pub raw_tx_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedFlashbotsAuth {
    pub body_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitDirectRawResult {
    pub attempt_id: String,
    pub status: ExecutionStatus,
    pub tx_hash: Option<H256>,
    pub from: Address,
    pub to: Address,
    pub nonce: U256,
    pub gas_limit: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub error: Option<String>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignDirectRawResult {
    pub attempt_id: String,
    pub status: ExecutionStatus,
    pub tx_hash: H256,
    pub raw_tx_hex: String,
    pub from: Address,
    pub to: Address,
    pub nonce: U256,
    pub gas_limit: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub elapsed_ms: u128,
}
