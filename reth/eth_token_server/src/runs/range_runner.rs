use std::sync::Arc;
use std::time::Instant;

use eth_token::manager::{RethChainDiscoveryProvider, TokenBlockUpdateReport};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, ProcessedBlock, ProcessedBlockSource};

use super::processed_block_cache::{TokenProcessedBlockCacheKey, TokenProcessedBlockCacheStore};
use super::progress::{now_unix_secs, RunStatus};
use super::{RunError, TrackingRun, TrackingRunState};

const PROCESSED_BLOCK_CACHE_PRUNE_INTERVAL: u64 = 1_000;

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
    let chain_id = provider.chain_id();
    if let Some(cache_store) = processed_block_cache.as_deref() {
        prune_processed_block_cache(cache_store, chain_id, processed_block_cache_blocks);
    }

    for block_number in run.request.start_block..=run.request.end_block {
        if run.stop_requested() {
            mark_stopped(&run).await;
            return;
        }

        let block_started = Instant::now();
        let processed = match process_block(
            &tx_processor,
            block_number,
            processed_block_cache.as_deref(),
        )
        .await
        {
            Ok(processed) => processed,
            Err(error) => {
                mark_failed(
                    &run,
                    RunError {
                        block_number: Some(block_number),
                        tx_index: None,
                        tx_hash: None,
                        message: error.to_string(),
                    },
                )
                .await;
                return;
            }
        };
        let upstream_elapsed = block_started.elapsed();

        if let Some(cache_store) = processed_block_cache.as_deref() {
            let should_prune = block_number == run.request.end_block
                || block_number % PROCESSED_BLOCK_CACHE_PRUNE_INTERVAL == 0;
            if should_prune {
                prune_processed_block_cache(cache_store, chain_id, processed_block_cache_blocks);
            }
        }

        let token_apply_started = Instant::now();
        let mut state = run.state.write().await;
        let report = state
            .processor
            .process_block_with_discovery_provider(&processed.block, &discovery_provider)
            .await;
        let token_apply_elapsed = token_apply_started.elapsed();

        apply_report(
            &mut state,
            report,
            upstream_elapsed.as_millis(),
            token_apply_elapsed.as_millis(),
            &processed.cache_metrics,
        );
    }

    mark_completed(&run).await;
}

struct ProcessedBlockWithMetrics {
    block: ProcessedBlock,
    cache_metrics: ProcessedBlockCacheMetrics,
}

struct ProcessedBlockCacheMetrics {
    cache_hit: bool,
    cache_read_ms: u128,
    cache_write_ms: u128,
    source: &'static str,
}

async fn process_block(
    tx_processor: &BlockProcessor,
    block_number: u64,
    processed_block_cache: Option<&TokenProcessedBlockCacheStore>,
) -> eyre::Result<ProcessedBlockWithMetrics> {
    if let Some(cache_store) = processed_block_cache {
        let provider = tx_processor
            .provider()
            .ok_or_else(|| eyre::eyre!("cached block processing requires MDBX provider access"))?;
        let header = provider.fetch_block_header_only(block_number).await?;
        let key = TokenProcessedBlockCacheKey::new(provider.chain_id(), &header);

        let read_started = Instant::now();
        if let Some(block) = cache_store.get(&key)? {
            let cache_metrics = ProcessedBlockCacheMetrics {
                cache_hit: true,
                cache_read_ms: read_started.elapsed().as_millis(),
                cache_write_ms: 0,
                source: "token_cache",
            };
            return Ok(ProcessedBlockWithMetrics {
                block,
                cache_metrics,
            });
        }
        let cache_read_ms = read_started.elapsed().as_millis();

        let block = tx_processor.process_block(block_number).await?;
        if block.header.hash != header.hash {
            eyre::bail!(
                "processed block hash changed during token cache fill for {}: header {:?}, processed {:?}",
                block_number,
                header.hash,
                block.header.hash
            );
        }
        let write_started = Instant::now();
        cache_store.put(&key, &block)?;
        let cache_metrics = ProcessedBlockCacheMetrics {
            cache_hit: false,
            cache_read_ms,
            cache_write_ms: write_started.elapsed().as_millis(),
            source: ProcessedBlockSource::Processed.as_str(),
        };
        return Ok(ProcessedBlockWithMetrics {
            block,
            cache_metrics,
        });
    }

    let block = tx_processor.process_block(block_number).await?;
    Ok(ProcessedBlockWithMetrics {
        block,
        cache_metrics: ProcessedBlockCacheMetrics {
            cache_hit: false,
            cache_read_ms: 0,
            cache_write_ms: 0,
            source: ProcessedBlockSource::Processed.as_str(),
        },
    })
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
