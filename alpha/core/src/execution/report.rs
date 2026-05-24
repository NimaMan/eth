use crate::{
    amount::Amount,
    ids::{BlockHash, BlockNumber, OrderId, TxHash},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Submitted,
    Pending,
    Confirmed,
    Deferred,
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
    #[serde(default)]
    pub gas_cost: Option<Amount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mined_evidence: Option<MinedExecutionEvidence>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MinedExecutionEvidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_block_number: Option<BlockNumber>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cumulative_gas_used: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_block_number: Option<BlockNumber>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_confirmation_block: Option<BlockNumber>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmation_lag_blocks: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_gas_price_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_gas_price_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_gas_cost_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_gas_limit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_max_fee_per_gas_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_max_priority_fee_per_gas_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_bribe_priority_fee_per_gas_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_bribe_max_fee_per_gas_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_policy_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_policy_signal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_policy_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_policy_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_policy_profiles: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_rank_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_estimated_max_cost_eth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_estimated_priority_spend_eth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_policy_guard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_confirmation_depth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recheck_confirmation_depth: Option<u64>,
}
