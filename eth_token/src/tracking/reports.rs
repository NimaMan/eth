use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenStateUpdateReport {
    pub token_address: String,
    pub token_state_updated: bool,
    #[serde(default)]
    pub discovered_uniswap_v2_pools: Vec<String>,
    #[serde(default)]
    pub updated_uniswap_v2_pools: Vec<String>,
    #[serde(default)]
    pub simulated_uniswap_v2_pools: Vec<String>,
    #[serde(default)]
    pub discovered_uniswap_v3_pools: Vec<String>,
    #[serde(default)]
    pub updated_uniswap_v3_pools: Vec<String>,
    #[serde(default)]
    pub simulated_uniswap_v3_pools: Vec<String>,
    #[serde(default)]
    pub discovered_uniswap_v4_pools: Vec<String>,
    #[serde(default)]
    pub updated_uniswap_v4_pools: Vec<String>,
    #[serde(default)]
    pub simulated_uniswap_v4_pools: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenTransactionUpdateError {
    pub tx_hash: String,
    pub tx_index: u64,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenBlockUpdateReport {
    pub block_number: u64,
    pub block_hash: String,
    pub block_timestamp: u64,
    pub transaction_count: usize,
    pub processed_transaction_count: usize,
    pub failed_transaction_count: usize,
    pub already_processed: bool,
    pub created_token_addresses: Vec<String>,
    pub updated_token_addresses: Vec<String>,
    pub token_updates: Vec<TokenStateUpdateReport>,
    pub transaction_errors: Vec<TokenTransactionUpdateError>,
}
