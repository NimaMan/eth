use alloy_primitives::U256;
use async_trait::async_trait;
use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    tx_prep::{gwei_to_wei_string, MEMPOOL_RACE_GAS_LABEL, MEMPOOL_RACE_GAS_SOURCE},
    PreparedSellRoute, RankedFeeCandidate,
};

use super::{LivePrioritySellPlannerError, LivePrioritySellPlannerInput};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasRankPlan {
    pub predicted_base_fee_gwei: DecimalAmount,
    pub candidates: Vec<RankedFeeCandidate>,
}

#[async_trait]
pub trait GasRankProvider: Send + Sync {
    async fn ranked_fee_candidates(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: &PreparedSellRoute,
    ) -> Result<GasRankPlan, LivePrioritySellPlannerError>;
}

#[derive(Clone, Debug)]
pub struct FixedGasRankProvider {
    plan: GasRankPlan,
}

impl FixedGasRankProvider {
    pub fn new(plan: GasRankPlan) -> Self {
        Self { plan }
    }
}

#[async_trait]
impl GasRankProvider for FixedGasRankProvider {
    async fn ranked_fee_candidates(
        &self,
        _input: &LivePrioritySellPlannerInput,
        _route: &PreparedSellRoute,
    ) -> Result<GasRankPlan, LivePrioritySellPlannerError> {
        Ok(self.plan.clone())
    }
}

#[derive(Clone, Debug)]
pub struct ChainServerGasRankProvider {
    http: reqwest::Client,
    base_url: String,
    lookback_blocks: u64,
}

impl ChainServerGasRankProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            lookback_blocks: 100,
        }
    }

    pub fn with_lookback_blocks(mut self, lookback_blocks: u64) -> Self {
        self.lookback_blocks = lookback_blocks.clamp(1, 100);
        self
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/api/v1/eth/alpha/gas-rank/estimate",
            self.base_url.trim_end_matches('/')
        )
    }
}

#[async_trait]
impl GasRankProvider for ChainServerGasRankProvider {
    async fn ranked_fee_candidates(
        &self,
        _input: &LivePrioritySellPlannerInput,
        route: &PreparedSellRoute,
    ) -> Result<GasRankPlan, LivePrioritySellPlannerError> {
        let estimated_gas_used = route.require_estimated_gas_used().map_err(|error| {
            LivePrioritySellPlannerError::GasRank(format!(
                "cannot request gas rank without simulation gas evidence: {error}"
            ))
        })?;
        let request = json!({
            "gas_limit": route.gas_limit,
            "estimated_gas_used": estimated_gas_used,
            "lookback_blocks": self.lookback_blocks,
        });
        let response = self
            .http
            .post(self.endpoint())
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                LivePrioritySellPlannerError::GasRank(format!(
                    "eth_chain_server gas-rank request failed: {error}"
                ))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            LivePrioritySellPlannerError::GasRank(format!(
                "eth_chain_server gas-rank response body read failed: {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(LivePrioritySellPlannerError::GasRank(format!(
                "eth_chain_server gas-rank returned HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }
        let payload = serde_json::from_str::<Value>(&body).map_err(|error| {
            LivePrioritySellPlannerError::GasRank(format!(
                "eth_chain_server gas-rank JSON decode failed: {error}; body={body}"
            ))
        })?;
        gas_rank_plan_from_chain_server_response(&payload)
    }
}

#[derive(Clone, Debug)]
pub struct MempoolRaceGasRankProvider<G> {
    inner: G,
    http: reqwest::Client,
    rpc_url: String,
    priority_buffer_min_gwei: DecimalAmount,
    priority_buffer_max_gwei: DecimalAmount,
}

impl<G> MempoolRaceGasRankProvider<G> {
    pub fn new(inner: G, rpc_url: impl Into<String>) -> Self {
        Self {
            inner,
            http: reqwest::Client::new(),
            rpc_url: rpc_url.into(),
            priority_buffer_min_gwei: DecimalAmount::new(1, 1),
            priority_buffer_max_gwei: DecimalAmount::new(2, 1),
        }
    }

    pub fn with_priority_buffer_gwei(mut self, priority_buffer_gwei: DecimalAmount) -> Self {
        let priority_buffer_gwei = priority_buffer_gwei.max(DecimalAmount::ZERO);
        self.priority_buffer_min_gwei = priority_buffer_gwei;
        self.priority_buffer_max_gwei = priority_buffer_gwei;
        self
    }

    pub fn with_priority_buffer_range_gwei(
        mut self,
        min_gwei: DecimalAmount,
        max_gwei: DecimalAmount,
    ) -> Self {
        let min_gwei = min_gwei.max(DecimalAmount::ZERO);
        let max_gwei = max_gwei.max(min_gwei);
        self.priority_buffer_min_gwei = min_gwei;
        self.priority_buffer_max_gwei = max_gwei;
        self
    }
}

#[async_trait]
impl<G> GasRankProvider for MempoolRaceGasRankProvider<G>
where
    G: GasRankProvider,
{
    async fn ranked_fee_candidates(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: &PreparedSellRoute,
    ) -> Result<GasRankPlan, LivePrioritySellPlannerError> {
        let mut plan = self.inner.ranked_fee_candidates(input, route).await?;
        let Some(race_context) = mempool_race_context(input) else {
            return Ok(plan);
        };

        let tx = self
            .fetch_transaction_by_hash(&race_context.tx_hash)
            .await?;
        let priority_buffer_gwei = deterministic_priority_buffer_gwei(
            &race_context.tx_hash,
            self.priority_buffer_min_gwei,
            self.priority_buffer_max_gwei,
        );
        let candidate = mempool_race_candidate_from_tx_json(
            &race_context,
            &tx,
            plan.predicted_base_fee_gwei,
            self.priority_buffer_min_gwei,
            self.priority_buffer_max_gwei,
            priority_buffer_gwei,
        )?;
        plan.candidates.insert(0, candidate);
        Ok(plan)
    }
}

impl<G> MempoolRaceGasRankProvider<G> {
    async fn fetch_transaction_by_hash(
        &self,
        tx_hash: &str,
    ) -> Result<Value, LivePrioritySellPlannerError> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getTransactionByHash",
            "params": [tx_hash],
        });
        let response = self
            .http
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                LivePrioritySellPlannerError::GasRank(format!(
                    "pending mempool-race tx fee lookup failed: {error}"
                ))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            LivePrioritySellPlannerError::GasRank(format!(
                "pending mempool-race tx fee lookup body read failed: {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(LivePrioritySellPlannerError::GasRank(format!(
                "pending mempool-race tx fee lookup returned HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }
        let payload = serde_json::from_str::<Value>(&body).map_err(|error| {
            LivePrioritySellPlannerError::GasRank(format!(
                "pending mempool-race tx fee lookup JSON decode failed: {error}; body={body}"
            ))
        })?;
        if let Some(error) = payload.get("error").filter(|error| !error.is_null()) {
            return Err(LivePrioritySellPlannerError::GasRank(format!(
                "pending mempool-race tx fee lookup RPC error: {error}"
            )));
        }
        payload
            .get("result")
            .filter(|result| !result.is_null())
            .cloned()
            .ok_or_else(|| {
                LivePrioritySellPlannerError::GasRank(format!(
                    "pending mempool-race tx {tx_hash} was not found by eth_getTransactionByHash"
                ))
            })
    }
}

fn gas_rank_plan_from_chain_server_response(
    payload: &Value,
) -> Result<GasRankPlan, LivePrioritySellPlannerError> {
    let candidate = payload
        .get("candidate")
        .ok_or_else(|| gas_rank_parse_error("missing candidate"))?;
    let predicted_base_fee_gwei = decimal_field(candidate, "predicted_base_fee_gwei", "candidate")?;
    let recommendations = payload
        .get("recommendations")
        .and_then(Value::as_array)
        .ok_or_else(|| gas_rank_parse_error("missing recommendations"))?;
    let mut candidates = Vec::with_capacity(recommendations.len());
    for recommendation in recommendations {
        let label = string_field(recommendation, "label", "recommendation")?.to_ascii_lowercase();
        let rank = recommendation.get("rank").unwrap_or(&Value::Null);
        candidates.push(RankedFeeCandidate {
            label,
            priority_fee_gwei: decimal_field(
                recommendation,
                "priority_fee_gwei",
                "recommendation",
            )?,
            max_fee_per_gas_gwei: decimal_field(
                recommendation,
                "max_fee_per_gas_gwei",
                "recommendation",
            )?,
            rank_position_p50: u64_field(rank, "position_p50"),
            gas_before_p50: u64_field(rank, "gas_before_p50"),
            likely_fits_at_p50: bool_field(rank, "likely_fits_at_p50"),
            source: Some("eth_chain_server_gas_rank".to_string()),
            metadata: None,
        });
    }
    if candidates.is_empty() {
        return Err(gas_rank_parse_error(
            "eth_chain_server gas-rank returned no recommendations",
        ));
    }
    Ok(GasRankPlan {
        predicted_base_fee_gwei,
        candidates,
    })
}

#[derive(Clone, Debug)]
struct MempoolRaceContext {
    tx_hash: String,
    signal_kind: &'static str,
}

fn mempool_race_context(input: &LivePrioritySellPlannerInput) -> Option<MempoolRaceContext> {
    if input.intent.side != eth_alpha_core::order::OrderSide::Sell {
        return None;
    }
    let reason = input.intent.decision_reason.as_ref()?;
    let source = reason
        .source
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let evidence_source = string_at(&reason.details, "/risk_event_evidence/signal_source")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let is_mempool = source.contains("mempool") || evidence_source.contains("mempool");
    let signal_kind = match reason.code.as_str() {
        "exit.mempool_liquidity_removal_signal" => "liquidity_removal",
        "exit.lp_approval" if is_mempool => "lp_approval",
        _ => return None,
    };
    Some(MempoolRaceContext {
        tx_hash: mempool_dependency_tx_hash(input)?,
        signal_kind,
    })
}

fn mempool_dependency_tx_hash(input: &LivePrioritySellPlannerInput) -> Option<String> {
    input
        .intent
        .decision_reason
        .as_ref()
        .and_then(|reason| {
            string_at(&reason.details, "/pending_tx_hash")
                .or_else(|| string_at(&reason.details, "/risk_event_evidence/detection_tx_hash"))
                .or_else(|| string_at(&reason.details, "/risk_event_evidence/pending_tx_hash"))
        })
        .or_else(|| {
            string_at(
                &input.source_metadata,
                "/decision_reason/details/pending_tx_hash",
            )
            .or_else(|| {
                string_at(
                    &input.source_metadata,
                    "/decision_reason/details/risk_event_evidence/detection_tx_hash",
                )
            })
            .or_else(|| {
                string_at(
                    &input.source_metadata,
                    "/decision_reason/details/risk_event_evidence/pending_tx_hash",
                )
            })
        })
}

fn string_at(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn mempool_race_candidate_from_tx_json(
    context: &MempoolRaceContext,
    tx: &Value,
    predicted_base_fee_gwei: DecimalAmount,
    priority_buffer_min_gwei: DecimalAmount,
    priority_buffer_max_gwei: DecimalAmount,
    priority_buffer_gwei: DecimalAmount,
) -> Result<RankedFeeCandidate, LivePrioritySellPlannerError> {
    let base_fee_wei = decimal_gwei_to_wei(predicted_base_fee_gwei, "predicted_base_fee_gwei")?;
    let buffer_wei = decimal_gwei_to_wei(priority_buffer_gwei, "priority_buffer_gwei")?;
    let gas_price = tx_quantity(tx, "gasPrice")?;
    let max_fee_per_gas = tx_quantity(tx, "maxFeePerGas")?
        .or(gas_price)
        .ok_or_else(|| {
            gas_rank_parse_error("pending mempool-race tx missing maxFeePerGas/gasPrice")
        })?;
    let max_priority_fee_per_gas = tx_quantity(tx, "maxPriorityFeePerGas")?
        .or(gas_price)
        .ok_or_else(|| {
            gas_rank_parse_error("pending mempool-race tx missing maxPriorityFeePerGas/gasPrice")
        })?;
    let dependency_effective_priority =
        effective_priority_fee(max_fee_per_gas, max_priority_fee_per_gas, base_fee_wei);
    let required_priority_fee = dependency_effective_priority + buffer_wei + U256::from(1u64);
    let required_max_fee_per_gas = base_fee_wei + required_priority_fee;

    Ok(RankedFeeCandidate {
        label: MEMPOOL_RACE_GAS_LABEL.to_string(),
        priority_fee_gwei: wei_to_decimal_gwei(required_priority_fee)?,
        max_fee_per_gas_gwei: wei_to_decimal_gwei(required_max_fee_per_gas)?,
        rank_position_p50: Some(0),
        gas_before_p50: Some(0),
        likely_fits_at_p50: Some(true),
        source: Some(MEMPOOL_RACE_GAS_SOURCE.to_string()),
        metadata: Some(json!({
            "method": "eth_getTransactionByHash",
            "dependency_signal_kind": context.signal_kind,
            "dependency_tx_hash": context.tx_hash.as_str(),
            "dependency_tx_type": tx.get("type").and_then(Value::as_str),
            "dependency_gas_price_wei": gas_price.map(|value| value.to_string()),
            "dependency_max_fee_per_gas_wei": max_fee_per_gas.to_string(),
            "dependency_max_priority_fee_per_gas_wei": max_priority_fee_per_gas.to_string(),
            "dependency_effective_priority_fee_wei": dependency_effective_priority.to_string(),
            "dependency_effective_priority_fee_gwei": wei_to_decimal_gwei(dependency_effective_priority)?,
            "removal_tx_hash": if context.signal_kind == "liquidity_removal" { Some(context.tx_hash.clone()) } else { None },
            "removal_effective_priority_fee_wei": if context.signal_kind == "liquidity_removal" { Some(dependency_effective_priority.to_string()) } else { None },
            "priority_buffer_min_gwei": priority_buffer_min_gwei,
            "priority_buffer_max_gwei": priority_buffer_max_gwei,
            "priority_buffer_gwei": priority_buffer_gwei,
            "selected_priority_fee_wei": required_priority_fee.to_string(),
            "selected_max_fee_per_gas_wei": required_max_fee_per_gas.to_string(),
            "predicted_base_fee_gwei": predicted_base_fee_gwei,
            "predicted_base_fee_wei": base_fee_wei.to_string(),
        })),
    })
}

fn deterministic_priority_buffer_gwei(
    tx_hash: &str,
    min_gwei: DecimalAmount,
    max_gwei: DecimalAmount,
) -> DecimalAmount {
    let min_gwei = min_gwei.max(DecimalAmount::ZERO);
    let max_gwei = max_gwei.max(min_gwei);
    if max_gwei <= min_gwei {
        return min_gwei;
    }
    let bucket = DecimalAmount::from(tx_hash_jitter_bucket(tx_hash));
    let denominator = DecimalAmount::from(9_999u64);
    min_gwei + ((max_gwei - min_gwei) * bucket / denominator)
}

fn tx_hash_jitter_bucket(tx_hash: &str) -> u64 {
    let hex = tx_hash.trim().strip_prefix("0x").unwrap_or(tx_hash.trim());
    let mut value = 0u64;
    let mut digits = 0u8;
    for ch in hex.chars().take(16) {
        let Some(digit) = ch.to_digit(16) else {
            continue;
        };
        value = (value << 4) | u64::from(digit);
        digits += 1;
    }
    if digits == 0 {
        0
    } else {
        value % 10_000
    }
}

fn tx_quantity(tx: &Value, field: &str) -> Result<Option<U256>, LivePrioritySellPlannerError> {
    tx.get(field)
        .and_then(Value::as_str)
        .map(parse_hex_quantity)
        .transpose()
}

fn parse_hex_quantity(value: &str) -> Result<U256, LivePrioritySellPlannerError> {
    let hex = value
        .trim()
        .strip_prefix("0x")
        .unwrap_or_else(|| value.trim());
    if hex.is_empty() {
        return Ok(U256::ZERO);
    }
    U256::from_str_radix(hex, 16)
        .map_err(|error| gas_rank_parse_error(format!("invalid hex quantity {value:?}: {error}")))
}

fn decimal_gwei_to_wei(
    gwei: DecimalAmount,
    label: &str,
) -> Result<U256, LivePrioritySellPlannerError> {
    U256::from_str_radix(&gwei_to_wei_string(gwei), 10)
        .map_err(|error| gas_rank_parse_error(format!("invalid {label}: {error}")))
}

fn wei_to_decimal_gwei(wei: U256) -> Result<DecimalAmount, LivePrioritySellPlannerError> {
    let wei_decimal = DecimalAmount::from_str_exact(&wei.to_string()).map_err(|error| {
        gas_rank_parse_error(format!(
            "cannot convert wei amount {wei} to decimal: {error}"
        ))
    })?;
    Ok(wei_decimal / DecimalAmount::from(1_000_000_000u64))
}

fn effective_priority_fee(
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
    base_fee_per_gas: U256,
) -> U256 {
    if max_fee_per_gas <= base_fee_per_gas {
        return U256::ZERO;
    }
    max_priority_fee_per_gas.min(max_fee_per_gas - base_fee_per_gas)
}

fn decimal_field(
    value: &Value,
    field: &str,
    context: &str,
) -> Result<DecimalAmount, LivePrioritySellPlannerError> {
    let raw = value
        .get(field)
        .ok_or_else(|| gas_rank_parse_error(format!("missing {context}.{field}")))?;
    let text = match raw {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        other => {
            return Err(gas_rank_parse_error(format!(
                "invalid {context}.{field} decimal value: {other}"
            )));
        }
    };
    text.parse::<DecimalAmount>().map_err(|error| {
        gas_rank_parse_error(format!(
            "invalid {context}.{field} decimal value {text:?}: {error}"
        ))
    })
}

fn string_field<'a>(
    value: &'a Value,
    field: &str,
    context: &str,
) -> Result<&'a str, LivePrioritySellPlannerError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| gas_rank_parse_error(format!("missing {context}.{field}")))
}

fn u64_field(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(Value::as_u64)
}

fn bool_field(value: &Value, field: &str) -> Option<bool> {
    value.get(field).and_then(Value::as_bool)
}

fn gas_rank_parse_error(message: impl Into<String>) -> LivePrioritySellPlannerError {
    LivePrioritySellPlannerError::GasRank(message.into())
}

#[cfg(test)]
mod chain_server_tests {
    use super::*;

    #[test]
    fn parses_chain_server_recommendations_into_named_candidates() {
        let payload = json!({
            "candidate": {
                "predicted_base_fee_gwei": 0.112345678
            },
            "recommendations": [
                {
                    "label": "Normal",
                    "priority_fee_gwei": 0.5,
                    "max_fee_per_gas_gwei": 0.626388888,
                    "rank": {
                        "position_p50": 50,
                        "gas_before_p50": 1000000,
                        "likely_fits_at_p50": true
                    }
                },
                {
                    "label": "P90",
                    "priority_fee_gwei": 2,
                    "max_fee_per_gas_gwei": 2.126388888,
                    "rank": {
                        "position_p50": 10,
                        "gas_before_p50": 200000,
                        "likely_fits_at_p50": true
                    }
                }
            ]
        });

        let plan = gas_rank_plan_from_chain_server_response(&payload).unwrap();

        assert_eq!(plan.predicted_base_fee_gwei.to_string(), "0.112345678");
        assert_eq!(plan.candidates.len(), 2);
        assert_eq!(plan.candidates[0].label, "normal");
        assert_eq!(plan.candidates[0].priority_fee_gwei.to_string(), "0.5");
        assert_eq!(plan.candidates[0].rank_position_p50, Some(50));
        assert_eq!(
            plan.candidates[0].source.as_deref(),
            Some("eth_chain_server_gas_rank")
        );
        assert_eq!(plan.candidates[1].label, "p90");
        assert_eq!(plan.candidates[1].priority_fee_gwei.to_string(), "2");
    }

    #[test]
    fn mempool_race_candidate_bids_above_pending_removal_effective_priority() {
        let tx = json!({
            "type": "0x2",
            "gasPrice": "0xb77bfc5f",
            "maxFeePerGas": "0xbef15bbe",
            "maxPriorityFeePerGas": "0xb2d1b5c0"
        });

        let context = MempoolRaceContext {
            tx_hash: "0x0ea03ef6399afa65d1ebbd72580b079157f0e30dd0a16c8fee7e4ec9de2a8fbf"
                .to_string(),
            signal_kind: "liquidity_removal",
        };
        let candidate = mempool_race_candidate_from_tx_json(
            &context,
            &tx,
            DecimalAmount::from_str_exact("0.105479164").unwrap(),
            DecimalAmount::new(1, 1),
            DecimalAmount::new(1, 1),
            DecimalAmount::new(1, 1),
        )
        .unwrap();

        assert_eq!(candidate.label, MEMPOOL_RACE_GAS_LABEL);
        assert_eq!(candidate.source.as_deref(), Some(MEMPOOL_RACE_GAS_SOURCE));
        assert_eq!(candidate.priority_fee_gwei.to_string(), "3.100088001");
        assert_eq!(candidate.max_fee_per_gas_gwei.to_string(), "3.205567165");
        assert_eq!(
            candidate
                .metadata
                .as_ref()
                .and_then(|value| value.get("removal_effective_priority_fee_wei"))
                .and_then(Value::as_str),
            Some("3000088000")
        );
    }

    #[test]
    fn priority_buffer_is_deterministic_and_bounded() {
        let tx_hash = "0x0ea03ef6399afa65d1ebbd72580b079157f0e30dd0a16c8fee7e4ec9de2a8fbf";
        let min = DecimalAmount::new(1, 1);
        let max = DecimalAmount::new(2, 1);

        let left = deterministic_priority_buffer_gwei(tx_hash, min, max);
        let right = deterministic_priority_buffer_gwei(tx_hash, min, max);

        assert_eq!(left, right);
        assert!(left >= min);
        assert!(left <= max);
        assert_ne!(left, min);
    }
}
