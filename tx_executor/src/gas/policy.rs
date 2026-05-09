use ethers_core::types::U256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TxGasProfile {
    pub gas_limit: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

pub fn effective_priority_fee(
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
    base_fee_per_gas: U256,
) -> U256 {
    if max_fee_per_gas <= base_fee_per_gas {
        U256::zero()
    } else {
        (max_fee_per_gas - base_fee_per_gas).min(max_priority_fee_per_gas)
    }
}
