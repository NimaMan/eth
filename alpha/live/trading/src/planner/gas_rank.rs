use async_trait::async_trait;
use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{PreparedSellRoute, RankedFeeCandidate};

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
}
