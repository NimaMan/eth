use std::collections::HashMap;
use std::time::Instant;

use futures::stream::{self, StreamExt, TryStreamExt};
use reth_chain_query::RethQueryProvider;

use crate::{
    BlockBatchOptions, BlockProcessor, ProcessedBlock, ProcessedBlockDiskCacheStore,
    ProcessedBlockReplayStoreWriter, ProcessedBlockSource,
};

use super::load::{process_uncached_block, process_uncached_block_with_options};

const PROCESSED_BLOCK_DISK_CACHE_PRUNE_INTERVAL: u64 = 1_000;
pub const DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH: u64 = 250;
pub const DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS: usize =
    DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH as usize;
pub const DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY: usize = 4;
pub const DEFAULT_PROCESSED_BLOCK_DISK_CACHE_READ_CONCURRENCY: usize = 2;

/// Adaptive default for concurrent fresh-block fill (cache misses). Each missing
/// block is traced on a `spawn_blocking` worker; the dominant cost (~0.7-1s) is
/// EVM replay + callTracer, which parallelizes cleanly across blocks. Profiling
/// showed throughput scaling to ~16-32 concurrency on a 32-core box (4 → ~333ms,
/// 16 → ~160ms per block), so the historical default of 4 left ~2-4x on the
/// table. Scale with available cores while leaving headroom for the live path,
/// clamped to a sane range; falls back to the const if core count is unknown.
pub fn default_processed_block_disk_cache_fill_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|cores| cores.get().saturating_sub(2).clamp(4, 16))
        .unwrap_or(DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY)
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessedBlockRangeLoadOptions {
    pub fill_batch_blocks: usize,
    pub fill_concurrency: usize,
    pub read_concurrency: usize,
}

impl Default for ProcessedBlockRangeLoadOptions {
    fn default() -> Self {
        Self {
            fill_batch_blocks: DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS,
            fill_concurrency: default_processed_block_disk_cache_fill_concurrency(),
            read_concurrency: DEFAULT_PROCESSED_BLOCK_DISK_CACHE_READ_CONCURRENCY,
        }
    }
}

impl ProcessedBlockRangeLoadOptions {
    pub fn with_fill_batch_blocks(mut self, value: usize) -> Self {
        if value > 0 {
            self.fill_batch_blocks = value;
        }
        self
    }

    pub fn with_fill_concurrency(mut self, value: usize) -> Self {
        if value > 0 {
            self.fill_concurrency = value;
        }
        self
    }

    pub fn with_read_concurrency(mut self, value: usize) -> Self {
        if value > 0 {
            self.read_concurrency = value;
        }
        self
    }
}

pub struct LoadedProcessedBlockWithMetrics {
    pub block: ProcessedBlock,
    pub upstream_ms: u128,
    pub disk_cache_metrics: ProcessedBlockLoadMetrics,
}

#[derive(Clone)]
struct DiskCacheFillMetrics {
    disk_cache_hit: bool,
    disk_cache_write_ms: u128,
    address_index_participating_txs: u64,
    address_index_inserted: u64,
    address_index_write_ms: u128,
    fill_ms: u128,
    source: &'static str,
}

struct FilledCacheEntry {
    block: ProcessedBlock,
    metrics: DiskCacheFillMetrics,
}

pub struct ProcessedBlockLoadMetrics {
    pub disk_cache_hit: bool,
    pub disk_cache_read_ms: u128,
    pub disk_cache_write_ms: u128,
    pub address_index_participating_txs: u64,
    pub address_index_inserted: u64,
    pub address_index_write_ms: u128,
    pub source: &'static str,
}

pub async fn load_processed_block_range(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    start_block: u64,
    end_block: u64,
    processed_block_replay_store: Option<&ProcessedBlockReplayStoreWriter>,
) -> eyre::Result<Vec<LoadedProcessedBlockWithMetrics>> {
    load_processed_block_range_with_options(
        tx_processor,
        provider,
        start_block,
        end_block,
        processed_block_replay_store,
        ProcessedBlockRangeLoadOptions::default(),
    )
    .await
}

pub async fn load_processed_block_range_with_options(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    start_block: u64,
    end_block: u64,
    processed_block_replay_store: Option<&ProcessedBlockReplayStoreWriter>,
    options: ProcessedBlockRangeLoadOptions,
) -> eyre::Result<Vec<LoadedProcessedBlockWithMetrics>> {
    if let Some(replay_store_writer) = processed_block_replay_store {
        return load_cached_block_range(
            tx_processor,
            provider,
            start_block,
            end_block,
            replay_store_writer,
            options,
        )
        .await;
    }

    let mut blocks = Vec::with_capacity((end_block - start_block + 1) as usize);
    for block_number in start_block..=end_block {
        let block_started = Instant::now();
        let block = process_uncached_block(tx_processor, block_number).await?;
        blocks.push(LoadedProcessedBlockWithMetrics {
            block,
            upstream_ms: block_started.elapsed().as_millis(),
            disk_cache_metrics: ProcessedBlockLoadMetrics {
                disk_cache_hit: false,
                disk_cache_read_ms: 0,
                disk_cache_write_ms: 0,
                address_index_participating_txs: 0,
                address_index_inserted: 0,
                address_index_write_ms: 0,
                source: ProcessedBlockSource::Processed.as_str(),
            },
        });
    }
    Ok(blocks)
}

async fn load_cached_block_range(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    start_block: u64,
    end_block: u64,
    replay_store_writer: &ProcessedBlockReplayStoreWriter,
    options: ProcessedBlockRangeLoadOptions,
) -> eyre::Result<Vec<LoadedProcessedBlockWithMetrics>> {
    let cache_store = replay_store_writer.disk_cache_store();
    let reader = cache_store.reader();
    let plan = reader.plan_range(provider, start_block, end_block).await?;
    let missing_count = plan.missing_keys.len();
    let mut fill_metrics_by_block = HashMap::with_capacity(plan.keys.len());
    for key in &plan.keys {
        fill_metrics_by_block.insert(
            key.block_number,
            DiskCacheFillMetrics {
                disk_cache_hit: true,
                disk_cache_write_ms: 0,
                address_index_participating_txs: 0,
                address_index_inserted: 0,
                address_index_write_ms: 0,
                fill_ms: 0,
                source: ProcessedBlockSource::Cache.as_str(),
            },
        );
    }

    if missing_count > 0 {
        tracing::info!(
            start_block,
            end_block,
            missing_blocks = missing_count,
            "filling missing processed block disk cache entries"
        );
    }

    let mut filled_blocks_by_block = HashMap::new();
    if missing_count > 0 {
        let (filled, fill_ms, write_wall_ms) = fill_cache_entries(
            tx_processor,
            replay_store_writer,
            &plan.missing_keys,
            options,
        )
        .await?;
        for (block_number, filled) in filled {
            fill_metrics_by_block.insert(block_number, filled.metrics.clone());
            filled_blocks_by_block.insert(block_number, filled);
        }
        tracing::info!(
            start_block,
            end_block,
            missing_blocks = missing_count,
            fill_ms,
            write_wall_ms,
            fill_batch_blocks = options.fill_batch_blocks.max(1),
            fill_concurrency = options.fill_concurrency.max(1),
            read_concurrency = options.read_concurrency.max(1),
            "filled missing processed block disk cache entries"
        );
    }

    let read_wall_started = Instant::now();
    let read_keys = plan.keys.clone();
    let reader_for_task = reader.clone();
    let read_concurrency = options.read_concurrency.max(1);
    let reads = tokio::task::spawn_blocking(move || {
        reader_for_task.get_many_parallel_with_limit(&read_keys, read_concurrency)
    })
    .await
    .map_err(|error| eyre::eyre!("processed block disk cache reader task failed: {error}"))??;
    let read_wall_ms = read_wall_started.elapsed().as_millis();
    let invalid_keys = reads
        .iter()
        .filter(|read| read.block.is_none())
        .map(|read| read.key.clone())
        .collect::<Vec<_>>();
    let invalid_count = invalid_keys.len();

    if invalid_count > 0 {
        tracing::info!(
            start_block,
            end_block,
            invalid_blocks = invalid_count,
            "rebuilding invalid processed block disk cache entries"
        );
        let (filled, fill_ms, write_wall_ms) =
            fill_cache_entries(tx_processor, replay_store_writer, &invalid_keys, options).await?;
        for (block_number, filled) in filled {
            fill_metrics_by_block.insert(block_number, filled.metrics.clone());
            filled_blocks_by_block.insert(block_number, filled);
        }
        tracing::info!(
            start_block,
            end_block,
            invalid_blocks = invalid_count,
            fill_ms,
            write_wall_ms,
            "rebuilt invalid processed block disk cache entries"
        );
    }

    tracing::info!(
        start_block,
        end_block,
        blocks = reads.len(),
        missing_blocks = missing_count,
        invalid_blocks = invalid_count,
        plan_ms = plan.plan_ms,
        read_wall_ms,
        read_concurrency,
        "read processed block disk cache chunk"
    );

    let mut blocks = Vec::with_capacity(reads.len());
    for read in reads {
        let fill_metrics = fill_metrics_by_block
            .remove(&read.key.block_number)
            .ok_or_else(|| {
                eyre::eyre!(
                    "processed block disk cache metrics missing for {}",
                    read.key.block_number
                )
            })?;
        let (block, disk_cache_read_ms) = match read.block {
            Some(block) => (block, ceil_ms(read.read_ms)),
            None => {
                let filled = filled_blocks_by_block
                    .remove(&read.key.block_number)
                    .ok_or_else(|| {
                        eyre::eyre!("cache miss after rebuild for {}", read.key.block_number)
                    })?;
                (filled.block, 0)
            }
        };
        blocks.push(LoadedProcessedBlockWithMetrics {
            block,
            upstream_ms: fill_metrics.fill_ms + disk_cache_read_ms,
            disk_cache_metrics: ProcessedBlockLoadMetrics {
                disk_cache_hit: fill_metrics.disk_cache_hit,
                disk_cache_read_ms,
                disk_cache_write_ms: fill_metrics.disk_cache_write_ms,
                address_index_participating_txs: fill_metrics.address_index_participating_txs,
                address_index_inserted: fill_metrics.address_index_inserted,
                address_index_write_ms: fill_metrics.address_index_write_ms,
                source: fill_metrics.source,
            },
        });
    }

    Ok(blocks)
}

async fn fill_cache_entries(
    tx_processor: &BlockProcessor,
    writer: &ProcessedBlockReplayStoreWriter,
    keys: &[crate::ProcessedBlockDiskCacheKey],
    options: ProcessedBlockRangeLoadOptions,
) -> eyre::Result<(HashMap<u64, FilledCacheEntry>, u128, u128)> {
    let keys_by_block = keys
        .iter()
        .map(|key| (key.block_number, key.clone()))
        .collect::<HashMap<_, _>>();
    let fill_started = Instant::now();
    let mut write_wall_ms = 0u128;
    let mut filled_blocks_by_block = HashMap::with_capacity(keys.len());

    for missing_chunk in keys.chunks(options.fill_batch_blocks.max(1)) {
        let missing_blocks = missing_chunk
            .iter()
            .map(|key| key.block_number)
            .collect::<Vec<_>>();
        let processed_missing_blocks = process_block_batch(
            tx_processor,
            missing_blocks,
            BlockBatchOptions::default().with_max_concurrency(options.fill_concurrency.max(1)),
        )
        .await?;

        for block in processed_missing_blocks {
            let key = keys_by_block.get(&block.header.number).ok_or_else(|| {
                eyre::eyre!(
                    "processed missing block {} was not part of the cache fill plan",
                    block.header.number
                )
            })?;
            let write = writer.write_processed_block(&block)?;
            write_wall_ms += write.total_write_ms();
            let address_index_write = write.address_block_index.as_ref();
            if write.disk_cache.key.chain_id != key.chain_id
                || write.disk_cache.key.block_number != key.block_number
            {
                eyre::bail!(
                    "processed block replay store writer produced unexpected key for {}: expected chain={} block={}, wrote {:?}",
                    key.block_number,
                    key.chain_id,
                    key.block_number,
                    write.disk_cache.key
                );
            }
            filled_blocks_by_block.insert(
                key.block_number,
                FilledCacheEntry {
                    block,
                    metrics: DiskCacheFillMetrics {
                        disk_cache_hit: false,
                        disk_cache_write_ms: write.disk_cache.write_ms,
                        address_index_participating_txs: address_index_write
                            .map(|write| write.participating_txs as u64)
                            .unwrap_or(0),
                        address_index_inserted: address_index_write
                            .map(|write| write.inserted as u64)
                            .unwrap_or(0),
                        address_index_write_ms: address_index_write
                            .map(|write| write.write_ms)
                            .unwrap_or(0),
                        fill_ms: fill_started.elapsed().as_millis(),
                        source: ProcessedBlockSource::Processed.as_str(),
                    },
                },
            );
        }
    }

    Ok((
        filled_blocks_by_block,
        fill_started.elapsed().as_millis(),
        write_wall_ms,
    ))
}

async fn process_block_batch<I>(
    tx_processor: &BlockProcessor,
    block_numbers: I,
    options: BlockBatchOptions,
) -> eyre::Result<Vec<ProcessedBlock>>
where
    I: IntoIterator<Item = u64>,
{
    let blocks: Vec<u64> = block_numbers.into_iter().collect();
    if blocks.is_empty() {
        return Ok(Vec::new());
    }

    let max_concurrency = options.max_concurrency.max(1);
    let processor = tx_processor.clone();
    let processed = stream::iter(blocks)
        .map(move |number| {
            let processor = processor.clone();
            async move {
                process_uncached_block_with_options(
                    &processor,
                    number,
                    options.include_traces,
                    options.trace_engine,
                )
                .await
                .map_err(|err| eyre::eyre!("failed to process block {number}: {err}"))
                .map(|block| (number, block))
            }
        })
        .buffer_unordered(max_concurrency)
        .try_collect::<Vec<_>>()
        .await?;

    let mut ordered = processed;
    ordered.sort_by_key(|(number, _)| *number);
    Ok(ordered.into_iter().map(|(_, block)| block).collect())
}

pub fn should_prune_processed_block_disk_cache(chunk_end: u64, end_block: u64) -> bool {
    chunk_end == end_block || chunk_end % PROCESSED_BLOCK_DISK_CACHE_PRUNE_INTERVAL == 0
}

pub fn prune_processed_block_disk_cache(
    cache_store: &ProcessedBlockDiskCacheStore,
    chain_id: u64,
    processed_block_disk_cache_blocks: u64,
) {
    match cache_store.prune_chain_to_recent_blocks(chain_id, processed_block_disk_cache_blocks) {
        Ok(0) => {}
        Ok(removed) => {
            tracing::info!(
                removed,
                retained = processed_block_disk_cache_blocks,
                "pruned processed block disk cache"
            );
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to prune processed block disk cache"
            );
        }
    }
}

fn ceil_ms(value: f64) -> u128 {
    value.ceil() as u128
}
