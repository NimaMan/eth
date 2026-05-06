use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Instant;

use eth_token::manager::{BlockTokenProcessor, RethChainDiscoveryProvider, TokenBlockUpdateReport};
use futures_util::FutureExt;
use reth_chain_query::RethQueryProvider;
use tokio::time::{timeout, Duration};
use tx_processor::{
    BlockBatchOptions, BlockProcessor, PoolBuySellSimulator, ProcessedBlock, ProcessedBlockSource,
};

use crate::processed_block_cache::TokenProcessedBlockCacheStore;

use super::progress::{now_unix_secs, RunStatus};
use super::{RunError, TrackingRun, TrackingRunState};

const PROCESSED_BLOCK_CACHE_PRUNE_INTERVAL: u64 = 1_000;
const PROCESSED_BLOCK_CACHE_READ_BATCH: u64 = 250;
const PROCESSED_BLOCK_CACHE_FILL_BATCH_BLOCKS: usize = 100;
const PROCESSED_BLOCK_CACHE_FILL_CONCURRENCY: usize = 10;
const TOKEN_APPLY_TIMEOUT: Duration = Duration::from_secs(180);

pub async fn run_range(
    run: Arc<TrackingRun>,
    provider: Arc<RethQueryProvider>,
    processed_block_cache: Option<Arc<TokenProcessedBlockCacheStore>>,
    processed_block_cache_blocks: u64,
) {
    tracing::info!(
        run_id = %run.id,
        start_block = run.request.start_block,
        end_block = run.request.end_block,
        processed_block_cache = processed_block_cache.is_some(),
        "starting token tracking run"
    );

    mark_running(&run).await;

    let tx_processor = BlockProcessor::new(provider.clone());
    let discovery_provider = RethChainDiscoveryProvider::new(provider.as_ref());
    let pool_simulator = PoolBuySellSimulator::from_simulator(provider.simulator().clone());
    let chain_id = provider.chain_id();
    if let Some(cache_store) = processed_block_cache.as_deref() {
        prune_processed_block_cache(cache_store, chain_id, processed_block_cache_blocks);
    }

    let mut next_block = run.request.start_block;
    while next_block <= run.request.end_block {
        if run.stop_requested() {
            mark_stopped(&run).await;
            return;
        }

        let chunk_end = if processed_block_cache.is_some() {
            next_block
                .saturating_add(PROCESSED_BLOCK_CACHE_READ_BATCH - 1)
                .min(run.request.end_block)
        } else {
            next_block
        };

        let processed_blocks = match process_block_chunk(
            &tx_processor,
            provider.as_ref(),
            next_block,
            chunk_end,
            processed_block_cache.as_deref(),
        )
        .await
        {
            Ok(processed) => processed,
            Err(error) => {
                mark_failed(
                    &run,
                    RunError {
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

        if let Some(cache_store) = processed_block_cache.as_deref() {
            let should_prune = chunk_end == run.request.end_block
                || chunk_end % PROCESSED_BLOCK_CACHE_PRUNE_INTERVAL == 0;
            if should_prune {
                prune_processed_block_cache(cache_store, chain_id, processed_block_cache_blocks);
            }
        }

        for processed in processed_blocks {
            if run.stop_requested() {
                mark_stopped(&run).await;
                return;
            }

            let block_number = processed.block.header.number;
            let mut processor = take_processor_for_apply(&run, block_number).await;
            let token_apply_started = Instant::now();
            let apply_future = processor.process_block_with_discovery_provider(
                &processed.block,
                &discovery_provider,
                &pool_simulator,
            );
            let report = match timeout(
                TOKEN_APPLY_TIMEOUT,
                AssertUnwindSafe(apply_future).catch_unwind(),
            )
            .await
            {
                Ok(Ok(report)) => report,
                Ok(Err(payload)) => {
                    restore_processor_after_apply(&run, processor).await;
                    mark_failed(
                        &run,
                        RunError {
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
                    return;
                }
                Err(_) => {
                    restore_processor_after_apply(&run, processor).await;
                    mark_failed(
                        &run,
                        RunError {
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
                    return;
                }
            };
            let token_apply_elapsed = token_apply_started.elapsed();
            let token_apply_ms = token_apply_elapsed.as_millis();
            if token_apply_ms > 10_000 {
                tracing::warn!(
                    block_number,
                    token_apply_ms,
                    "slow token tracking block apply"
                );
            }

            let mut state = run.state.write().await;
            state.processor = processor;
            apply_report(
                &mut state,
                report,
                processed.upstream_ms,
                token_apply_ms,
                &processed.cache_metrics,
            );
        }

        if chunk_end == u64::MAX {
            break;
        }
        next_block = chunk_end + 1;
    }

    mark_completed(&run).await;
}

struct ProcessedBlockWithMetrics {
    block: ProcessedBlock,
    upstream_ms: u128,
    cache_metrics: ProcessedBlockCacheMetrics,
}

struct CacheFillMetrics {
    cache_hit: bool,
    cache_write_ms: u128,
    fill_ms: u128,
    source: &'static str,
}

struct ProcessedBlockCacheMetrics {
    cache_hit: bool,
    cache_read_ms: u128,
    cache_write_ms: u128,
    source: &'static str,
}

async fn process_block_chunk(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    start_block: u64,
    end_block: u64,
    processed_block_cache: Option<&TokenProcessedBlockCacheStore>,
) -> eyre::Result<Vec<ProcessedBlockWithMetrics>> {
    if let Some(cache_store) = processed_block_cache {
        return process_cached_block_chunk(
            tx_processor,
            provider,
            start_block,
            end_block,
            cache_store,
        )
        .await;
    }

    let mut blocks = Vec::with_capacity((end_block - start_block + 1) as usize);
    for block_number in start_block..=end_block {
        let block_started = Instant::now();
        let block = tx_processor.process_block(block_number).await?;
        blocks.push(ProcessedBlockWithMetrics {
            block,
            upstream_ms: block_started.elapsed().as_millis(),
            cache_metrics: ProcessedBlockCacheMetrics {
                cache_hit: false,
                cache_read_ms: 0,
                cache_write_ms: 0,
                source: ProcessedBlockSource::Processed.as_str(),
            },
        });
    }
    Ok(blocks)
}

async fn process_cached_block_chunk(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    start_block: u64,
    end_block: u64,
    cache_store: &TokenProcessedBlockCacheStore,
) -> eyre::Result<Vec<ProcessedBlockWithMetrics>> {
    let reader = cache_store.reader();
    let plan = reader.plan_range(provider, start_block, end_block).await?;
    let missing_count = plan.missing_keys.len();
    let mut fill_metrics_by_block = HashMap::with_capacity(plan.keys.len());
    for key in &plan.keys {
        fill_metrics_by_block.insert(
            key.block_number,
            CacheFillMetrics {
                cache_hit: true,
                cache_write_ms: 0,
                fill_ms: 0,
                source: "token_cache",
            },
        );
    }

    if missing_count > 0 {
        tracing::info!(
            start_block,
            end_block,
            missing_blocks = missing_count,
            "filling missing processed block cache entries"
        );
    }

    let writer = cache_store.writer(provider.chain_id());
    if missing_count > 0 {
        let missing_keys_by_block = plan
            .missing_keys
            .iter()
            .map(|key| (key.block_number, key.clone()))
            .collect::<HashMap<_, _>>();

        let fill_started = Instant::now();
        let mut write_wall_ms = 0u128;
        for missing_chunk in plan
            .missing_keys
            .chunks(PROCESSED_BLOCK_CACHE_FILL_BATCH_BLOCKS)
        {
            let missing_blocks = missing_chunk
                .iter()
                .map(|key| key.block_number)
                .collect::<Vec<_>>();
            let processed_missing_blocks = tx_processor
                .process_block_batch(
                    missing_blocks,
                    BlockBatchOptions::default()
                        .with_max_concurrency(PROCESSED_BLOCK_CACHE_FILL_CONCURRENCY),
                )
                .await?;

            for block in processed_missing_blocks {
                let key = missing_keys_by_block
                    .get(&block.header.number)
                    .ok_or_else(|| {
                        eyre::eyre!(
                            "processed missing block {} was not part of the cache fill plan",
                            block.header.number
                        )
                    })?;
                if block.header.hash != key.block_hash {
                    eyre::bail!(
                "processed block hash changed during token cache fill for {}: header {:?}, processed {:?}",
                key.block_number,
                key.block_hash,
                block.header.hash
            );
                }
                let write = writer.write_processed_block(&block)?;
                write_wall_ms += write.write_ms;
                if write.key != *key {
                    eyre::bail!(
                "processed block cache writer produced unexpected key for {}: expected {:?}, wrote {:?}",
                key.block_number,
                key,
                write.key
            );
                }
                fill_metrics_by_block.insert(
                    key.block_number,
                    CacheFillMetrics {
                        cache_hit: false,
                        cache_write_ms: write.write_ms,
                        fill_ms: fill_started.elapsed().as_millis(),
                        source: ProcessedBlockSource::Processed.as_str(),
                    },
                );
            }
        }
        let fill_ms = fill_started.elapsed().as_millis();
        tracing::info!(
            start_block,
            end_block,
            missing_blocks = missing_count,
            fill_ms,
            write_wall_ms,
            fill_batch_blocks = PROCESSED_BLOCK_CACHE_FILL_BATCH_BLOCKS,
            fill_concurrency = PROCESSED_BLOCK_CACHE_FILL_CONCURRENCY,
            "filled missing processed block cache entries"
        );
    }

    let read_wall_started = Instant::now();
    let read_keys = plan.keys.clone();
    let reader_for_task = reader.clone();
    let reads = tokio::task::spawn_blocking(move || reader_for_task.get_many_parallel(&read_keys))
        .await
        .map_err(|error| eyre::eyre!("processed block cache reader task failed: {error}"))??;
    let read_wall_ms = read_wall_started.elapsed().as_millis();

    tracing::info!(
        start_block,
        end_block,
        blocks = reads.len(),
        missing_blocks = missing_count,
        read_wall_ms,
        "read processed block cache chunk"
    );

    let mut blocks = Vec::with_capacity(reads.len());
    for read in reads {
        let fill_metrics = fill_metrics_by_block
            .remove(&read.key.block_number)
            .ok_or_else(|| {
                eyre::eyre!(
                    "processed block cache metrics missing for {}",
                    read.key.block_number
                )
            })?;
        let block = read
            .block
            .ok_or_else(|| eyre::eyre!("cache miss after fill for {}", read.key.block_number))?;
        let cache_read_ms = ceil_ms(read.read_ms);
        blocks.push(ProcessedBlockWithMetrics {
            block,
            upstream_ms: fill_metrics.fill_ms + cache_read_ms,
            cache_metrics: ProcessedBlockCacheMetrics {
                cache_hit: fill_metrics.cache_hit,
                cache_read_ms,
                cache_write_ms: fill_metrics.cache_write_ms,
                source: fill_metrics.source,
            },
        });
    }

    Ok(blocks)
}

fn ceil_ms(value: f64) -> u128 {
    value.ceil() as u128
}

async fn take_processor_for_apply(run: &TrackingRun, block_number: u64) -> BlockTokenProcessor {
    let mut state = run.state.write().await;
    state.progress.current_block = Some(block_number);
    state.progress.updated_at_unix_secs = now_unix_secs();
    std::mem::replace(
        &mut state.processor,
        BlockTokenProcessor::new(run.request.history_limit),
    )
}

async fn restore_processor_after_apply(run: &TrackingRun, processor: BlockTokenProcessor) {
    let mut state = run.state.write().await;
    state.processor = processor;
    state.progress.updated_at_unix_secs = now_unix_secs();
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

fn prune_processed_block_cache(
    cache_store: &TokenProcessedBlockCacheStore,
    chain_id: u64,
    processed_block_cache_blocks: u64,
) {
    match cache_store.prune_chain_to_recent_blocks(chain_id, processed_block_cache_blocks) {
        Ok(0) => {}
        Ok(removed) => {
            tracing::info!(
                removed,
                retained = processed_block_cache_blocks,
                "pruned processed block cache"
            );
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to prune processed block cache"
            );
        }
    }
}

async fn mark_running(run: &TrackingRun) {
    let mut state = run.state.write().await;
    state.progress.status = RunStatus::Running;
    state.progress.updated_at_unix_secs = now_unix_secs();
}

async fn mark_completed(run: &TrackingRun) {
    let mut state = run.state.write().await;
    state.progress.status = RunStatus::Completed;
    state.progress.completed_at_unix_secs = Some(now_unix_secs());
    state.progress.updated_at_unix_secs = now_unix_secs();
    tracing::info!(run_id = %run.id, "completed token tracking run");
}

async fn mark_stopped(run: &TrackingRun) {
    let mut state = run.state.write().await;
    state.progress.status = RunStatus::Stopped;
    state.progress.completed_at_unix_secs = Some(now_unix_secs());
    state.progress.updated_at_unix_secs = now_unix_secs();
    tracing::info!(run_id = %run.id, "stopped token tracking run");
}

async fn mark_failed(run: &TrackingRun, error: RunError) {
    let mut state = run.state.write().await;
    state.progress.status = RunStatus::Failed;
    state.progress.last_error = Some(error.message.clone());
    state.progress.completed_at_unix_secs = Some(now_unix_secs());
    state.progress.updated_at_unix_secs = now_unix_secs();
    state.errors.push(error);
    tracing::warn!(run_id = %run.id, error = ?state.progress.last_error, "failed token tracking run");
}

fn apply_report(
    state: &mut TrackingRunState,
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
        state.errors.push(RunError {
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
