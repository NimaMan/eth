use eth_ops_events::{emit_issue, PipelineBottleneckSample, PipelineIssue};
use eth_token::token_analytics::collect_current_observations;
use eth_token::tracking::{LiveTokenRetentionReport, TokenBlockUpdateReport};
use tx_processor::LoadedProcessedBlock as LiveBlockLoad;

use super::event::LiveTokenEvent;
use super::helpers::normalize_address;
use super::progress::LiveTokenError;
use super::snapshot::LiveTokenSnapshot;
use super::state::LiveTokenState;
use super::time::now_unix_secs;

const MAX_LIVE_ISSUES: usize = 1_000;
const MAX_LIVE_BOTTLENECKS: usize = 1_000;
const MAX_LIVE_ERRORS: usize = 1_000;

pub(super) fn apply_report(
    state: &mut LiveTokenState,
    report: TokenBlockUpdateReport,
    retention_report: Option<LiveTokenRetentionReport>,
    loaded: LiveBlockLoad,
    token_apply_ms: u128,
    is_live_tail: bool,
) -> LiveTokenEvent {
    let updated_tokens = report.updated_token_addresses.clone();
    let created_tokens = report.created_token_addresses.clone();
    let block_hash = report.block_hash.clone();
    let block_number = report.block_number;

    state.progress.current_block = Some(report.block_number);
    state.progress.current_block_hash = Some(block_hash.clone());
    state.progress.blocks_processed += 1;
    if is_live_tail {
        state.progress.live_blocks_processed += 1;
    }
    state.progress.txs_scanned += report.transaction_count;
    state.progress.txs_processed += report.processed_transaction_count;
    state.progress.transaction_failures += report.failed_transaction_count;
    state.progress.pool_simulation_failures += report.pool_simulation_failure_count;
    state.progress.token_update_reports += report.token_updates.len();
    state.progress.last_block_upstream_ms = Some(loaded.upstream_ms);
    state.progress.last_block_token_apply_ms = Some(token_apply_ms);
    state.progress.last_block_disk_cache_read_ms = Some(loaded.disk_cache_read_ms);
    state.progress.last_block_disk_cache_write_ms = Some(loaded.disk_cache_write_ms);
    state.progress.last_block_source = Some(loaded.source.to_string());
    state.progress.last_error = None;
    if loaded.disk_cache_hit {
        state.progress.processed_block_disk_cache_hits += 1;
    } else {
        state.progress.processed_block_disk_cache_misses += 1;
    }
    state.progress.updated_at_unix_secs = now_unix_secs();

    let observations = collect_current_observations(
        state.processor.registry(),
        &report,
        &mut state.active_observation_counts_by_pool,
    );
    state.observations.extend(observations);

    state.created_tokens.extend(created_tokens);
    state.updated_tokens.extend(updated_tokens.clone());

    let mut updated_v2_pools = Vec::new();
    let mut updated_v3_pools = Vec::new();
    let mut updated_v4_pools = Vec::new();
    for update in report.token_updates {
        state
            .discovered_v2_pools
            .extend(update.discovered_known_v2_pools);
        updated_v2_pools.extend(update.updated_known_v2_pools.clone());
        state.updated_v2_pools.extend(update.updated_known_v2_pools);
        state
            .discovered_v3_pools
            .extend(update.discovered_uniswap_v3_pools);
        updated_v3_pools.extend(update.updated_uniswap_v3_pools.clone());
        state
            .updated_v3_pools
            .extend(update.updated_uniswap_v3_pools);
        state
            .discovered_v4_pools
            .extend(update.discovered_uniswap_v4_pools);
        updated_v4_pools.extend(update.updated_uniswap_v4_pools.clone());
        state
            .updated_v4_pools
            .extend(update.updated_uniswap_v4_pools);
    }
    updated_v2_pools.sort();
    updated_v2_pools.dedup();
    updated_v3_pools.sort();
    updated_v3_pools.dedup();
    updated_v4_pools.sort();
    updated_v4_pools.dedup();

    let run_id = state.progress.id.clone();
    for error in report.transaction_errors {
        let issue = PipelineIssue::live_transaction_error(
            run_id.clone(),
            report.block_number,
            error.tx_index,
            error.tx_hash.clone(),
            error.message.clone(),
        );
        emit_issue(&issue);
        push_issue(state, issue);
        push_error(
            state,
            LiveTokenError::new(
                Some(report.block_number),
                Some(error.tx_index),
                Some(error.tx_hash),
                error.message,
            )
            .with_context("phase", "token_block_apply"),
        );
    }

    if let Some(retention_report) = retention_report {
        state.progress.retention_evaluated_tokens = retention_report.evaluated_tokens;
        state.progress.retention_dropped_tokens += retention_report.dropped_tokens;
        state.progress.retention_dropped_v2_pools += retention_report.dropped_v2_pool_count;
        state.last_retention_report = Some(retention_report);
    }

    refresh_progress_counts(state);
    let token_snapshots = token_snapshots_for_update(state, &updated_tokens);

    LiveTokenEvent::BlockApplied {
        block_number,
        block_hash,
        updated_tokens,
        updated_v2_pools,
        updated_v3_pools,
        updated_v4_pools,
        token_snapshots,
    }
}

fn token_snapshots_for_update(
    state: &LiveTokenState,
    updated_tokens: &[String],
) -> Vec<LiveTokenSnapshot> {
    let registry = state.processor.registry();
    let mut snapshots = updated_tokens
        .iter()
        .filter_map(|address| {
            registry
                .tokens
                .get(&normalize_address(address))
                .map(LiveTokenSnapshot::from_token)
        })
        .collect::<Vec<_>>();
    snapshots.sort_by(|left, right| left.contract_address.cmp(&right.contract_address));
    snapshots
}

pub(super) fn push_issue(state: &mut LiveTokenState, issue: PipelineIssue) {
    state.issues.push(issue);
    if state.issues.len() > MAX_LIVE_ISSUES {
        let excess = state.issues.len() - MAX_LIVE_ISSUES;
        state.issues.drain(0..excess);
    }
}

pub(super) fn push_bottleneck(state: &mut LiveTokenState, sample: PipelineBottleneckSample) {
    state.bottlenecks.push(sample);
    if state.bottlenecks.len() > MAX_LIVE_BOTTLENECKS {
        let excess = state.bottlenecks.len() - MAX_LIVE_BOTTLENECKS;
        state.bottlenecks.drain(0..excess);
    }
}

fn push_error(state: &mut LiveTokenState, error: LiveTokenError) {
    state.errors.push(error);
    if state.errors.len() > MAX_LIVE_ERRORS {
        let excess = state.errors.len() - MAX_LIVE_ERRORS;
        state.errors.drain(0..excess);
    }
}

fn refresh_progress_counts(state: &mut LiveTokenState) {
    state.progress.created_tokens_unique = state.created_tokens.len();
    state.progress.updated_tokens_unique = state.updated_tokens.len();
    state.progress.discovered_v2_pools_unique = state.discovered_v2_pools.len();
    state.progress.updated_v2_pools_unique = state.updated_v2_pools.len();
    state.progress.discovered_v3_pools_unique = state.discovered_v3_pools.len();
    state.progress.updated_v3_pools_unique = state.updated_v3_pools.len();
    state.progress.discovered_v4_pools_unique = state.discovered_v4_pools.len();
    state.progress.updated_v4_pools_unique = state.updated_v4_pools.len();
    state.progress.tracked_tokens = state.processor.registry().tokens.len();
    state.progress.indexed_tokens = state.processor.block_processor().token_index.entries.len();
    state.progress.indexed_pools = state
        .processor
        .block_processor()
        .token_index
        .pool_to_token
        .len();
    state.progress.indexed_v2_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum();
    state.progress.tracked_v2_pools = state.progress.indexed_v2_pools;
    state.progress.indexed_v3_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v3_pools.len())
        .sum();
    state.progress.tracked_v3_pools = state.progress.indexed_v3_pools;
    state.progress.indexed_v4_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v4_pools.len())
        .sum();
    state.progress.tracked_v4_pools = state.progress.indexed_v4_pools;
    state.progress.tracked_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.pool_count())
        .sum();
}
