use alloy_primitives::U256;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransactionFees {
    pub gas_price: U256,
    pub gas_used: u64,
    pub txn_fee: U256,
}

impl TransactionFees {
    pub fn new(gas_price: U256, gas_used: u64) -> Self {
        let txn_fee = gas_price * U256::from(gas_used);
        Self {
            gas_price,
            gas_used,
            txn_fee,
        }
    }
}