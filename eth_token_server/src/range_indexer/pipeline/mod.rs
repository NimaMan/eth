mod apply;
mod cache;
mod state;

use std::sync::Arc;

use eth_token::chain_metadata::RethChainMetadataProvider;
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, PoolBuySellSimulator, ProcessedBlockReplayStoreWriter};

use crate::memory;
use crate::range_indexer::{RangeIndexError, RangeIndexJob};

pub async fn run_range_index(
    run: Arc<RangeIndexJob>,
    provider: Arc<RethQueryProvider>,
    processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    processed_block_disk_cache_blocks: u64,
    processed_block_retry: cache::ProcessedBlockProviderRetry,
) {
    tracing::info!(
        run_id = %run.id,
        start_block = run.request.start_block,
        end_block = run.request.end_block,
        processed_block_replay_store = processed_block_replay_store.is_some(),
        "starting token tracking run"
    );

    state::mark_running(&run).await;

    let tx_processor = BlockProcessor::new(provider.clone());
    let discovery_provider = RethChainMetadataProvider::new(provider.as_ref());
    let pool_simulator = PoolBuySellSimulator::from_simulator(provider.simulator().clone());
    let chain_id = provider.chain_id();
    let processed_block_load_options =
        cache::ProcessedBlockRangeLoadOptions::default().with_retry(processed_block_retry);
    if let Some(replay_store) = processed_block_replay_store.as_deref() {
        cache::prune_processed_block_disk_cache(
            replay_store.disk_cache_store(),
            chain_id,
            processed_block_disk_cache_blocks,
        );
    }

    let mut next_block = run.request.start_block;
    while next_block <= run.request.end_block {
        if run.stop_requested() {
            state::mark_stopped(&run).await;
            return;
        }

        let chunk_end = if processed_block_replay_store.is_some() {
            next_block
                .saturating_add(cache::PROCESSED_BLOCK_DISK_CACHE_READ_BATCH - 1)
                .min(run.request.end_block)
        } else {
            next_block
        };

        if let Err(error) = provider.refresh_static_file_provider() {
            state::mark_failed(
                &run,
                RangeIndexError {
                    block_number: Some(next_block),
                    tx_index: None,
                    tx_hash: None,
                    message: format!("failed to refresh Reth static file provider: {error}"),
                },
            )
            .await;
            return;
        }

        {
            let processed_blocks = match cache::process_block_chunk(
                &tx_processor,
                provider.as_ref(),
                next_block,
                chunk_end,
                processed_block_replay_store.as_deref(),
                processed_block_load_options,
            )
            .await
            {
                Ok(processed) => processed,
                Err(error) => {
                    state::mark_failed(
                        &run,
                        RangeIndexError {
                            block_number: Some(next_block),
                            tx_index: None,
                            tx_hash: None,
                            message: error.to_string(),
                        },
                    )
                    .await;
                    return;
                }
            };

            if let Some(replay_store) = processed_block_replay_store.as_deref() {
                if cache::should_prune_processed_block_disk_cache(chunk_end, run.request.end_block)
                {
                    cache::prune_processed_block_disk_cache(
                        replay_store.disk_cache_store(),
                        chain_id,
                        processed_block_disk_cache_blocks,
                    );
                }
            }

            for processed in processed_blocks {
                if run.stop_requested() {
                    state::mark_stopped(&run).await;
                    return;
                }

                let applied = apply::apply_processed_block(
                    &run,
                    processed,
                    &discovery_provider,
                    &pool_simulator,
                )
                .await;
                if !applied {
                    return;
                }
            }
        }
        if memory::trim_allocator() {
            tracing::debug!(run_id = %run.id, chunk_end, "trimmed allocator after token range chunk");
        }

        if chunk_end == u64::MAX {
            break;
        }
        next_block = chunk_end + 1;
    }

    state::mark_completed(&run).await;
}
