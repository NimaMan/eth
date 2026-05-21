use alloy_primitives::U256;
use eth_block_tx_rank::{
    effective_priority_fee, estimate_from_samples, BlockTxRankEstimate, CandidateTxGas,
    MinedBlockFeeSample,
};
use eyre::{eyre, Result};
use reth_chain_query::provider::RpcBlockDataFetcher;
use serde::{Deserialize, Serialize};

use crate::app::config::shared_config_value;
use crate::http::ServerState;
use crate::recent_blocks::RecentProcessedBlock;

const DEFAULT_LOOKBACK_BLOCKS: u64 = 100;
const MAX_LOOKBACK_BLOCKS: u64 = 100;
const DEFAULT_GAS_LIMIT: u64 = 300_000;
const DEFAULT_PRIORITY_FEE_GWEI: f64 = 2.0;
const TEMPORARY_ETH_USD_PRICE: f64 = 2_000.0;
const WEI_PER_GWEI: f64 = 1_000_000_000.0;
const WEI_PER_ETH: f64 = 1_000_000_000_000_000_000.0;
const EIP1559_ELASTICITY_MULTIPLIER: u64 = 2;
const EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;
const DEFAULT_EXECUTION_RPC_URL: &str = "http://127.0.0.1:8545";

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

#[derive(Debug, Clone, Serialize)]
pub struct GasRankEstimateResponse {
    pub eth_usd_price: f64,
    pub eth_usd_price_source: &'static str,
    pub blocks: RecentBlockWindow,
    pub candidate: CandidateGasRankView,
    pub recommendations: Vec<GasRankRecommendation>,
    pub history: Vec<GasRankHistoryRow>,
    pub skipped_blocks: Vec<SkippedBlock>,
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
    pub target_position: u64,
    pub sample_quantile: f64,
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
    pub minimum_priority_gwei: f64,
    pub balanced_priority_gwei: f64,
    pub aggressive_priority_gwei: f64,
    pub urgent_priority_gwei: f64,
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
    target_position: u64,
    sample_quantile: f64,
}

const RECOMMENDATION_PROFILES: [RecommendationProfile; 4] = [
    RecommendationProfile {
        label: "Minimum",
        target_position: 50,
        sample_quantile: 0.50,
    },
    RecommendationProfile {
        label: "Balanced",
        target_position: 25,
        sample_quantile: 0.75,
    },
    RecommendationProfile {
        label: "Aggressive",
        target_position: 10,
        sample_quantile: 0.90,
    },
    RecommendationProfile {
        label: "Urgent",
        target_position: 5,
        sample_quantile: 0.95,
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

    let (recent_blocks, available_recent_blocks) =
        load_recent_block_window(state, lookback_blocks as usize).await?;
    if recent_blocks.is_empty() {
        return Err(eyre!(
            "no recent processed blocks found in eth_chain_server"
        ));
    }
    let inferred_blocks = recent_blocks.len().saturating_sub(available_recent_blocks);

    let rpc_fetcher = match RpcBlockDataFetcher::new(&execution_rpc_url()) {
        Ok(fetcher) => Some(fetcher),
        Err(error) => {
            tracing::warn!(error = %error, "gas-rank execution RPC fallback unavailable");
            None
        }
    };
    if let Err(error) = state.provider.refresh_static_file_provider() {
        tracing::debug!(
            error = %error,
            "failed to refresh Reth static file provider before gas-rank window load"
        );
    }
    let mut sampled = Vec::new();
    let mut skipped_blocks = Vec::new();
    for event in &recent_blocks {
        match load_sample(state, event.block_number, rpc_fetcher.as_ref()).await {
            Ok(sample) => sampled.push((event.clone(), sample)),
            Err(error) => skipped_blocks.push(SkippedBlock {
                block_number: event.block_number,
                error: error.to_string(),
            }),
        }
    }
    if sampled.is_empty() {
        return Err(eyre!(
            "no recent eth_chain_server blocks could be loaded from Reth"
        ));
    }

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
    let candidate_rank = estimate_from_samples(candidate, predicted_base_fee, &samples)?;
    let eth_usd_price = resolve_eth_usd_price();
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
                eth_usd_price,
                &samples,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let history = sampled
        .iter()
        .map(|(event, sample)| history_row(event, sample))
        .collect::<Vec<_>>();

    Ok(GasRankEstimateResponse {
        eth_usd_price,
        eth_usd_price_source: "temporary_fixed_usd_2000_until_live_price_resolver",
        blocks: RecentBlockWindow {
            source: "eth_chain_server_recent_live_blocks",
            requested_blocks: lookback_blocks,
            available_recent_blocks,
            inferred_blocks,
            sampled_blocks: samples.len(),
            sampled_transactions: samples
                .iter()
                .map(|sample| sample.transactions.len())
                .sum::<usize>(),
            latest_block: recent_blocks.first().map(|event| event.block_number),
            latest_block_hash: recent_blocks
                .first()
                .and_then(|event| event.block_hash.clone()),
            latest_processed_at_ms: recent_blocks
                .first()
                .and_then(|event| event.processed_at_ms),
            block_numbers: recent_blocks
                .iter()
                .map(|event| event.block_number)
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
            estimated_base_cost_usd: eth_to_usd(candidate_base_cost_eth, eth_usd_price),
            estimated_priority_spend_eth: candidate_priority_spend_eth,
            estimated_priority_spend_usd: eth_to_usd(candidate_priority_spend_eth, eth_usd_price),
            estimated_total_cost_eth: candidate_total_cost_eth,
            estimated_total_cost_usd: eth_to_usd(candidate_total_cost_eth, eth_usd_price),
            estimated_max_cost_eth: candidate_max_cost_eth,
            estimated_max_cost_usd: eth_to_usd(candidate_max_cost_eth, eth_usd_price),
            rank: candidate_rank,
        },
        recommendations,
        history,
        skipped_blocks,
    })
}

async fn load_recent_block_window(
    state: &ServerState,
    count: usize,
) -> Result<(Vec<RecentBlockEvent>, usize)> {
    let recent = state.recent_live_blocks.latest(count);
    if !recent.is_empty() {
        let available = recent.len();
        return Ok((expand_recent_block_window(recent, count), available));
    }

    let progress = state.live_tracker.progress().await;
    let Some(latest_block) = progress.current_block else {
        return Ok((Vec::new(), 0));
    };
    let inferred = (0..count)
        .filter_map(|offset| latest_block.checked_sub(offset as u64))
        .map(|block_number| RecentBlockEvent {
            source_id: Some(format!("live_progress:{block_number}")),
            block_number,
            block_hash: None,
            tx_count: None,
            processed_at_ms: None,
        })
        .collect::<Vec<_>>();
    Ok((inferred, 0))
}

fn expand_recent_block_window(
    events: Vec<RecentProcessedBlock>,
    count: usize,
) -> Vec<RecentBlockEvent> {
    let Some(latest) = events.first().cloned() else {
        return Vec::new();
    };
    let mut by_block = events
        .into_iter()
        .map(|event| (event.block_number, event))
        .collect::<std::collections::HashMap<_, _>>();

    (0..count)
        .filter_map(|offset| {
            let block_number = latest.block_number.checked_sub(offset as u64)?;
            let event = by_block.remove(&block_number);
            Some(RecentBlockEvent {
                source_id: Some(format!("recent_live_blocks:{block_number}")),
                block_number,
                block_hash: event.as_ref().map(|event| event.block_hash.clone()),
                tx_count: None,
                processed_at_ms: event.map(|event| event.applied_at_unix_ms),
            })
        })
        .collect()
}

async fn load_sample(
    state: &ServerState,
    block_number: u64,
    rpc_fetcher: Option<&RpcBlockDataFetcher>,
) -> Result<MinedBlockFeeSample> {
    match load_sample_from_provider(state, block_number).await {
        Ok(sample) => Ok(sample),
        Err(provider_error) => {
            let Some(fetcher) = rpc_fetcher else {
                return Err(provider_error);
            };
            load_sample_from_rpc(fetcher, block_number)
                .await
                .map_err(|rpc_error| {
                    eyre!("Reth sample failed: {provider_error}; RPC fallback failed: {rpc_error}")
                })
        }
    }
}

async fn load_sample_from_provider(
    state: &ServerState,
    block_number: u64,
) -> Result<MinedBlockFeeSample> {
    let header = state.provider.fetch_block_header_only(block_number).await?;
    let rows = state
        .provider
        .fetch_block_tx_metadata_and_receipts(block_number)
        .await?;
    Ok(MinedBlockFeeSample::from_provider_rows(header, rows))
}

async fn load_sample_from_rpc(
    rpc_fetcher: &RpcBlockDataFetcher,
    block_number: u64,
) -> Result<MinedBlockFeeSample> {
    let raw = rpc_fetcher
        .fetch_raw_block_by_number_data(block_number, false)
        .await?;
    let rows = raw
        .transactions
        .into_iter()
        .zip(raw.receipts.into_iter())
        .collect::<Vec<_>>();
    Ok(MinedBlockFeeSample::from_provider_rows(raw.header, rows))
}

fn recommendation(
    profile: RecommendationProfile,
    gas_limit: u64,
    estimated_gas_used: u64,
    predicted_base_fee: U256,
    eth_usd_price: f64,
    samples: &[MinedBlockFeeSample],
) -> Result<GasRankRecommendation> {
    let priority_fee =
        recommended_priority(samples, profile.target_position, profile.sample_quantile);
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
        target_position: profile.target_position,
        sample_quantile: profile.sample_quantile,
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
        minimum_priority_gwei: wei_to_gwei(priority_for_target_position(sample, 50)),
        balanced_priority_gwei: wei_to_gwei(priority_for_target_position(sample, 25)),
        aggressive_priority_gwei: wei_to_gwei(priority_for_target_position(sample, 10)),
        urgent_priority_gwei: wei_to_gwei(priority_for_target_position(sample, 5)),
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

fn wei_to_gwei(value: U256) -> f64 {
    u256_to_f64(value) / WEI_PER_GWEI
}

fn resolve_eth_usd_price() -> f64 {
    TEMPORARY_ETH_USD_PRICE
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

fn execution_rpc_url() -> String {
    shared_config_value("RETH_HTTP_RPC")
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_EXECUTION_RPC_URL.to_string())
}
