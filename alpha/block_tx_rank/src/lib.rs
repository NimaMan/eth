//! Rough transaction rank estimates from recent mined Ethereum blocks.
//!
//! This crate is intentionally alpha-owned. Strategies and future real
//! execution adapters can use it before handing a final prepared transaction to
//! `tx_executor`, keeping the executor focused on signing and broadcast.

use alloy_primitives::{B256, U256};
use eyre::{eyre, Result};
use reth_chain_query::provider::{
    BlockHeader, RethQueryProvider, TransactionMetadata, TransactionReceipt,
};
use serde::{Deserialize, Serialize};

const EIP1559_ELASTICITY_MULTIPLIER: u64 = 2;
const EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;

/// Gas parameters for the transaction we are considering submitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateTxGas {
    pub gas_limit: u64,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

/// Runtime knobs for mined-block rank estimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockTxRankConfig {
    pub lookback_blocks: u64,
}

impl Default for BlockTxRankConfig {
    fn default() -> Self {
        Self {
            lookback_blocks: 20,
        }
    }
}

/// One mined transaction's fee evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinedTxFeeSample {
    pub tx_hash: B256,
    pub tx_index: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub effective_gas_price: U256,
    pub effective_priority_fee: U256,
}

impl MinedTxFeeSample {
    pub fn from_metadata_and_receipt(
        base_fee_per_gas: U256,
        metadata: &TransactionMetadata,
        receipt: &TransactionReceipt,
    ) -> Self {
        Self {
            tx_hash: metadata.hash,
            tx_index: metadata.tx_index,
            gas_limit: metadata.gas_limit,
            gas_used: receipt.gas_used,
            effective_gas_price: receipt.effective_gas_price,
            effective_priority_fee: saturating_sub_u256(
                receipt.effective_gas_price,
                base_fee_per_gas,
            ),
        }
    }
}

/// Fee samples for one mined block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinedBlockFeeSample {
    pub block_number: u64,
    pub block_hash: B256,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub base_fee_per_gas: U256,
    pub transactions: Vec<MinedTxFeeSample>,
}

impl MinedBlockFeeSample {
    pub fn from_provider_rows(
        header: BlockHeader,
        rows: Vec<(TransactionMetadata, TransactionReceipt)>,
    ) -> Self {
        let base_fee_per_gas = U256::from(header.base_fee_per_gas.unwrap_or_default());
        let transactions = rows
            .iter()
            .map(|(metadata, receipt)| {
                MinedTxFeeSample::from_metadata_and_receipt(base_fee_per_gas, metadata, receipt)
            })
            .collect();

        Self {
            block_number: header.number,
            block_hash: header.hash,
            gas_limit: header.gas_limit,
            gas_used: header.gas_used,
            base_fee_per_gas,
            transactions,
        }
    }
}

/// Confidence label for a rank estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockTxRankSource {
    RecentMinedBlocks,
}

/// Rough position bands for a candidate transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockTxRankEstimate {
    pub latest_block: u64,
    pub analyzed_blocks: usize,
    pub analyzed_transactions: usize,
    pub predicted_base_fee_per_gas: U256,
    pub candidate_effective_priority_fee: U256,
    pub fee_cap_covers_predicted_base_fee: bool,
    /// Median 1-based position if mined transactions were sorted by priority fee.
    pub position_p50: u64,
    /// 75th percentile 1-based position; higher is worse.
    pub position_p75: u64,
    /// 90th percentile 1-based position; higher is worse.
    pub position_p90: u64,
    pub transactions_ahead_p50: u64,
    pub transactions_ahead_p75: u64,
    pub transactions_ahead_p90: u64,
    pub gas_before_p50: u64,
    pub gas_before_p75: u64,
    pub gas_before_p90: u64,
    /// Median percent of mined transactions priced below the candidate.
    pub fee_percentile_p50: f64,
    pub fee_percentile_p25: f64,
    pub likely_fits_at_p50: bool,
    pub likely_fits_at_p90: bool,
    pub confidence: EstimateConfidence,
    pub source: BlockTxRankSource,
}

/// Provider-backed estimator over recent mined blocks.
pub struct MinedBlockRankEstimator<'a> {
    provider: &'a RethQueryProvider,
}

impl<'a> MinedBlockRankEstimator<'a> {
    pub fn new(provider: &'a RethQueryProvider) -> Self {
        Self { provider }
    }

    pub async fn estimate_latest(
        &self,
        candidate: CandidateTxGas,
        config: BlockTxRankConfig,
    ) -> Result<BlockTxRankEstimate> {
        let latest_block = self.provider.get_latest_block()?;
        self.estimate_at_head(latest_block, candidate, config).await
    }

    pub async fn estimate_at_head(
        &self,
        head_block: u64,
        candidate: CandidateTxGas,
        config: BlockTxRankConfig,
    ) -> Result<BlockTxRankEstimate> {
        let head = self.provider.fetch_block_header_only(head_block).await?;
        let predicted_base_fee_per_gas = predict_next_base_fee(&head);
        let samples = self.load_recent_samples(head_block, config).await?;
        estimate_from_samples(candidate, predicted_base_fee_per_gas, &samples)
    }

    pub async fn load_recent_samples(
        &self,
        head_block: u64,
        config: BlockTxRankConfig,
    ) -> Result<Vec<MinedBlockFeeSample>> {
        if config.lookback_blocks == 0 {
            return Err(eyre!("lookback_blocks must be greater than zero"));
        }

        let first_block = head_block
            .saturating_add(1)
            .saturating_sub(config.lookback_blocks);
        let mut samples = Vec::new();

        for block_number in first_block..=head_block {
            let header = self.provider.fetch_block_header_only(block_number).await?;
            let rows = self
                .provider
                .fetch_block_tx_metadata_and_receipts(block_number)
                .await?;
            samples.push(MinedBlockFeeSample::from_provider_rows(header, rows));
        }

        Ok(samples)
    }
}

/// Estimate rank using caller-provided mined-block samples.
pub fn estimate_from_samples(
    candidate: CandidateTxGas,
    predicted_base_fee_per_gas: U256,
    samples: &[MinedBlockFeeSample],
) -> Result<BlockTxRankEstimate> {
    if candidate.gas_limit == 0 {
        return Err(eyre!("candidate gas_limit must be greater than zero"));
    }
    if samples.is_empty() {
        return Err(eyre!("at least one mined block sample is required"));
    }

    let candidate_priority_fee = effective_priority_fee(candidate, predicted_base_fee_per_gas);
    let fee_cap_covers_predicted_base_fee = candidate.max_fee_per_gas >= predicted_base_fee_per_gas;
    let latest_block = samples
        .iter()
        .map(|sample| sample.block_number)
        .max()
        .unwrap_or_default();
    let analyzed_transactions = samples
        .iter()
        .map(|sample| sample.transactions.len())
        .sum::<usize>();

    let mut transactions_ahead = Vec::new();
    let mut gas_before = Vec::new();
    let mut fee_percentiles = Vec::new();
    let mut block_gas_limits = Vec::new();

    for sample in samples {
        let mut ahead_count = 0u64;
        let mut ahead_gas = 0u64;

        for tx in &sample.transactions {
            if tx.effective_priority_fee > candidate_priority_fee {
                ahead_count = ahead_count.saturating_add(1);
                ahead_gas = ahead_gas.saturating_add(tx.gas_used);
            }
        }

        transactions_ahead.push(ahead_count);
        gas_before.push(ahead_gas);
        block_gas_limits.push(sample.gas_limit);

        if sample.transactions.is_empty() {
            fee_percentiles.push(100.0);
        } else {
            let priced_below = sample
                .transactions
                .iter()
                .filter(|tx| tx.effective_priority_fee < candidate_priority_fee)
                .count();
            fee_percentiles.push((priced_below as f64 / sample.transactions.len() as f64) * 100.0);
        }
    }

    let tx_ahead_p50 = quantile_u64(transactions_ahead.clone(), 0.50);
    let tx_ahead_p75 = quantile_u64(transactions_ahead.clone(), 0.75);
    let tx_ahead_p90 = quantile_u64(transactions_ahead, 0.90);
    let gas_before_p50 = quantile_u64(gas_before.clone(), 0.50);
    let gas_before_p75 = quantile_u64(gas_before.clone(), 0.75);
    let gas_before_p90 = quantile_u64(gas_before, 0.90);
    let fit_gas_limit = quantile_u64(block_gas_limits, 0.50);

    Ok(BlockTxRankEstimate {
        latest_block,
        analyzed_blocks: samples.len(),
        analyzed_transactions,
        predicted_base_fee_per_gas,
        candidate_effective_priority_fee: candidate_priority_fee,
        fee_cap_covers_predicted_base_fee,
        position_p50: tx_ahead_p50.saturating_add(1),
        position_p75: tx_ahead_p75.saturating_add(1),
        position_p90: tx_ahead_p90.saturating_add(1),
        transactions_ahead_p50: tx_ahead_p50,
        transactions_ahead_p75: tx_ahead_p75,
        transactions_ahead_p90: tx_ahead_p90,
        gas_before_p50,
        gas_before_p75,
        gas_before_p90,
        fee_percentile_p50: quantile_f64(fee_percentiles.clone(), 0.50),
        fee_percentile_p25: quantile_f64(fee_percentiles, 0.25),
        likely_fits_at_p50: fee_cap_covers_predicted_base_fee
            && candidate.gas_limit.saturating_add(gas_before_p50) <= fit_gas_limit,
        likely_fits_at_p90: fee_cap_covers_predicted_base_fee
            && candidate.gas_limit.saturating_add(gas_before_p90) <= fit_gas_limit,
        confidence: estimate_confidence(samples.len(), analyzed_transactions),
        source: BlockTxRankSource::RecentMinedBlocks,
    })
}

pub fn effective_priority_fee(candidate: CandidateTxGas, base_fee_per_gas: U256) -> U256 {
    if candidate.max_fee_per_gas <= base_fee_per_gas {
        return U256::ZERO;
    }

    let max_possible_priority_fee = candidate.max_fee_per_gas - base_fee_per_gas;
    candidate
        .max_priority_fee_per_gas
        .min(max_possible_priority_fee)
}

pub fn predict_next_base_fee(parent: &BlockHeader) -> U256 {
    let Some(parent_base_fee) = parent.base_fee_per_gas else {
        return U256::ZERO;
    };

    let parent_gas_target = parent.gas_limit / EIP1559_ELASTICITY_MULTIPLIER;
    if parent_gas_target == 0 || parent.gas_used == parent_gas_target {
        return U256::from(parent_base_fee);
    }

    let parent_base_fee = parent_base_fee as u128;
    let parent_gas_target = parent_gas_target as u128;
    let parent_gas_used = parent.gas_used as u128;

    if parent_gas_used > parent_gas_target {
        let gas_delta = parent_gas_used - parent_gas_target;
        let base_delta = (parent_base_fee * gas_delta
            / parent_gas_target
            / EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR as u128)
            .max(1);
        U256::from(parent_base_fee.saturating_add(base_delta))
    } else {
        let gas_delta = parent_gas_target - parent_gas_used;
        let base_delta = parent_base_fee * gas_delta
            / parent_gas_target
            / EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR as u128;
        U256::from(parent_base_fee.saturating_sub(base_delta))
    }
}

fn saturating_sub_u256(left: U256, right: U256) -> U256 {
    if left >= right {
        left - right
    } else {
        U256::ZERO
    }
}

fn quantile_u64(mut values: Vec<u64>, quantile: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }

    values.sort_unstable();
    let index = quantile_index(values.len(), quantile);
    values[index]
}

fn quantile_f64(mut values: Vec<f64>, quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.sort_by(|left, right| left.total_cmp(right));
    let index = quantile_index(values.len(), quantile);
    values[index]
}

fn quantile_index(len: usize, quantile: f64) -> usize {
    let bounded = quantile.clamp(0.0, 1.0);
    ((len.saturating_sub(1) as f64) * bounded).round() as usize
}

fn estimate_confidence(block_count: usize, tx_count: usize) -> EstimateConfidence {
    match (block_count, tx_count) {
        (blocks, txs) if blocks >= 12 && txs >= 1_000 => EstimateConfidence::High,
        (blocks, txs) if blocks >= 4 && txs >= 200 => EstimateConfidence::Medium,
        _ => EstimateConfidence::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::B256;

    fn sample(block_number: u64, priorities_gwei: &[u64], gas_used: u64) -> MinedBlockFeeSample {
        let base_fee_per_gas = U256::from(50_000_000_000u64);
        let transactions = priorities_gwei
            .iter()
            .enumerate()
            .map(|(index, priority_gwei)| {
                let priority = U256::from(priority_gwei * 1_000_000_000);
                MinedTxFeeSample {
                    tx_hash: B256::with_last_byte(index as u8),
                    tx_index: index as u64,
                    gas_limit: gas_used,
                    gas_used,
                    effective_gas_price: base_fee_per_gas + priority,
                    effective_priority_fee: priority,
                }
            })
            .collect();

        MinedBlockFeeSample {
            block_number,
            block_hash: B256::with_last_byte(block_number as u8),
            gas_limit: 30_000_000,
            gas_used: priorities_gwei.len() as u64 * gas_used,
            base_fee_per_gas,
            transactions,
        }
    }

    fn candidate(priority_gwei: u64) -> CandidateTxGas {
        CandidateTxGas {
            gas_limit: 100_000,
            max_fee_per_gas: U256::from(200_000_000_000u64),
            max_priority_fee_per_gas: U256::from(priority_gwei * 1_000_000_000),
        }
    }

    #[test]
    fn effective_priority_fee_respects_fee_cap() {
        let capped = effective_priority_fee(
            CandidateTxGas {
                gas_limit: 21_000,
                max_fee_per_gas: U256::from(55_000_000_000u64),
                max_priority_fee_per_gas: U256::from(10_000_000_000u64),
            },
            U256::from(50_000_000_000u64),
        );

        assert_eq!(capped, U256::from(5_000_000_000u64));
    }

    #[test]
    fn higher_tip_improves_estimated_position() {
        let samples = vec![
            sample(10, &[1, 2, 3, 4, 5], 21_000),
            sample(11, &[2, 3, 4, 5, 6], 21_000),
        ];

        let low =
            estimate_from_samples(candidate(2), U256::from(50_000_000_000u64), &samples).unwrap();
        let high =
            estimate_from_samples(candidate(5), U256::from(50_000_000_000u64), &samples).unwrap();

        assert!(high.position_p50 < low.position_p50);
        assert!(high.gas_before_p50 < low.gas_before_p50);
    }

    #[test]
    fn gas_before_band_uses_mined_gas_used() {
        let samples = vec![
            sample(20, &[10, 9, 1], 50_000),
            sample(21, &[10, 9, 8, 1], 50_000),
        ];

        let estimate =
            estimate_from_samples(candidate(5), U256::from(50_000_000_000u64), &samples).unwrap();

        assert_eq!(estimate.transactions_ahead_p50, 3);
        assert_eq!(estimate.gas_before_p50, 150_000);
        assert_eq!(estimate.position_p50, 4);
    }

    #[test]
    fn fee_cap_below_predicted_base_fee_is_not_fit() {
        let samples = vec![sample(30, &[1, 2, 3], 21_000)];
        let estimate = estimate_from_samples(
            CandidateTxGas {
                gas_limit: 21_000,
                max_fee_per_gas: U256::from(40_000_000_000u64),
                max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            },
            U256::from(50_000_000_000u64),
            &samples,
        )
        .unwrap();

        assert!(!estimate.fee_cap_covers_predicted_base_fee);
        assert!(!estimate.likely_fits_at_p50);
        assert_eq!(estimate.candidate_effective_priority_fee, U256::ZERO);
    }
}
