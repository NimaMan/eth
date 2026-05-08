use eth_token::manager::{BlockTokenProcessor, TokenBlockUpdateReport};

use crate::range_indexer::progress::{now_unix_secs, RangeIndexStatus};
use crate::range_indexer::{RangeIndexError, RangeIndexJob, RangeIndexState};

use super::cache::ProcessedBlockDiskCacheMetrics;

pub(super) async fn take_processor_for_apply(
    run: &RangeIndexJob,
    block_number: u64,
) -> BlockTokenProcessor {
    let mut state = run.state.write().await;
    state.progress.current_block = Some(block_number);
    state.progress.updated_at_unix_secs = now_unix_secs();
    state.processor.clone()
}

pub(super) async fn restore_processor_after_apply(
    run: &RangeIndexJob,
    processor: BlockTokenProcessor,
) {
    let mut state = run.state.write().await;
    state.processor = processor;
    state.progress.updated_at_unix_secs = now_unix_secs();
}

pub(super) async fn mark_running(run: &RangeIndexJob) {
    let mut state = run.state.write().await;
    state.progress.status = RangeIndexStatus::Running;
    state.progress.updated_at_unix_secs = now_unix_secs();
}

pub(super) async fn mark_completed(run: &RangeIndexJob) {
    let mut state = run.state.write().await;
    state.progress.status = RangeIndexStatus::Completed;
    state.progress.completed_at_unix_secs = Some(now_unix_secs());
    state.progress.updated_at_unix_secs = now_unix_secs();
    tracing::info!(run_id = %run.id, "completed token tracking run");
}

pub(super) async fn mark_stopped(run: &RangeIndexJob) {
    let mut state = run.state.write().await;
    state.progress.status = RangeIndexStatus::Stopped;
    state.progress.completed_at_unix_secs = Some(now_unix_secs());
    state.progress.updated_at_unix_secs = now_unix_secs();
    tracing::info!(run_id = %run.id, "stopped token tracking run");
}

pub(super) async fn mark_failed(run: &RangeIndexJob, error: RangeIndexError) {
    let mut state = run.state.write().await;
    state.progress.status = RangeIndexStatus::Failed;
    state.progress.last_error = Some(error.message.clone());
    state.progress.completed_at_unix_secs = Some(now_unix_secs());
    state.progress.updated_at_unix_secs = now_unix_secs();
    state.errors.push(error);
    tracing::warn!(run_id = %run.id, error = ?state.progress.last_error, "failed token tracking run");
}

pub(super) fn apply_report(
    run_id: &str,
    state: &mut RangeIndexState,
    report: TokenBlockUpdateReport,
    upstream_ms: u128,
    token_apply_ms: u128,
    disk_cache_metrics: &ProcessedBlockDiskCacheMetrics,
) {
    let simulation_summary = simulation_summary(state, &report);
    tracing::info!(
        run_id = %run_id,
        block_number = report.block_number,
        simulations_attempted = simulation_summary.attempted,
        simulations_succeeded = simulation_summary.succeeded,
        cannot_sell_count = simulation_summary.cannot_sell,
        simulator_errors_count = simulation_summary.errors,
        token_apply_ms,
        "range block simulation summary"
    );

    state.progress.current_block = Some(report.block_number);
    state.progress.blocks_processed += 1;
    state.progress.txs_scanned += report.transaction_count;
    state.progress.txs_processed += report.processed_transaction_count;
    state.progress.tx_failures += report.failed_transaction_count;
    state.progress.token_update_reports += report.token_updates.len();
    state.progress.last_block_upstream_ms = Some(upstream_ms);
    state.progress.last_block_token_apply_ms = Some(token_apply_ms);
    if disk_cache_metrics.disk_cache_hit {
        state.progress.processed_block_disk_cache_hits += 1;
    } else {
        state.progress.processed_block_disk_cache_misses += 1;
    }
    state.progress.last_block_disk_cache_read_ms = Some(disk_cache_metrics.disk_cache_read_ms);
    state.progress.last_block_disk_cache_write_ms = Some(disk_cache_metrics.disk_cache_write_ms);
    state.progress.last_block_source = Some(disk_cache_metrics.source.to_string());
    state.progress.updated_at_unix_secs = now_unix_secs();

    state.created_tokens.extend(report.created_token_addresses);
    state.updated_tokens.extend(report.updated_token_addresses);

    for update in report.token_updates {
        state
            .discovered_v2_pools
            .extend(update.discovered_uniswap_v2_pools);
        state
            .updated_v2_pools
            .extend(update.updated_uniswap_v2_pools);
        state
            .discovered_v3_pools
            .extend(update.discovered_uniswap_v3_pools);
        state
            .updated_v3_pools
            .extend(update.updated_uniswap_v3_pools);
        state
            .discovered_v4_pools
            .extend(update.discovered_uniswap_v4_pools);
        state
            .updated_v4_pools
            .extend(update.updated_uniswap_v4_pools);
    }

    for error in report.transaction_errors {
        state.errors.push(RangeIndexError {
            block_number: Some(report.block_number),
            tx_index: Some(error.tx_index),
            tx_hash: Some(error.tx_hash),
            message: error.message,
        });
    }

    state.progress.created_tokens_unique = state.created_tokens.len();
    state.progress.updated_tokens_unique = state.updated_tokens.len();
    state.progress.discovered_v2_pools_unique = state.discovered_v2_pools.len();
    state.progress.updated_v2_pools_unique = state.updated_v2_pools.len();
    state.progress.discovered_v3_pools_unique = state.discovered_v3_pools.len();
    state.progress.updated_v3_pools_unique = state.updated_v3_pools.len();
    state.progress.discovered_v4_pools_unique = state.discovered_v4_pools.len();
    state.progress.updated_v4_pools_unique = state.updated_v4_pools.len();
    state.progress.tracked_tokens = state.processor.registry.tokens.len();
    state.progress.indexed_tokens = state.processor.token_index.entries.len();
    state.progress.indexed_pools = state.processor.token_index.pool_to_token.len();
    state.progress.tracked_pools = state
        .processor
        .registry
        .tokens
        .values()
        .map(|token| token.pool_count())
        .sum();
    state.progress.indexed_v2_pools = state
        .processor
        .registry
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum();
    state.progress.tracked_v2_pools = state
        .processor
        .registry
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum();
    state.progress.indexed_v3_pools = state
        .processor
        .registry
        .tokens
        .values()
        .map(|token| token.v3_pools.len())
        .sum();
    state.progress.tracked_v3_pools = state.progress.indexed_v3_pools;
    state.progress.indexed_v4_pools = state
        .processor
        .registry
        .tokens
        .values()
        .map(|token| token.v4_pools.len())
        .sum();
    state.progress.tracked_v4_pools = state.progress.indexed_v4_pools;
}

#[derive(Clone, Copy, Debug, Default)]
struct SimulationSummary {
    attempted: usize,
    succeeded: usize,
    cannot_sell: usize,
    errors: usize,
}

fn simulation_summary(
    state: &RangeIndexState,
    report: &TokenBlockUpdateReport,
) -> SimulationSummary {
    let mut summary = SimulationSummary {
        errors: report
            .transaction_errors
            .iter()
            .filter(|error| looks_like_simulator_error(&error.message))
            .count(),
        ..SimulationSummary::default()
    };

    for update in &report.token_updates {
        let simulated_pool_count = update.simulated_uniswap_v2_pools.len()
            + update.simulated_uniswap_v3_pools.len()
            + update.simulated_uniswap_v4_pools.len();
        summary.attempted += simulated_pool_count;
        summary.succeeded += simulated_pool_count;

        let Some(token) = state.processor.registry.token(&update.token_address) else {
            continue;
        };
        for pool_address in update
            .simulated_uniswap_v2_pools
            .iter()
            .chain(update.simulated_uniswap_v3_pools.iter())
            .chain(update.simulated_uniswap_v4_pools.iter())
        {
            let Some(pool) = token.pool_base(pool_address) else {
                continue;
            };
            if pool.state.can_buy && !pool.state.can_sell {
                summary.cannot_sell += 1;
            }
        }
    }

    summary
}

fn looks_like_simulator_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("simulat")
        || message.contains("transfer_failed")
        || message.contains("revert")
        || message.contains("cannot sell")
        || message.contains("cannot buy")
}
