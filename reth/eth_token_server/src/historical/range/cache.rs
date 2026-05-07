use std::collections::HashMap;
use std::time::Instant;

use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockBatchOptions, BlockProcessor, ProcessedBlock, ProcessedBlockSource};

use crate::processed_block_cache::TokenProcessedBlockCacheStore;

const PROCESSED_BLOCK_CACHE_PRUNE_INTERVAL: u64 = 1_000;
pub(super) const PROCESSED_BLOCK_CACHE_READ_BATCH: u64 = 250;
const PROCESSED_BLOCK_CACHE_FILL_BATCH_BLOCKS: usize = 100;
const PROCESSED_BLOCK_CACHE_FILL_CONCURRENCY: usize = 10;

pub(super) struct ProcessedBlockWithMetrics {
    pub(super) block: ProcessedBlock,
    pub(super) upstream_ms: u128,
    pub(super) cache_metrics: ProcessedBlockCacheMetrics,
}

struct CacheFillMetrics {
    cache_hit: bool,
    cache_write_ms: u128,
    fill_ms: u128,
    source: &'static str,
}

pub(super) struct ProcessedBlockCacheMetrics {
    pub(super) cache_hit: bool,
    pub(super) cache_read_ms: u128,
    pub(super) cache_write_ms: u128,
    pub(super) source: &'static str,
}

pub(super) async fn process_block_chunk(
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

pub(super) fn should_prune_processed_block_cache(chunk_end: u64, end_block: u64) -> bool {
    chunk_end == end_block || chunk_end % PROCESSED_BLOCK_CACHE_PRUNE_INTERVAL == 0
}

pub(super) fn prune_processed_block_cache(
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

fn ceil_ms(value: f64) -> u128 {
    value.ceil() as u128
}
