use eth_token::manager::{BlockTokenProcessor, TokenBlockUpdateReport};

use crate::range_indexer::progress::{now_unix_secs, RangeIndexStatus};
use crate::range_indexer::{RangeIndexError, RangeIndexJob, RangeIndexState};

use super::cache::ProcessedBlockCacheMetrics;

pub(super) async fn take_processor_for_apply(
    run: &RangeIndexJob,
    block_number: u64,
) -> BlockTokenProcessor {
    let mut state = run.state.write().await;
    state.progress.current_block = Some(block_number);
    state.progress.updated_at_unix_secs = now_unix_secs();
    std::mem::replace(
        &mut state.processor,
        BlockTokenProcessor::new(run.request.history_limit),
    )
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
    state: &mut RangeIndexState,
    report: TokenBlockUpdateReport,
    upstream_ms: u128,
    token_apply_ms: u128,
    cache_metrics: &ProcessedBlockCacheMetrics,
) {
    state.progress.current_block = Some(report.block_number);
    state.progress.blocks_processed += 1;
    state.progress.txs_scanned += report.transaction_count;
    state.progress.txs_processed += report.processed_transaction_count;
    state.progress.tx_failures += report.failed_transaction_count;
    state.progress.token_update_reports += report.token_updates.len();
    state.progress.last_block_upstream_ms = Some(upstream_ms);
    state.progress.last_block_token_apply_ms = Some(token_apply_ms);
    if cache_metrics.cache_hit {
        state.progress.processed_block_cache_hits += 1;
    } else {
        state.progress.processed_block_cache_misses += 1;
    }
    state.progress.last_block_cache_read_ms = Some(cache_metrics.cache_read_ms);
    state.progress.last_block_cache_write_ms = Some(cache_metrics.cache_write_ms);
    state.progress.last_block_source = Some(cache_metrics.source.to_string());
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
    state.progress.tracked_tokens = state.processor.registry.tokens.len();
    state.progress.indexed_tokens = state.processor.token_index.entries.len();
    state.progress.indexed_v2_pools = state.processor.token_index.pool_to_token.len();
    state.progress.tracked_v2_pools = state
        .processor
        .registry
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum();
}
