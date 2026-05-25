use alloy_primitives::U256;
use eth_block_tx_rank::{
    effective_priority_fee, estimate_from_samples, BlockTxRankEstimate, CandidateTxGas,
    MinedBlockFeeSample,
};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

use crate::app::config::shared_config_value;
use crate::http::ServerState;
use crate::recent_blocks::RecentLiveFeeSample;

const DEFAULT_LOOKBACK_BLOCKS: u64 = 100;
const MAX_LOOKBACK_BLOCKS: u64 = 100;
const DEFAULT_GAS_LIMIT: u64 = 300_000;
const DEFAULT_PRIORITY_FEE_GWEI: f64 = 2.0;
const DEFAULT_PRIORITY_TIE_BREAKER_GWEI: f64 = 0.1456;
const PRIORITY_TIE_BREAKER_GWEI_CONFIG: &str = "ALPHA_GAS_RANK_PRIORITY_TIE_BREAKER_GWEI";
const ETH_USD_PRICE_CONFIG: &str = "ALPHA_GAS_RANK_ETH_USD_PRICE";
const ETH_USD_UNCONFIGURED_SOURCE: &str =
    "ALPHA_GAS_RANK_ETH_USD_PRICE not configured; gas-rank estimate does not query Reth";
const WEI_PER_GWEI: f64 = 1_000_000_000.0;
const WEI_PER_ETH: f64 = 1_000_000_000_000_000_000.0;
const EIP1559_ELASTICITY_MULTIPLIER: u64 = 2;
const EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;

#[derive(Debug, Clone, Deserialize)]
pub struct GasRankEstimateRequest {
    #[serde(default)]
    pub gas_limit: Option<u64>,
    #[serde(default)]
    pub estimated_gas_used: Option<u64>,
    #[serde(default)]
    pub max_fee_per_gas_gwei: Option<f64>,
    #[serde(default)]
    pub max_priority_fee_per_gas_gwei: Option<f64>,
    #[serde(default)]
    pub lookback_blocks: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GasRankSamplesRequest {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GasRankEstimateResponse {
    pub eth_usd_price: f64,
    pub eth_usd_price_source: String,
    pub priority_tie_breaker_gwei: f64,
    pub blocks: RecentBlockWindow,
    pub candidate: CandidateGasRankView,
    pub recommendations: Vec<GasRankRecommendation>,
    pub history: Vec<GasRankHistoryRow>,
    pub skipped_blocks: Vec<SkippedBlock>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GasRankSamplesResponse {
    pub source: &'static str,
    pub requested_blocks: usize,
    pub available_recent_blocks: usize,
    pub latest_block: Option<u64>,
    pub latest_block_hash: Option<String>,
    pub samples: Vec<RecentLiveFeeSample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentBlockWindow {
    pub source: &'static str,
    pub requested_blocks: u64,
    pub available_recent_blocks: usize,
    pub inferred_blocks: usize,
    pub sampled_blocks: usize,
    pub sampled_transactions: usize,
    pub latest_block: Option<u64>,
    pub latest_block_hash: Option<String>,
    pub latest_processed_at_ms: Option<u64>,
    pub block_numbers: Vec<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateGasRankView {
    pub gas_limit: u64,
    pub estimated_gas_used: u64,
    pub max_fee_per_gas_gwei: f64,
    pub max_priority_fee_per_gas_gwei: f64,
    pub predicted_base_fee_gwei: f64,
    pub effective_priority_fee_gwei: f64,
    pub estimated_base_cost_eth: f64,
    pub estimated_base_cost_usd: f64,
    pub estimated_priority_spend_eth: f64,
    pub estimated_priority_spend_usd: f64,
    pub estimated_total_cost_eth: f64,
    pub estimated_total_cost_usd: f64,
    pub estimated_max_cost_eth: f64,
    pub estimated_max_cost_usd: f64,
    pub rank: BlockTxRankEstimate,
}

#[derive(Debug, Clone, Serialize)]
pub struct GasRankRecommendation {
    pub label: &'static str,
    pub target_position: Option<u64>,
    pub sample_quantile: Option<f64>,
    pub priority_percentile: Option<f64>,
    pub raw_priority_fee_gwei: f64,
    pub priority_tie_breaker_gwei: f64,
    pub priority_fee_gwei: f64,
    pub max_fee_per_gas_gwei: f64,
    pub effective_priority_fee_gwei: f64,
    pub estimated_base_cost_eth: f64,
    pub estimated_base_cost_usd: f64,
    pub estimated_priority_spend_eth: f64,
    pub estimated_priority_spend_usd: f64,
    pub estimated_total_cost_eth: f64,
    pub estimated_total_cost_usd: f64,
    pub estimated_max_cost_eth: f64,
    pub estimated_max_cost_usd: f64,
    pub rank: BlockTxRankEstimate,
}

#[derive(Debug, Clone, Serialize)]
pub struct GasRankHistoryRow {
    pub source_id: Option<String>,
    pub block_number: u64,
    pub block_hash: Option<String>,
    pub tx_count: usize,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub gas_used_ratio: f64,
    pub base_fee_gwei: f64,
    pub processed_at_ms: Option<u64>,
    pub priority_p50_gwei: f64,
    pub priority_p75_gwei: f64,
    pub priority_p90_gwei: f64,
    pub priority_p95_gwei: f64,
    pub normal_priority_gwei: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedBlock {
    pub block_number: u64,
    pub error: String,
}

#[derive(Debug, Clone)]
struct RecentBlockEvent {
    source_id: Option<String>,
    block_number: u64,
    block_hash: Option<String>,
    tx_count: Option<u64>,
    processed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
struct RecommendationProfile {
    label: &'static str,
    priority: RecommendationPriority,
}

#[derive(Debug, Clone, Copy)]
enum RecommendationPriority {
    /// Sample the fee needed to beat a transaction position in each block, then
    /// take a quantile across sampled blocks.
    TargetPosition {
        target_position: u64,
        sample_quantile: f64,
    },
    /// Pool mined transaction priority fees across the sampled window and take
    /// a fee percentile. This is what P50/P75/P90/P95 labels mean.
    PriorityPercentile { percentile: f64 },
}

#[derive(Debug, Clone)]
struct EthUsdPrice {
    price: f64,
    source: String,
}

const RECOMMENDATION_PROFILES: [RecommendationProfile; 20] = [
    RecommendationProfile {
        label: "Normal",
        priority: RecommendationPriority::TargetPosition {
            target_position: 50,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "Rank25",
        priority: RecommendationPriority::TargetPosition {
            target_position: 25,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "Rank10",
        priority: RecommendationPriority::TargetPosition {
            target_position: 10,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "Rank5",
        priority: RecommendationPriority::TargetPosition {
            target_position: 5,
            sample_quantile: 0.50,
        },
    },
    RecommendationProfile {
        label: "P50",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.50 },
    },
    RecommendationProfile {
        label: "P55",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.55 },
    },
    RecommendationProfile {
        label: "P60",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.60 },
    },
    RecommendationProfile {
        label: "P65",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.65 },
    },
    RecommendationProfile {
        label: "P70",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.70 },
    },
    RecommendationProfile {
        label: "P75",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.75 },
    },
    RecommendationProfile {
        label: "P77",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.77 },
    },
    RecommendationProfile {
        label: "P85",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.85 },
    },
    RecommendationProfile {
        label: "P88",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.88 },
    },
    RecommendationProfile {
        label: "P90",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.90 },
    },
    RecommendationProfile {
        label: "P92",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.92 },
    },
    RecommendationProfile {
        label: "P94",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.94 },
    },
    RecommendationProfile {
        label: "P95",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.95 },
    },
    RecommendationProfile {
        label: "P96",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.96 },
    },
    RecommendationProfile {
        label: "P97",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.97 },
    },
    RecommendationProfile {
        label: "P99",
        priority: RecommendationPriority::PriorityPercentile { percentile: 0.99 },
    },
];

pub async fn estimate(
    state: &ServerState,
    request: GasRankEstimateRequest,
) -> Result<GasRankEstimateResponse> {
    let lookback_blocks = request
        .lookback_blocks
        .unwrap_or(DEFAULT_LOOKBACK_BLOCKS)
        .clamp(1, MAX_LOOKBACK_BLOCKS);
    let gas_limit = request.gas_limit.unwrap_or(DEFAULT_GAS_LIMIT).max(1);
    let estimated_gas_used = request.estimated_gas_used.unwrap_or(gas_limit).max(1);
    let priority_fee = gwei_to_wei(
        request
            .max_priority_fee_per_gas_gwei
            .unwrap_or(DEFAULT_PRIORITY_FEE_GWEI),
    )?;

    let sampled = state
        .recent_live_fee_samples
        .latest(lookback_blocks as usize)
        .into_iter()
        .map(|entry| (recent_fee_sample_event(&entry), entry.sample))
        .collect::<Vec<_>>();
    if sampled.is_empty() {
        return Err(eyre!(
            "no recent live fee samples built from processed block transactions"
        ));
    }

    let available_recent_blocks = sampled.len();
    let inferred_blocks = 0usize;
    let skipped_blocks = Vec::new();
    let samples = sampled
        .iter()
        .map(|(_, sample)| sample.clone())
        .collect::<Vec<_>>();
    let latest_sample = samples
        .iter()
        .max_by_key(|sample| sample.block_number)
        .expect("non-empty samples checked above");
    let predicted_base_fee = predict_next_base_fee_from_sample(latest_sample);
    let max_fee = match request.max_fee_per_gas_gwei {
        Some(max_fee_gwei) => gwei_to_wei(max_fee_gwei)?,
        None => suggested_max_fee(predicted_base_fee, priority_fee),
    };
    let candidate = CandidateTxGas {
        gas_limit,
        max_fee_per_gas: max_fee,
        max_priority_fee_per_gas: priority_fee,
    };
    let priority_tie_breaker = priority_tie_breaker_wei();
    let candidate_rank = estimate_from_samples(candidate, predicted_base_fee, &samples)?;
    let eth_usd = resolve_eth_usd_price()?;
    let candidate_effective_priority_fee = effective_priority_fee(candidate, predicted_base_fee);
    let candidate_base_cost_eth = gas_spend_eth(predicted_base_fee, estimated_gas_used);
    let candidate_priority_spend_eth =
        gas_spend_eth(candidate_effective_priority_fee, estimated_gas_used);
    let candidate_total_cost_eth = candidate_base_cost_eth + candidate_priority_spend_eth;
    let candidate_max_cost_eth = max_cost_eth(max_fee, gas_limit);

    let recommendations = RECOMMENDATION_PROFILES
        .iter()
        .map(|profile| {
            recommendation(
                *profile,
                gas_limit,
                estimated_gas_used,
                predicted_base_fee,
                priority_tie_breaker,
                eth_usd.price,
                &samples,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let history = sampled
        .iter()
        .map(|(event, sample)| history_row(event, sample))
        .collect::<Vec<_>>();

    Ok(GasRankEstimateResponse {
        eth_usd_price: eth_usd.price,
        eth_usd_price_source: eth_usd.source.clone(),
        priority_tie_breaker_gwei: wei_to_gwei(priority_tie_breaker),
        blocks: RecentBlockWindow {
            source: "eth_chain_server_recent_live_fee_samples_from_processed_transactions",
            requested_blocks: lookback_blocks,
            available_recent_blocks,
            inferred_blocks,
            sampled_blocks: samples.len(),
            sampled_transactions: samples
                .iter()
                .map(|sample| sample.transactions.len())
                .sum::<usize>(),
            latest_block: sampled.first().map(|(event, _)| event.block_number),
            latest_block_hash: sampled
                .first()
                .and_then(|(event, _)| event.block_hash.clone()),
            latest_processed_at_ms: sampled.first().and_then(|(event, _)| event.processed_at_ms),
            block_numbers: sampled
                .iter()
                .map(|(event, _)| event.block_number)
                .collect(),
        },
        candidate: CandidateGasRankView {
            gas_limit,
            estimated_gas_used,
            max_fee_per_gas_gwei: wei_to_gwei(max_fee),
            max_priority_fee_per_gas_gwei: wei_to_gwei(priority_fee),
            predicted_base_fee_gwei: wei_to_gwei(predicted_base_fee),
            effective_priority_fee_gwei: wei_to_gwei(candidate_effective_priority_fee),
            estimated_base_cost_eth: candidate_base_cost_eth,
            estimated_base_cost_usd: eth_to_usd(candidate_base_cost_eth, eth_usd.price),
            estimated_priority_spend_eth: candidate_priority_spend_eth,
            estimated_priority_spend_usd: eth_to_usd(candidate_priority_spend_eth, eth_usd.price),
            estimated_total_cost_eth: candidate_total_cost_eth,
            estimated_total_cost_usd: eth_to_usd(candidate_total_cost_eth, eth_usd.price),
            estimated_max_cost_eth: candidate_max_cost_eth,
            estimated_max_cost_usd: eth_to_usd(candidate_max_cost_eth, eth_usd.price),
            rank: candidate_rank,
        },
        recommendations,
        history,
        skipped_blocks,
    })
}

pub fn recent_samples(state: &ServerState, limit: usize) -> GasRankSamplesResponse {
    let requested_blocks = limit.clamp(1, MAX_LOOKBACK_BLOCKS as usize);
    let samples = state.recent_live_fee_samples.latest(requested_blocks);
    GasRankSamplesResponse {
        source: "eth_chain_server_recent_live_fee_samples_from_processed_transactions",
        requested_blocks,
        available_recent_blocks: samples.len(),
        latest_block: samples.first().map(|entry| entry.sample.block_number),
        latest_block_hash: samples
            .first()
            .map(|entry| entry.sample.block_hash.to_string()),
        samples,
    }
}

fn recent_fee_sample_event(entry: &RecentLiveFeeSample) -> RecentBlockEvent {
    RecentBlockEvent {
        source_id: Some(format!(
            "recent_live_fee_samples:{}",
            entry.sample.block_number
        )),
        block_number: entry.sample.block_number,
        block_hash: Some(entry.sample.block_hash.to_string()),
        tx_count: Some(entry.sample.transactions.len() as u64),
        processed_at_ms: Some(entry.applied_at_unix_ms),
    }
}

fn recommendation(
    profile: RecommendationProfile,
    gas_limit: u64,
    estimated_gas_used: u64,
    predicted_base_fee: U256,
    priority_tie_breaker: U256,
    eth_usd_price: f64,
    samples: &[MinedBlockFeeSample],
) -> Result<GasRankRecommendation> {
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
    let candidate = CandidateTxGas {
        gas_limit,
        max_fee_per_gas: max_fee,
        max_priority_fee_per_gas: priority_fee,
    };
    let rank = estimate_from_samples(candidate, predicted_base_fee, samples)?;
    let effective_priority = effective_priority_fee(candidate, predicted_base_fee);
    let base_cost_eth = gas_spend_eth(predicted_base_fee, estimated_gas_used);
    let priority_spend_eth = gas_spend_eth(effective_priority, estimated_gas_used);
    let total_cost_eth = base_cost_eth + priority_spend_eth;
    let max_cost_eth_value = max_cost_eth(max_fee, gas_limit);
    Ok(GasRankRecommendation {
        label: profile.label,
        target_position: match profile.priority {
            RecommendationPriority::TargetPosition {
                target_position, ..
            } => Some(target_position),
            RecommendationPriority::PriorityPercentile { .. } => None,
        },
        sample_quantile: match profile.priority {
            RecommendationPriority::TargetPosition {
                sample_quantile, ..
            } => Some(sample_quantile),
            RecommendationPriority::PriorityPercentile { .. } => None,
        },
        priority_percentile: match profile.priority {
            RecommendationPriority::TargetPosition { .. } => None,
            RecommendationPriority::PriorityPercentile { percentile } => Some(percentile),
        },
        raw_priority_fee_gwei: wei_to_gwei(raw_priority_fee),
        priority_tie_breaker_gwei: wei_to_gwei(priority_tie_breaker),
        priority_fee_gwei: wei_to_gwei(priority_fee),
        max_fee_per_gas_gwei: wei_to_gwei(max_fee),
        effective_priority_fee_gwei: wei_to_gwei(effective_priority),
        estimated_base_cost_eth: base_cost_eth,
        estimated_base_cost_usd: eth_to_usd(base_cost_eth, eth_usd_price),
        estimated_priority_spend_eth: priority_spend_eth,
        estimated_priority_spend_usd: eth_to_usd(priority_spend_eth, eth_usd_price),
        estimated_total_cost_eth: total_cost_eth,
        estimated_total_cost_usd: eth_to_usd(total_cost_eth, eth_usd_price),
        estimated_max_cost_eth: max_cost_eth_value,
        estimated_max_cost_usd: eth_to_usd(max_cost_eth_value, eth_usd_price),
        rank,
    })
}

fn history_row(event: &RecentBlockEvent, sample: &MinedBlockFeeSample) -> GasRankHistoryRow {
    let priorities = sample_priorities_ascending(sample);
    GasRankHistoryRow {
        source_id: event.source_id.clone(),
        block_number: sample.block_number,
        block_hash: event
            .block_hash
            .clone()
            .or_else(|| Some(sample.block_hash.to_string())),
        tx_count: event
            .tx_count
            .map(|tx_count| tx_count as usize)
            .unwrap_or(sample.transactions.len()),
        gas_used: sample.gas_used,
        gas_limit: sample.gas_limit,
        gas_used_ratio: if sample.gas_limit == 0 {
            0.0
        } else {
            sample.gas_used as f64 / sample.gas_limit as f64
        },
        base_fee_gwei: wei_to_gwei(sample.base_fee_per_gas),
        processed_at_ms: event.processed_at_ms,
        priority_p50_gwei: wei_to_gwei(quantile_u256(priorities.clone(), 0.50)),
        priority_p75_gwei: wei_to_gwei(quantile_u256(priorities.clone(), 0.75)),
        priority_p90_gwei: wei_to_gwei(quantile_u256(priorities.clone(), 0.90)),
        priority_p95_gwei: wei_to_gwei(quantile_u256(priorities, 0.95)),
        normal_priority_gwei: wei_to_gwei(priority_for_target_position(sample, 50)),
    }
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

fn sample_priorities_ascending(sample: &MinedBlockFeeSample) -> Vec<U256> {
    let mut priorities = sample
        .transactions
        .iter()
        .map(|tx| tx.effective_priority_fee)
        .collect::<Vec<_>>();
    priorities.sort_unstable();
    priorities
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

    let parent_base_fee = parent_base_fee;
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

fn gwei_to_wei(value: f64) -> Result<U256> {
    if !value.is_finite() || value < 0.0 {
        return Err(eyre!("gwei value must be a finite non-negative number"));
    }
    Ok(U256::from((value * WEI_PER_GWEI).round() as u128))
}

fn priority_tie_breaker_wei() -> U256 {
    shared_config_value(PRIORITY_TIE_BREAKER_GWEI_CONFIG)
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .and_then(|value| gwei_to_wei(value).ok())
        .unwrap_or_else(|| {
            gwei_to_wei(DEFAULT_PRIORITY_TIE_BREAKER_GWEI)
                .expect("default priority tie-breaker is finite")
        })
}

fn wei_to_gwei(value: U256) -> f64 {
    u256_to_f64(value) / WEI_PER_GWEI
}

fn resolve_eth_usd_price() -> Result<EthUsdPrice> {
    let Some(price) = shared_config_value(ETH_USD_PRICE_CONFIG)?
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
    else {
        return Ok(EthUsdPrice {
            price: 0.0,
            source: ETH_USD_UNCONFIGURED_SOURCE.to_string(),
        });
    };

    Ok(EthUsdPrice {
        price,
        source: format!("config:{ETH_USD_PRICE_CONFIG}"),
    })
}

fn gas_spend_eth(fee_per_gas: U256, estimated_gas_used: u64) -> f64 {
    u256_to_f64(fee_per_gas) * estimated_gas_used as f64 / WEI_PER_ETH
}

fn max_cost_eth(max_fee: U256, gas_limit: u64) -> f64 {
    u256_to_f64(max_fee) * gas_limit as f64 / WEI_PER_ETH
}

fn eth_to_usd(eth_amount: f64, eth_usd_price: f64) -> f64 {
    eth_amount * eth_usd_price
}

fn u256_to_f64(value: U256) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(f64::INFINITY)
}
