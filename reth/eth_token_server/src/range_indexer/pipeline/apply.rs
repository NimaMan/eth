use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Instant;

use eth_token::chain_metadata::HistoricalRethChainMetadataProvider;
use futures_util::FutureExt;
use tokio::time::{timeout, Duration};
use tx_processor::PoolBuySellSimulator;

use crate::range_indexer::{RangeIndexError, RangeIndexJob};

use super::cache::ProcessedBlockWithMetrics;
use super::state;

const TOKEN_APPLY_TIMEOUT: Duration = Duration::from_secs(180);

pub(super) async fn apply_processed_block(
    run: &Arc<RangeIndexJob>,
    processed: ProcessedBlockWithMetrics,
    discovery_provider: &HistoricalRethChainMetadataProvider<'_>,
    pool_simulator: &PoolBuySellSimulator,
) -> bool {
    let block_number = processed.block.header.number;
    let mut processor = state::take_processor_for_apply(run, block_number).await;
    let token_apply_started = Instant::now();
    let apply_future = processor.process_block_with_discovery_provider(
        &processed.block,
        discovery_provider,
        pool_simulator,
    );
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

    let token_apply_ms = token_apply_started.elapsed().as_millis();
    if token_apply_ms > 10_000 {
        tracing::warn!(
            block_number,
            token_apply_ms,
            "slow token tracking block apply"
        );
    }

    let mut run_state = run.state.write().await;
    run_state.processor = processor;
    state::apply_report(
        &mut run_state,
        report,
        processed.upstream_ms,
        token_apply_ms,
        &processed.cache_metrics,
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
