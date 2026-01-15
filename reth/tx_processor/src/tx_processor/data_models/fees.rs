use alloy_primitives::U256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransactionFees {
    pub gas_price: U256, // Effective gas price paid
    pub gas_used: u64,
    #[serde(default)]
    pub gas_limit: u64,
    pub tx_fee: U256, // Total fee in wei

    // EIP-1559 fields
    pub protocol_type: String,          // "legacy", "eip1559", "eip2930"
    pub max_fee_per_gas: Option<U256>,  // User's max willingness
    pub max_priority_fee: Option<U256>, // User's max tip
    pub max_fee_per_blob_gas: Option<U256>,
    pub blob_gas_used: Option<u64>,
}

impl TransactionFees {
    pub fn new(gas_price: U256, gas_used: u64, gas_limit: u64) -> Self {
        let tx_fee = gas_price * U256::from(gas_used);
        Self {
            gas_price,
            gas_used,
            gas_limit,
            tx_fee,
            protocol_type: "unknown".to_string(),
            max_fee_per_gas: None,
            max_priority_fee: None,
            max_fee_per_blob_gas: None,
            blob_gas_used: None,
        }
    }

    pub fn new_eip1559(
        gas_price: U256,
        gas_used: u64,
        gas_limit: u64,
        max_fee_per_gas: U256,
        max_priority_fee: U256,
    ) -> Self {
        let tx_fee = gas_price * U256::from(gas_used);
        Self {
            gas_price,
            gas_used,
            gas_limit,
            tx_fee,
            protocol_type: "eip1559".to_string(),
            max_fee_per_gas: Some(max_fee_per_gas),
            max_priority_fee: Some(max_priority_fee),
            max_fee_per_blob_gas: None,
            blob_gas_used: None,
        }
    }
}
