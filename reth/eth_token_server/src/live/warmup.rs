use std::sync::Arc;
use std::time::Instant;

use eyre::{Result, WrapErr};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, ProcessedBlock, ProcessedBlockSource};

use crate::processed_block_cache::TokenProcessedBlockCacheStore;

#[derive(Debug)]
pub struct LiveWarmupBlock {
    pub block: ProcessedBlock,
    pub upstream_ms: u128,
    pub cache_hit: bool,
    pub cache_read_ms: u128,
    pub cache_write_ms: u128,
    pub source: &'static str,
}

pub async fn load_processed_block(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    cache_store: Option<Arc<TokenProcessedBlockCacheStore>>,
    block_number: u64,
) -> Result<LiveWarmupBlock> {
    if let Some(cache_store) = cache_store {
        if let Some(cached) =
            read_cached_block(cache_store.clone(), provider.chain_id(), block_number).await?
        {
            return Ok(cached);
        }

        let started = Instant::now();
        let block = tx_processor.process_block(block_number).await?;
        let mut cache_write_ms = 0;
        match cache_store
            .writer(provider.chain_id())
            .write_processed_block(&block)
        {
            Ok(write) => {
                cache_write_ms = write.write_ms;
            }
            Err(error) => {
                tracing::warn!(
                    block_number,
                    error = %error,
                    "failed to write live warmup processed block cache entry"
                );
            }
        }

        return Ok(LiveWarmupBlock {
            block,
            upstream_ms: started.elapsed().as_millis(),
            cache_hit: false,
            cache_read_ms: 0,
            cache_write_ms,
            source: ProcessedBlockSource::Processed.as_str(),
        });
    }

    let started = Instant::now();
    let block = tx_processor.process_block(block_number).await?;
    Ok(LiveWarmupBlock {
        block,
        upstream_ms: started.elapsed().as_millis(),
        cache_hit: false,
        cache_read_ms: 0,
        cache_write_ms: 0,
        source: ProcessedBlockSource::Processed.as_str(),
    })
}

async fn read_cached_block(
    cache_store: Arc<TokenProcessedBlockCacheStore>,
    chain_id: u64,
    block_number: u64,
) -> Result<Option<LiveWarmupBlock>> {
    let read_started = Instant::now();
    let key = tokio::task::spawn_blocking({
        let cache_store = cache_store.clone();
        move || cache_store.cached_key_for_block_number(chain_id, block_number)
    })
    .await
    .wrap_err("live warmup cache key task failed")??;

    let Some(key) = key else {
        return Ok(None);
    };

    let block = tokio::task::spawn_blocking({
        let cache_store = cache_store.clone();
        let key = key.clone();
        move || cache_store.get(&key)
    })
    .await
    .wrap_err("live warmup cache read task failed")??;

    let Some(block) = block else {
        return Ok(None);
    };

    Ok(Some(LiveWarmupBlock {
        block,
        upstream_ms: read_started.elapsed().as_millis(),
        cache_hit: true,
        cache_read_ms: read_started.elapsed().as_millis(),
        cache_write_ms: 0,
        source: "token_cache",
    }))
}
