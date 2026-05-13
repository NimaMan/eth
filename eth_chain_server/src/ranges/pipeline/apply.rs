use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Instant;

use eth_token::chain_metadata::RethChainMetadataProvider;
use futures_util::FutureExt;
use tokio::time::{timeout, Duration};
use tracing::Instrument;
use tx_processor::PoolBuySellSimulator;

use crate::ranges::{RangeIndexError, RangeIndexJob};

use super::cache::ProcessedBlockWithMetrics;
use super::state;

const TOKEN_APPLY_TIMEOUT: Duration = Duration::from_secs(180);
const TOKEN_RANGE_APPLY_PROFILE_LOG_TARGET: &str = "token_range_apply_profile";

pub(super) async fn apply_processed_block(
    run: &Arc<RangeIndexJob>,
    processed: ProcessedBlockWithMetrics,
    discovery_provider: &RethChainMetadataProvider<'_>,
    pool_simulator: &PoolBuySellSimulator,
) -> bool {
    let apply_processed_block_started = Instant::now();
    let block_number = processed.block.header.number;
    let take_processor_started = Instant::now();
    let mut processor = state::take_processor_for_apply(run, block_number).await;
    let take_processor_us = take_processor_started.elapsed().as_micros();
    let token_apply_started = Instant::now();
    let apply_span = tracing::info_span!(
        "range_block_apply",
        run_id = %run.id,
        block_number
    );
    let apply_future = processor
        .process_block_with_discovery_provider_and_profile_run_id(
            &processed.block,
            discovery_provider,
            pool_simulator,
            Some(run.id.as_str()),
        )
        .instrument(apply_span);
    let report = match timeout(
        TOKEN_APPLY_TIMEOUT,
        AssertUnwindSafe(apply_future).catch_unwind(),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(payload)) => {
            state::restore_processor_after_apply(run, processor).await;
            state::mark_failed(
                run,
                RangeIndexError {
                    block_number: Some(block_number),
                    tx_index: None,
                    tx_hash: None,
                    message: format!(
                        "token tracking block apply panicked: {}",
                        panic_message(payload)
                    ),
                },
            )
            .await;
            return false;
        }
        Err(_) => {
            state::restore_processor_after_apply(run, processor).await;
            state::mark_failed(
                run,
                RangeIndexError {
                    block_number: Some(block_number),
                    tx_index: None,
                    tx_hash: None,
                    message: format!(
                        "token tracking block apply timed out after {}s",
                        TOKEN_APPLY_TIMEOUT.as_secs()
                    ),
                },
            )
            .await;
            return false;
        }
    };

    let token_apply_us = token_apply_started.elapsed().as_micros();
    let token_apply_ms = token_apply_us / 1_000;
    if token_apply_ms > 10_000 {
        tracing::warn!(
            block_number,
            token_apply_ms,
            "slow token tracking block apply"
        );
    }

    let state_update_started = Instant::now();
    let mut run_state = run.state.write().await;
    run_state.processor = processor;
    state::apply_report(
        &run.id,
        &mut run_state,
        report,
        processed.upstream_ms,
        token_apply_ms,
        &processed.disk_cache_metrics,
    );
    let state_update_us = state_update_started.elapsed().as_micros();
    let block_apply_wall_us = apply_processed_block_started.elapsed().as_micros();
    let measured_us = take_processor_us
        .saturating_add(token_apply_us)
        .saturating_add(state_update_us);
    let unaccounted_us = block_apply_wall_us.saturating_sub(measured_us);
    tracing::info!(
        target: TOKEN_RANGE_APPLY_PROFILE_LOG_TARGET,
        run_id = %run.id,
        block_number,
        block_apply_wall_us,
        unaccounted_us,
        take_processor_us,
        token_apply_us,
        state_update_us,
        disk_cache_read_us = processed.disk_cache_metrics.disk_cache_read_ms.saturating_mul(1_000),
        block_apply_wall_ms = block_apply_wall_us / 1_000,
        unaccounted_ms = unaccounted_us / 1_000,
        take_processor_ms = take_processor_us / 1_000,
        token_apply_ms = token_apply_us / 1_000,
        state_update_ms = state_update_us / 1_000,
        disk_cache_read_ms = processed.disk_cache_metrics.disk_cache_read_ms,
        "range block apply profile"
    );
    true
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}
