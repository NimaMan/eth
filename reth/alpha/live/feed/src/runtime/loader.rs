use std::sync::Arc;
use std::time::Instant;

use eyre::{Result, WrapErr};
use reth_chain_query::RethQueryProvider;
use tokio::time::{sleep, Duration};
use tx_processor::{
    BlockProcessor, ProcessedBlock, ProcessedBlockDiskCacheStore, ProcessedBlockSource,
};

const PROCESSED_BLOCK_DISK_CACHE_SOURCE: &str = "processed_block_disk_cache";

#[derive(Debug)]
pub struct LiveBlockLoad {
    pub block: ProcessedBlock,
    pub upstream_ms: u128,
    pub disk_cache_hit: bool,
    pub disk_cache_read_ms: u128,
    pub disk_cache_write_ms: u128,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessedBlockDiskCacheRetry {
    pub attempts: usize,
    pub delay_ms: u64,
}

impl ProcessedBlockDiskCacheRetry {
    pub const fn none() -> Self {
        Self {
            attempts: 0,
            delay_ms: 0,
        }
    }
}

pub async fn load_processed_block(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    cache_store: Option<Arc<ProcessedBlockDiskCacheStore>>,
    block_number: u64,
    retry: ProcessedBlockDiskCacheRetry,
) -> Result<LiveBlockLoad> {
    if let Some(cache_store) = cache_store {
        if let Some(cached) = load_cached_processed_block_with_retry(
            provider,
            cache_store.clone(),
            block_number,
            retry,
        )
        .await?
        {
            return Ok(cached);
        }

        let started = Instant::now();
        let block = tx_processor.process_block(block_number).await?;
        let mut disk_cache_write_ms = 0;
        match cache_store
            .writer(provider.chain_id())
            .write_processed_block(&block)
        {
            Ok(write) => {
                disk_cache_write_ms = write.write_ms;
            }
            Err(error) => {
                tracing::warn!(
                    block_number,
                    error = %error,
                    "failed to write live processed block disk cache entry"
                );
            }
        }

        return Ok(LiveBlockLoad {
            block,
            upstream_ms: started.elapsed().as_millis(),
            disk_cache_hit: false,
            disk_cache_read_ms: 0,
            disk_cache_write_ms,
            source: ProcessedBlockSource::Processed.as_str(),
        });
    }

    let started = Instant::now();
    let block = tx_processor.process_block(block_number).await?;
    Ok(LiveBlockLoad {
        block,
        upstream_ms: started.elapsed().as_millis(),
        disk_cache_hit: false,
        disk_cache_read_ms: 0,
        disk_cache_write_ms: 0,
        source: ProcessedBlockSource::Processed.as_str(),
    })
}

pub async fn load_cached_processed_block_with_retry(
    provider: &RethQueryProvider,
    cache_store: Arc<ProcessedBlockDiskCacheStore>,
    block_number: u64,
    retry: ProcessedBlockDiskCacheRetry,
) -> Result<Option<LiveBlockLoad>> {
    for attempt in 0..=retry.attempts {
        if let Some(cached) =
            read_cached_block(cache_store.clone(), provider.chain_id(), block_number).await?
        {
            return Ok(Some(cached));
        }
        if attempt < retry.attempts && retry.delay_ms > 0 {
            sleep(Duration::from_millis(retry.delay_ms)).await;
        }
    }

    Ok(None)
}

async fn read_cached_block(
    cache_store: Arc<ProcessedBlockDiskCacheStore>,
    chain_id: u64,
    block_number: u64,
) -> Result<Option<LiveBlockLoad>> {
    let read_started = Instant::now();
    let key = tokio::task::spawn_blocking({
        let cache_store = cache_store.clone();
        move || cache_store.cached_key_for_block_number(chain_id, block_number)
    })
    .await
    .wrap_err("processed block disk cache key task failed")??;

    let Some(key) = key else {
        return Ok(None);
    };

    let block = tokio::task::spawn_blocking({
        let cache_store = cache_store.clone();
        let key = key.clone();
        move || cache_store.get(&key)
    })
    .await
    .wrap_err("processed block disk cache read task failed")??;

    let Some(block) = block else {
        return Ok(None);
    };

    Ok(Some(LiveBlockLoad {
        block,
        upstream_ms: read_started.elapsed().as_millis(),
        disk_cache_hit: true,
        disk_cache_read_ms: read_started.elapsed().as_millis(),
        disk_cache_write_ms: 0,
        source: PROCESSED_BLOCK_DISK_CACHE_SOURCE,
    }))
}
