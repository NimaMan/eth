use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthTxPolicyDecisionList {
    #[serde(default)]
    pub attempt_id: Option<String>,
    #[serde(default)]
    pub decisions: Vec<EthTxPolicyDecision>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthTxPolicyDecision {
    pub id: i64,
    pub attempt_id: String,
    pub created_at: String,
    pub route: String,
    pub decision: String,
    pub policy_version: String,
    pub reasons: Value,
    pub request: Value,
    pub normalized: Value,
    pub metadata: Value,
    pub strategy_name: Option<String>,
    pub strategy_run_id: Option<String>,
    pub trade_id: Option<String>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub selector: Option<String>,
    pub value_wei: Option<String>,
    pub gas_limit: Option<String>,
    pub max_fee_per_gas_wei: Option<String>,
    pub max_priority_fee_per_gas_wei: Option<String>,
    pub estimated_worst_case_cost_wei: Option<String>,
    pub simulation_block_number: Option<i64>,
    pub latest_block_number: Option<i64>,
}

impl EthTxPolicyDecision {
    pub fn is_accepted(&self) -> bool {
        self.decision.eq_ignore_ascii_case("accepted")
    }

    pub fn is_rejected_or_disabled(&self) -> bool {
        self.decision.eq_ignore_ascii_case("rejected")
            || self.decision.eq_ignore_ascii_case("disabled")
    }
}
