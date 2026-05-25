use alloy_primitives::U256;
use async_trait::async_trait;
use eth_alpha_core::amount::DecimalAmount;
use eth_block_tx_rank::{estimate_from_samples, CandidateTxGas, MinedBlockFeeSample};
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

const DEFAULT_PRIORITY_TIE_BREAKER_GWEI: &str = "0.1456";
const GAS_RANK_SOURCE: &str = "eth_chain_server_gas_rank";
const WEI_PER_GWEI: u64 = 1_000_000_000;
const EIP1559_ELASTICITY_MULTIPLIER: u64 = 2;
const EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;

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
    priority_tie_breaker_gwei: DecimalAmount,
}

impl ChainServerGasRankProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            lookback_blocks: 100,
            priority_tie_breaker_gwei: DecimalAmount::from_str_exact(
                DEFAULT_PRIORITY_TIE_BREAKER_GWEI,
            )
            .unwrap_or_default(),
        }
    }

    pub fn with_lookback_blocks(mut self, lookback_blocks: u64) -> Self {
        self.lookback_blocks = lookback_blocks.clamp(1, 100);
        self
    }

    pub fn with_priority_tie_breaker_gwei(
        mut self,
        priority_tie_breaker_gwei: DecimalAmount,
    ) -> Self {
        self.priority_tie_breaker_gwei = priority_tie_breaker_gwei.max(DecimalAmount::ZERO);
        self
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/api/v1/eth/alpha/gas-rank/samples?limit={}",
            self.base_url.trim_end_matches('/'),
            self.lookback_blocks
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
        let response = self
            .http
            .get(self.endpoint())
            .send()
            .await
            .map_err(|error| {
                LivePrioritySellPlannerError::GasRank(format!(
                    "eth_chain_server gas-rank samples request failed: {error}"
                ))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            LivePrioritySellPlannerError::GasRank(format!(
                "eth_chain_server gas-rank samples response body read failed: {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(LivePrioritySellPlannerError::GasRank(format!(
                "eth_chain_server gas-rank samples returned HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }
        let payload =
            serde_json::from_str::<ChainServerGasRankSamplesResponse>(&body).map_err(|error| {
                LivePrioritySellPlannerError::GasRank(format!(
                    "eth_chain_server gas-rank samples JSON decode failed: {error}; body={body}"
                ))
            })?;
        gas_rank_plan_from_processed_block_samples(
            payload,
            route.gas_limit,
            estimated_gas_used,
            self.priority_tie_breaker_gwei,
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
struct ChainServerGasRankSamplesResponse {
    source: String,
    requested_blocks: usize,
    available_recent_blocks: usize,
    latest_block: Option<u64>,
    latest_block_hash: Option<String>,
    samples: Vec<RecentLiveFeeSampleWire>,
}

#[derive(Clone, Debug, Deserialize)]
struct RecentLiveFeeSampleWire {
    sample: MinedBlockFeeSample,
    applied_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct RecommendationProfile {
    label: &'static str,
    priority: RecommendationPriority,
}

#[derive(Debug, Clone, Copy)]
enum RecommendationPriority {
    TargetPosition {
        target_position: u64,
        sample_quantile: f64,
    },
    PriorityPercentile {
        percentile: f64,
    },
}

const RECOMMENDATION_PROFILES: [RecommendationProfile; 20] = [
    RecommendationProfile {
        label: "normal",
        priority: RecommendationPriority::TargetPosition {
            target_position: 50,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "rank25",
        priority: RecommendationPriority::TargetPosition {
            target_position: 25,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "rank10",
        priority: RecommendationPriority::TargetPosition {
            target_position: 10,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "rank5",
        priority: RecommendationPriority::TargetPosition {
            target_position: 5,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "p50",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.50 },
    },
    RecommendationProfile {
        label: "p55",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.55 },
    },
    RecommendationProfile {
        label: "p60",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.60 },
    },
    RecommendationProfile {
        label: "p65",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.65 },
    },
    RecommendationProfile {
        label: "p70",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.70 },
    },
    RecommendationProfile {
        label: "p75",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.75 },
    },
    RecommendationProfile {
        label: "p77",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.77 },
    },
    RecommendationProfile {
        label: "p85",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.85 },
    },
    RecommendationProfile {
        label: "p88",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.88 },
    },
    RecommendationProfile {
        label: "p90",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.90 },
    },
    RecommendationProfile {
        label: "p92",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.92 },
    },
    RecommendationProfile {
        label: "p94",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.94 },
    },
    RecommendationProfile {
        label: "p95",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.95 },
    },
    RecommendationProfile {
        label: "p96",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.96 },
    },
    RecommendationProfile {
        label: "p97",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.97 },
    },
    RecommendationProfile {
        label: "p99",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.99 },
    },
];

fn gas_rank_plan_from_processed_block_samples(
    response: ChainServerGasRankSamplesResponse,
    gas_limit: u64,
    estimated_gas_used: u64,
    priority_tie_breaker_gwei: DecimalAmount,
) -> Result<GasRankPlan, LivePrioritySellPlannerError> {
    let samples = response
        .samples
        .iter()
        .map(|entry| entry.sample.clone())
        .collect::<Vec<_>>();
    if samples.is_empty() {
        return Err(gas_rank_parse_error(
            "eth_chain_server returned no live fee samples from processed blocks",
        ));
    }
    let latest_sample = samples
        .iter()
        .max_by_key(|sample| sample.block_number)
        .ok_or_else(|| gas_rank_parse_error("missing latest live fee sample"))?;
    let predicted_base_fee = predict_next_base_fee_from_sample(latest_sample);
    let priority_tie_breaker =
        decimal_gwei_to_wei(priority_tie_breaker_gwei, "priority_tie_breaker_gwei")?;
    let predicted_base_fee_gwei = wei_to_decimal_gwei(predicted_base_fee)?;
    let candidates = RECOMMENDATION_PROFILES
        .iter()
        .copied()
        .map(|profile| {
            ranked_fee_candidate_from_profile(
                profile,
                gas_limit,
                estimated_gas_used,
                predicted_base_fee,
                priority_tie_breaker,
                &samples,
                &response,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(GasRankPlan {
        predicted_base_fee_gwei,
        candidates,
    })
}

fn ranked_fee_candidate_from_profile(
    profile: RecommendationProfile,
    gas_limit: u64,
    estimated_gas_used: u64,
    predicted_base_fee: U256,
    priority_tie_breaker: U256,
    samples: &[MinedBlockFeeSample],
    response: &ChainServerGasRankSamplesResponse,
) -> Result<RankedFeeCandidate, LivePrioritySellPlannerError> {
    let raw_priority_fee = match profile.priority {
        RecommendationPriority::TargetPosition {
            target_position,
            sample_quantile,
        } => recommended_priority(samples, target_position, sample_quantile),
        RecommendationPriority::PriorityPercentile { percentile } => {
            recommended_priority_percentile(samples, percentile)
        }
    };
    let priority_fee = raw_priority_fee.saturating_add(priority_tie_breaker);
    let max_fee = suggested_max_fee(predicted_base_fee, priority_fee);
    let rank = estimate_from_samples(
        CandidateTxGas {
            gas_limit,
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: priority_fee,
        },
        predicted_base_fee,
        samples,
    )
    .map_err(|error| gas_rank_parse_error(format!("failed to estimate gas rank: {error}")))?;

    Ok(RankedFeeCandidate {
        label: profile.label.to_string(),
        priority_fee_gwei: wei_to_decimal_gwei(priority_fee)?,
        max_fee_per_gas_gwei: wei_to_decimal_gwei(max_fee)?,
        rank_position_p50: Some(rank.position_p50),
        gas_before_p50: Some(rank.gas_before_p50),
        likely_fits_at_p50: Some(rank.likely_fits_at_p50),
        source: Some(GAS_RANK_SOURCE.to_string()),
        metadata: Some(json!({
            "method": "local_estimate_from_processed_block_fee_samples",
            "sample_source": response.source.as_str(),
            "requested_blocks": response.requested_blocks,
            "available_recent_blocks": response.available_recent_blocks,
            "latest_block": response.latest_block,
            "latest_block_hash": response.latest_block_hash.as_deref(),
            "latest_sample_applied_at_unix_ms": response.samples.first().map(|entry| entry.applied_at_unix_ms),
            "sampled_blocks": samples.len(),
            "sampled_transactions": samples.iter().map(|sample| sample.transactions.len()).sum::<usize>(),
            "estimated_gas_used": estimated_gas_used,
            "raw_priority_fee_wei": raw_priority_fee.to_string(),
            "priority_tie_breaker_wei": priority_tie_breaker.to_string(),
            "predicted_base_fee_wei": predicted_base_fee.to_string(),
        })),
    })
}

fn recommended_priority(
    samples: &[MinedBlockFeeSample],
    target_position: u64,
    sample_quantile: f64,
) -> U256 {
    let values = samples
        .iter()
        .map(|sample| priority_for_target_position(sample, target_position))
        .collect::<Vec<_>>();
    quantile_u256(values, sample_quantile)
}

fn recommended_priority_percentile(samples: &[MinedBlockFeeSample], percentile: f64) -> U256 {
    let values = samples
        .iter()
        .flat_map(|sample| {
            sample
                .transactions
                .iter()
                .map(|tx| tx.effective_priority_fee)
        })
        .collect::<Vec<_>>();
    quantile_u256(values, percentile)
}

fn priority_for_target_position(sample: &MinedBlockFeeSample, target_position: u64) -> U256 {
    let mut priorities = sample
        .transactions
        .iter()
        .map(|tx| tx.effective_priority_fee)
        .collect::<Vec<_>>();
    if priorities.is_empty() {
        return U256::ZERO;
    }
    priorities.sort_unstable_by(|left, right| right.cmp(left));
    let index = target_position.saturating_sub(1) as usize;
    if index >= priorities.len() {
        U256::ZERO
    } else {
        priorities[index].saturating_add(U256::from(1u64))
    }
}

fn quantile_u256(mut values: Vec<U256>, quantile: f64) -> U256 {
    if values.is_empty() {
        return U256::ZERO;
    }
    values.sort_unstable();
    let bounded = quantile.clamp(0.0, 1.0);
    let index = ((values.len().saturating_sub(1) as f64) * bounded).round() as usize;
    values[index]
}

fn suggested_max_fee(predicted_base_fee: U256, priority_fee: U256) -> U256 {
    let base_fee_cushion = predicted_base_fee + (predicted_base_fee / U256::from(8u64));
    base_fee_cushion + priority_fee
}

fn predict_next_base_fee_from_sample(sample: &MinedBlockFeeSample) -> U256 {
    let parent_base_fee = sample.base_fee_per_gas;
    let parent_gas_target = sample.gas_limit / EIP1559_ELASTICITY_MULTIPLIER;
    if parent_gas_target == 0 || sample.gas_used == parent_gas_target {
        return parent_base_fee;
    }

    if sample.gas_used > parent_gas_target {
        let gas_used_delta = U256::from(sample.gas_used - parent_gas_target);
        let base_fee_delta = (parent_base_fee * gas_used_delta)
            / U256::from(parent_gas_target)
            / U256::from(EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR);
        parent_base_fee + base_fee_delta.max(U256::from(1u64))
    } else {
        let gas_used_delta = U256::from(parent_gas_target - sample.gas_used);
        let base_fee_delta = (parent_base_fee * gas_used_delta)
            / U256::from(parent_gas_target)
            / U256::from(EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR);
        parent_base_fee.saturating_sub(base_fee_delta)
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
    Ok(wei_decimal / DecimalAmount::from(WEI_PER_GWEI))
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

fn gas_rank_parse_error(message: impl Into<String>) -> LivePrioritySellPlannerError {
    LivePrioritySellPlannerError::GasRank(message.into())
}

#[cfg(test)]
mod chain_server_tests {
    use super::*;
    use alloy_primitives::B256;
    use eth_block_tx_rank::MinedTxFeeSample;

    #[test]
    fn builds_ranked_candidates_from_processed_block_samples() {
        let response = ChainServerGasRankSamplesResponse {
            source: "eth_chain_server_recent_live_fee_samples_from_processed_transactions"
                .to_string(),
            requested_blocks: 1,
            available_recent_blocks: 1,
            latest_block: Some(100),
            latest_block_hash: Some(B256::ZERO.to_string()),
            samples: vec![fee_sample_wire(100, &[1, 2, 3])],
        };

        let plan = gas_rank_plan_from_processed_block_samples(
            response,
            120_000,
            100_000,
            DecimalAmount::new(1, 1),
        )
        .unwrap();

        let p90 = plan
            .candidates
            .iter()
            .find(|candidate| candidate.label == "p90")
            .expect("p90 candidate");
        assert_eq!(plan.predicted_base_fee_gwei.to_string(), "0.10");
        assert_eq!(p90.priority_fee_gwei.to_string(), "3.10");
        assert_eq!(p90.max_fee_per_gas_gwei.to_string(), "3.2125");
        assert_eq!(p90.source.as_deref(), Some(GAS_RANK_SOURCE));
        assert_eq!(
            p90.metadata
                .as_ref()
                .and_then(|value| value.get("method"))
                .and_then(Value::as_str),
            Some("local_estimate_from_processed_block_fee_samples")
        );
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

    fn fee_sample_wire(block_number: u64, priority_fees_gwei: &[u64]) -> RecentLiveFeeSampleWire {
        let base_fee = U256::from(100_000_000u64);
        RecentLiveFeeSampleWire {
            sample: MinedBlockFeeSample {
                block_number,
                block_hash: B256::ZERO,
                gas_limit: 30_000_000,
                gas_used: 15_000_000,
                base_fee_per_gas: base_fee,
                transactions: priority_fees_gwei
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, priority_gwei)| {
                        let priority = U256::from(priority_gwei * WEI_PER_GWEI);
                        MinedTxFeeSample {
                            tx_hash: B256::ZERO,
                            tx_index: index as u64,
                            gas_limit: 21_000,
                            gas_used: 21_000,
                            effective_gas_price: base_fee + priority,
                            effective_priority_fee: priority,
                        }
                    })
                    .collect(),
            },
            applied_at_unix_ms: 1_000,
        }
    }
}
