use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectRawTransactionRequest {
    pub attempt_id: Option<String>,
    pub chain_id: u64,
    pub from: String,
    pub to: String,
    #[serde(default = "zero_string")]
    pub value: String,
    #[serde(default = "empty_data")]
    pub data: String,
    pub gas_limit: String,
    pub max_fee_per_gas: String,
    pub max_priority_fee_per_gas: String,
    pub nonce: Option<String>,
    pub bribe: Option<BribeRequest>,
    pub simulation: Option<SimulationReference>,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BribeRequest {
    /// Direct EOA bribe is the EIP-1559 priority fee paid to the block proposer.
    pub priority_fee_per_gas: String,
    /// Optional absolute max fee. When omitted, the executor preserves the request max fee unless
    /// it is below the requested priority fee.
    pub max_fee_per_gas: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationReference {
    pub block_number: u64,
    pub block_hash: Option<String>,
    pub state_root: Option<String>,
    pub expected_output_token: Option<String>,
    pub expected_output_amount: Option<String>,
    pub min_output_amount: Option<String>,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
}

fn zero_string() -> String {
    "0".to_string()
}

fn empty_data() -> String {
    "0x".to_string()
}

fn default_metadata() -> Value {
    json!({})
}
