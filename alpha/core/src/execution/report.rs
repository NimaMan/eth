use crate::{
    amount::Amount,
    ids::{BlockNumber, OrderId, TxHash},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Submitted,
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub order_id: OrderId,
    pub status: ExecutionStatus,
    pub tx_hash: Option<TxHash>,
    pub block_number: Option<BlockNumber>,
    /// For buys: ETH spent. For sells: ETH received.
    pub filled_amount: Option<Amount>,
    /// For buys only: tokens received from the swap (from simulation or on-chain data).
    pub token_amount: Option<Amount>,
    pub gas_used: Option<u64>,
    pub error: Option<String>,
}
