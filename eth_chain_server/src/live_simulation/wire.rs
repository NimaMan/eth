use alloy_primitives::{Log as AlloyLog, B256};
use serde::{Deserialize, Serialize};
use tx_simulator::UnsignedTransaction;

#[derive(Debug, Clone, Deserialize)]
pub struct LiveUnsignedTxSimulationRequest {
    pub block: u64,
    pub transaction: UnsignedTransaction,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveUnsignedTxSimulationResponse {
    pub schema: &'static str,
    pub block: u64,
    pub block_hash: Option<B256>,
    pub state_source: &'static str,
    pub success: bool,
    pub gas_used: u64,
    pub effective_gas_price_wei: Option<String>,
    pub tx_type: Option<u8>,
    pub revert_reason: Option<String>,
    pub log_count: usize,
    pub logs: Vec<AlloyLog>,
    pub base_fee_per_gas_wei: Option<String>,
}
