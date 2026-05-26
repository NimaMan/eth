use serde::{Deserialize, Serialize};

pub use crate::{
    KartalBribeRequest, KartalSimulationReference, KartalSubmitDirectRawResult,
    KartalSubmitTransactionRequest, KartalSubmitTransactionResult, LiveDirectRawTransactionRequest,
    LiveTraderTxSignal, LiveTxExecution, TxOrderingPolicy, TxSubmissionPolicy,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KartalStatusBroadcastMode {
    DryRun,
    Broadcast,
    #[serde(other)]
    Unknown,
}

impl KartalStatusBroadcastMode {
    pub fn is_dry_run(&self) -> bool {
        matches!(self, Self::DryRun)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KartalEthTxExecutorStatus {
    pub service: String,
    pub enabled: bool,
    pub api_token_configured: bool,
    pub signer_available: bool,
    pub execution_disabled: bool,
    pub broadcast_mode: KartalStatusBroadcastMode,
    #[serde(default)]
    pub submission_policy_kinds: Vec<String>,
    pub chain_id: u64,
    pub rpc_url: String,
    pub journal_path: Option<String>,
    pub direct_raw_endpoint: String,
    #[serde(default)]
    pub submit_endpoint: Option<String>,
    #[serde(default)]
    pub flashbots_tail_bundle_endpoint: Option<String>,
    #[serde(default)]
    pub flashbots_relay_url: Option<String>,
    #[serde(default)]
    pub flashbots_auth_configured: Option<bool>,
    pub policy: KartalEthTxPolicyStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KartalEthTxPolicyStatus {
    pub version: String,
    pub allowed_from_count: usize,
    pub allowed_target_count: usize,
    pub allowed_selector_count: usize,
    pub max_value_wei: String,
    pub max_gas_limit: u64,
    pub max_fee_per_gas_wei: String,
    pub max_priority_fee_per_gas_wei: String,
    pub max_transaction_cost_wei: String,
    pub max_daily_cost_wei: String,
    #[serde(default)]
    pub daily_spend_cap_enabled: Option<bool>,
    pub daily_spend: KartalDailySpendStatus,
    pub require_simulation: bool,
    pub max_simulation_age_blocks: u64,
    pub required_metadata_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KartalDailySpendStatus {
    pub spend_day: String,
    pub spent_wei: String,
    pub remaining_daily_cost_wei: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn status_decodes_dry_run_policy_summary() {
        let status: KartalEthTxExecutorStatus = serde_json::from_value(json!({
            "service": "eth_tx_executor",
            "enabled": false,
            "api_token_configured": true,
            "signer_available": false,
            "execution_disabled": false,
            "broadcast_mode": "dry_run",
            "chain_id": 1,
            "rpc_url": "http://127.0.0.1:8545",
            "journal_path": null,
            "direct_raw_endpoint": "/eth/tx/direct-raw",
            "submit_endpoint": "/eth/tx/submit",
            "flashbots_tail_bundle_endpoint": "/eth/tx/flashbots/mev-share-tail",
            "flashbots_relay_url": "https://relay.flashbots.net",
            "flashbots_auth_configured": true,
            "policy": {
                "version": "eth_tx_policy_v1",
                "allowed_from_count": 0,
                "allowed_target_count": 0,
                "allowed_selector_count": 0,
                "max_value_wei": "0",
                "max_gas_limit": 500000,
                "max_fee_per_gas_wei": "1000000000000",
                "max_priority_fee_per_gas_wei": "500000000000",
                "max_transaction_cost_wei": "0",
                "max_daily_cost_wei": "0",
                "daily_spend_cap_enabled": false,
                "daily_spend": {
                    "spend_day": "2026-05-19",
                    "spent_wei": "0",
                    "remaining_daily_cost_wei": null
                },
                "require_simulation": true,
                "max_simulation_age_blocks": 2,
                "required_metadata_fields": ["wire_protocol"]
            }
        }))
        .unwrap();

        assert!(status.broadcast_mode.is_dry_run());
        assert_eq!(status.policy.allowed_target_count, 0);
        assert_eq!(
            status.flashbots_tail_bundle_endpoint.as_deref(),
            Some("/eth/tx/flashbots/mev-share-tail")
        );
        assert_eq!(status.flashbots_auth_configured, Some(true));
    }
}
