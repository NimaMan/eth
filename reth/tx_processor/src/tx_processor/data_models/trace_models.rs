use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalTransaction {
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub value: U256,
    pub gas: u64,
    pub gas_used: u64,
    pub trace_type: String,
    pub call_type: Option<String>,
    pub depth: u32,
    pub error: Option<String>,
}
