use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_executor::DirectRawTransactionRequest;

use super::config::EthTxPolicyConfig;

pub const ETH_UNSIGNED_TX_WIRE_PROTOCOL: &str = "eth_unsigned_tx";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EthTxPolicyOutcome {
    Accepted,
    Rejected,
    Disabled,
}

impl EthTxPolicyOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthTxPolicyEvaluation {
    pub attempt_id: String,
    pub outcome: EthTxPolicyOutcome,
    pub policy_version: String,
    pub reasons: Vec<String>,
    pub normalized: EthTxPolicyNormalized,
}

impl EthTxPolicyEvaluation {
    pub fn is_accepted(&self) -> bool {
        self.outcome == EthTxPolicyOutcome::Accepted
    }

    pub fn rejected_from(
        attempt_id: impl Into<String>,
        policy_version: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            attempt_id: attempt_id.into(),
            outcome: EthTxPolicyOutcome::Rejected,
            policy_version: policy_version.into(),
            reasons: vec![reason.into()],
            normalized: EthTxPolicyNormalized::default(),
        }
    }

    pub fn with_rejection(mut self, reason: impl Into<String>) -> Self {
        self.outcome = EthTxPolicyOutcome::Rejected;
        self.reasons.push(reason.into());
        self
    }

    pub fn disabled_from(
        attempt_id: impl Into<String>,
        policy_version: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            attempt_id: attempt_id.into(),
            outcome: EthTxPolicyOutcome::Disabled,
            policy_version: policy_version.into(),
            reasons: vec![reason.into()],
            normalized: EthTxPolicyNormalized::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EthTxPolicyNormalized {
    pub from: Option<String>,
    pub to: Option<String>,
    pub selector: Option<String>,
    pub value_wei: Option<String>,
    pub gas_limit: Option<String>,
    pub max_fee_per_gas_wei: Option<String>,
    pub max_priority_fee_per_gas_wei: Option<String>,
    pub estimated_worst_case_cost_wei: Option<String>,
    pub simulation_block_number: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub strategy_name: Option<String>,
    pub strategy_run_id: Option<String>,
    pub trade_id: Option<String>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
}

impl EthTxPolicyNormalized {
    pub fn to_json(&self) -> Value {
        json!(self)
    }
}

pub fn evaluate_eth_tx_policy(
    policy: &EthTxPolicyConfig,
    request: &DirectRawTransactionRequest,
    latest_block_number: Option<u64>,
) -> EthTxPolicyEvaluation {
    let attempt_id = request
        .attempt_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let mut reasons = Vec::new();
    let mut normalized = EthTxPolicyNormalized {
        latest_block_number,
        ..EthTxPolicyNormalized::default()
    };

    let from = normalize_address_like(&request.from);
    normalized.from = from.clone();
    if let Some(from) = from.as_deref() {
        if policy.allowed_from_addresses.is_empty() {
            reasons.push("from address allowlist is empty".to_string());
        } else if !contains_normalized_address(&policy.allowed_from_addresses, from) {
            reasons.push(format!("from address {from} is not allowed"));
        }
    } else {
        reasons.push("from address is invalid".to_string());
    }

    let to = normalize_address_like(&request.to);
    normalized.to = to.clone();
    if let Some(to) = to.as_deref() {
        if policy.allowed_targets.is_empty() {
            reasons.push("target allowlist is empty".to_string());
        } else if !contains_normalized_address(&policy.allowed_targets, to) {
            reasons.push(format!("target {to} is not allowed"));
        }
    } else {
        reasons.push("target address is invalid".to_string());
    }

    let selector = calldata_selector(&request.data);
    normalized.selector = selector.clone();
    if let Some(selector) = selector.as_deref() {
        if policy.allowed_selectors.is_empty() {
            reasons.push("calldata selector allowlist is empty".to_string());
        } else if !contains_normalized_selector(&policy.allowed_selectors, selector) {
            reasons.push(format!("calldata selector {selector} is not allowed"));
        }
    } else {
        reasons.push("calldata selector is missing or invalid".to_string());
    }

    let value = parse_quantity_u128(&request.value);
    match &value {
        Ok(value) => {
            normalized.value_wei = Some(value.to_string());
            if *value > policy.max_value_wei {
                reasons.push(format!(
                    "value {} exceeds policy cap {}",
                    value, policy.max_value_wei
                ));
            }
        }
        Err(reason) => reasons.push(format!("value is invalid: {reason}")),
    }

    let gas_limit = parse_quantity_u128(&request.gas_limit);
    match &gas_limit {
        Ok(gas_limit) => {
            normalized.gas_limit = Some(gas_limit.to_string());
            if *gas_limit > u128::from(policy.max_gas_limit) {
                reasons.push(format!(
                    "gas_limit {} exceeds policy cap {}",
                    gas_limit, policy.max_gas_limit
                ));
            }
        }
        Err(reason) => reasons.push(format!("gas_limit is invalid: {reason}")),
    }

    let max_fee = parse_quantity_u128(&request.max_fee_per_gas);
    match &max_fee {
        Ok(max_fee) => {
            normalized.max_fee_per_gas_wei = Some(max_fee.to_string());
            if *max_fee > policy.max_fee_per_gas_wei {
                reasons.push(format!(
                    "max_fee_per_gas {} exceeds policy cap {}",
                    max_fee, policy.max_fee_per_gas_wei
                ));
            }
        }
        Err(reason) => reasons.push(format!("max_fee_per_gas is invalid: {reason}")),
    }

    let max_priority_fee = parse_quantity_u128(&request.max_priority_fee_per_gas);
    match &max_priority_fee {
        Ok(max_priority_fee) => {
            normalized.max_priority_fee_per_gas_wei = Some(max_priority_fee.to_string());
            if *max_priority_fee > policy.max_priority_fee_per_gas_wei {
                reasons.push(format!(
                    "max_priority_fee_per_gas {} exceeds policy cap {}",
                    max_priority_fee, policy.max_priority_fee_per_gas_wei
                ));
            }
        }
        Err(reason) => reasons.push(format!("max_priority_fee_per_gas is invalid: {reason}")),
    }

    if let (Ok(value), Ok(gas_limit), Ok(max_fee)) = (value, gas_limit, max_fee) {
        match gas_limit
            .checked_mul(max_fee)
            .and_then(|gas_cost| gas_cost.checked_add(value))
        {
            Some(cost) => {
                normalized.estimated_worst_case_cost_wei = Some(cost.to_string());
                if policy.max_transaction_cost_wei == 0 {
                    reasons.push(
                        "max transaction cost policy cap is zero; live tx signing is not configured"
                            .to_string(),
                    );
                } else if cost > policy.max_transaction_cost_wei {
                    reasons.push(format!(
                        "estimated worst-case cost {} exceeds policy cap {}",
                        cost, policy.max_transaction_cost_wei
                    ));
                }
            }
            None => reasons.push("estimated worst-case cost overflows u128".to_string()),
        }
    }

    evaluate_simulation(
        policy,
        request,
        latest_block_number,
        &mut normalized,
        &mut reasons,
    );
    evaluate_metadata(policy, &request.metadata, &mut normalized, &mut reasons);

    EthTxPolicyEvaluation {
        attempt_id,
        outcome: if reasons.is_empty() {
            EthTxPolicyOutcome::Accepted
        } else {
            EthTxPolicyOutcome::Rejected
        },
        policy_version: policy.version.clone(),
        reasons,
        normalized,
    }
}

fn evaluate_simulation(
    policy: &EthTxPolicyConfig,
    request: &DirectRawTransactionRequest,
    latest_block_number: Option<u64>,
    normalized: &mut EthTxPolicyNormalized,
    reasons: &mut Vec<String>,
) {
    let Some(simulation) = request.simulation.as_ref() else {
        if policy.require_simulation {
            reasons.push("simulation reference is required".to_string());
        }
        return;
    };

    normalized.simulation_block_number = Some(simulation.block_number);

    if let Some(latest) = latest_block_number {
        if simulation.block_number > latest {
            reasons.push(format!(
                "simulation block {} is ahead of latest block {}",
                simulation.block_number, latest
            ));
            return;
        }

        let age = latest - simulation.block_number;
        if age > policy.max_simulation_age_blocks {
            reasons.push(format!(
                "simulation age {} blocks exceeds policy cap {}",
                age, policy.max_simulation_age_blocks
            ));
        }
    } else if policy.require_simulation {
        reasons
            .push("latest block number is required to validate simulation freshness".to_string());
    }
}

fn evaluate_metadata(
    policy: &EthTxPolicyConfig,
    metadata: &Value,
    normalized: &mut EthTxPolicyNormalized,
    reasons: &mut Vec<String>,
) {
    let Some(object) = metadata.as_object() else {
        reasons.push("metadata must be a JSON object".to_string());
        return;
    };

    for field in &policy.required_metadata_fields {
        if metadata_string(metadata, field).is_none() {
            reasons.push(format!("metadata field {field:?} is required"));
        }
    }

    normalized.strategy_name = metadata_string(metadata, "strategy_name");
    normalized.strategy_run_id = metadata_string(metadata, "strategy_run_id");
    normalized.trade_id = metadata_string(metadata, "trade_id");
    normalized.token_address = metadata_string(metadata, "token_address");
    normalized.pool_address = metadata_string(metadata, "pool_address");

    if let Some(wire_protocol) = object.get("wire_protocol").and_then(Value::as_str) {
        let wire_protocol = wire_protocol.trim();
        if wire_protocol != ETH_UNSIGNED_TX_WIRE_PROTOCOL {
            reasons.push(format!(
                "metadata wire_protocol must be {ETH_UNSIGNED_TX_WIRE_PROTOCOL}"
            ));
        }
    }
}

pub fn metadata_string(metadata: &Value, field: &str) -> Option<String> {
    metadata
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_address_like(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.len() == 40 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(format!("0x{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

fn contains_normalized_address(values: &[String], needle: &str) -> bool {
    values
        .iter()
        .filter_map(|value| normalize_address_like(value))
        .any(|value| value == needle)
}

fn normalize_selector(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.len() == 8 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(format!("0x{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

fn contains_normalized_selector(values: &[String], needle: &str) -> bool {
    values
        .iter()
        .filter_map(|value| normalize_selector(value))
        .any(|value| value == needle)
}

fn calldata_selector(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.len() < 8 || !hex.chars().take(8).all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    Some(format!("0x{}", hex[..8].to_ascii_lowercase()))
}

fn parse_quantity_u128(value: &str) -> Result<u128, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("empty quantity".to_string());
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        u128::from_str_radix(hex, 16).map_err(|err| err.to_string())
    } else {
        trimmed.parse::<u128>().map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_policy() -> EthTxPolicyConfig {
        EthTxPolicyConfig {
            version: "test".to_string(),
            allowed_from_addresses: vec!["0x0000000000000000000000000000000000000001".to_string()],
            allowed_targets: vec!["0x0000000000000000000000000000000000000002".to_string()],
            allowed_selectors: vec!["0x38ed1739".to_string()],
            max_value_wei: 0,
            max_gas_limit: 200_000,
            max_fee_per_gas_wei: 100,
            max_priority_fee_per_gas_wei: 10,
            max_transaction_cost_wei: 20_000_000,
            max_daily_cost_wei: 50_000_000,
            require_simulation: true,
            max_simulation_age_blocks: 2,
            required_metadata_fields: vec![
                "wire_protocol".to_string(),
                "intent_kind".to_string(),
                "strategy_name".to_string(),
                "strategy_run_id".to_string(),
                "trade_id".to_string(),
                "token_address".to_string(),
                "pool_address".to_string(),
            ],
        }
    }

    fn base_request() -> DirectRawTransactionRequest {
        DirectRawTransactionRequest {
            attempt_id: Some("attempt-1".to_string()),
            chain_id: 1,
            from: "0x0000000000000000000000000000000000000001".to_string(),
            to: "0x0000000000000000000000000000000000000002".to_string(),
            value: "0".to_string(),
            data: "0x38ed173900000000".to_string(),
            gas_limit: "100000".to_string(),
            max_fee_per_gas: "100".to_string(),
            max_priority_fee_per_gas: "10".to_string(),
            nonce: None,
            bribe: None,
            simulation: Some(tx_executor::SimulationReference {
                block_number: 100,
                block_hash: None,
                state_root: None,
                expected_output_token: None,
                expected_output_amount: None,
                min_output_amount: None,
                metadata: json!({}),
            }),
            metadata: json!({
                "wire_protocol": ETH_UNSIGNED_TX_WIRE_PROTOCOL,
                "intent_kind": "priority_sell",
                "strategy_name": "strategy",
                "strategy_run_id": "run",
                "trade_id": "trade",
                "token_address": "0x0000000000000000000000000000000000000003",
                "pool_address": "0x0000000000000000000000000000000000000004"
            }),
        }
    }

    #[test]
    fn accepts_valid_request() {
        let evaluation = evaluate_eth_tx_policy(&base_policy(), &base_request(), Some(101));
        assert!(evaluation.is_accepted(), "{:?}", evaluation.reasons);
        assert_eq!(
            evaluation
                .normalized
                .estimated_worst_case_cost_wei
                .as_deref(),
            Some("10000000")
        );
    }

    #[test]
    fn rejects_unlisted_target() {
        let mut request = base_request();
        request.to = "0x00000000000000000000000000000000000000ff".to_string();
        let evaluation = evaluate_eth_tx_policy(&base_policy(), &request, Some(101));
        assert!(!evaluation.is_accepted());
        assert!(evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("target")));
    }

    #[test]
    fn rejects_missing_from_allowlist() {
        let mut policy = base_policy();
        policy.allowed_from_addresses = Vec::new();
        let evaluation = evaluate_eth_tx_policy(&policy, &base_request(), Some(101));
        assert!(!evaluation.is_accepted());
        assert!(evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("from address allowlist")));
    }

    #[test]
    fn rejects_unlisted_selector() {
        let mut request = base_request();
        request.data = "0x095ea7b300000000".to_string();
        let evaluation = evaluate_eth_tx_policy(&base_policy(), &request, Some(101));
        assert!(!evaluation.is_accepted());
        assert!(evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("selector")));
    }

    #[test]
    fn rejects_value_gas_and_fee_caps() {
        let mut request = base_request();
        request.value = "1".to_string();
        request.gas_limit = "200001".to_string();
        request.max_fee_per_gas = "101".to_string();
        request.max_priority_fee_per_gas = "11".to_string();
        let evaluation = evaluate_eth_tx_policy(&base_policy(), &request, Some(101));
        assert!(!evaluation.is_accepted());
        assert!(evaluation.reasons.len() >= 4);
    }

    #[test]
    fn rejects_stale_simulation() {
        let evaluation = evaluate_eth_tx_policy(&base_policy(), &base_request(), Some(103));
        assert!(!evaluation.is_accepted());
        assert!(evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("simulation age")));
    }

    #[test]
    fn rejects_missing_metadata() {
        let mut request = base_request();
        request.metadata = json!({});
        let evaluation = evaluate_eth_tx_policy(&base_policy(), &request, Some(101));
        assert!(!evaluation.is_accepted());
        assert!(evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("strategy_name")));
    }

    #[test]
    fn rejects_missing_spend_cap() {
        let mut policy = base_policy();
        policy.max_transaction_cost_wei = 0;
        let evaluation = evaluate_eth_tx_policy(&policy, &base_request(), Some(101));
        assert!(!evaluation.is_accepted());
        assert!(evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("max transaction cost")));
    }

    #[test]
    fn disabled_evaluation_never_accepts() {
        let evaluation =
            EthTxPolicyEvaluation::disabled_from("attempt-1", "test", "kill switch active");
        assert!(!evaluation.is_accepted());
        assert_eq!(evaluation.outcome.as_str(), "disabled");
    }
}
