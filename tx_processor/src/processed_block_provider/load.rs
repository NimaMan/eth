use std::sync::Arc;
use std::time::Instant;

use eyre::{Result, WrapErr};
use reth_chain_query::RethQueryProvider;
use tokio::time::{sleep, Duration};
use tx_simulator::block_simulation::BlockTraceEngine;

use crate::{
    BlockProcessor, ProcessedBlock, ProcessedBlockReplayStoreWriter, ProcessedBlockSource,
};

#[derive(Debug)]
pub struct LoadedProcessedBlock {
    pub block: ProcessedBlock,
    pub upstream_ms: u128,
    pub disk_cache_hit: bool,
    pub disk_cache_read_ms: u128,
    pub disk_cache_write_ms: u128,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessedBlockProviderRetry {
    pub attempts: usize,
    pub delay_ms: u64,
}

impl ProcessedBlockProviderRetry {
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
    replay_store_writer: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    block_number: u64,
    retry: ProcessedBlockProviderRetry,
) -> Result<LoadedProcessedBlock> {
    if let Some(replay_store_writer) = replay_store_writer {
        if let Some(cached) = load_cached_processed_block_with_retry(
            provider,
            replay_store_writer.clone(),
            block_number,
            retry,
        )
        .await?
        {
            return Ok(cached);
        }

        let started = Instant::now();
        let block = process_uncached_block_with_retry(tx_processor, block_number, retry)
            .await
            .wrap_err_with(|| format!("failed to process uncached block {block_number}"))?;
        let mut disk_cache_write_ms = 0;
        match replay_store_writer.write_processed_block(&block) {
            Ok(write) => {
                disk_cache_write_ms = write.disk_cache.write_ms;
            }
            Err(error) => {
                tracing::warn!(
                    block_number,
                    error = %error,
                    "failed to write processed block replay store entry"
                );
            }
        }

        return Ok(LoadedProcessedBlock {
            block,
            upstream_ms: started.elapsed().as_millis(),
            disk_cache_hit: false,
            disk_cache_read_ms: 0,
            disk_cache_write_ms,
            source: ProcessedBlockSource::Processed.as_str(),
        });
    }

    let started = Instant::now();
    let block = process_uncached_block_with_retry(tx_processor, block_number, retry)
        .await
        .wrap_err_with(|| format!("failed to process block {block_number}"))?;
    Ok(LoadedProcessedBlock {
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
    replay_store_writer: Arc<ProcessedBlockReplayStoreWriter>,
    block_number: u64,
    retry: ProcessedBlockProviderRetry,
) -> Result<Option<LoadedProcessedBlock>> {
    for attempt in 0..=retry.attempts {
        if let Some(cached) = read_cached_block(
            replay_store_writer.clone(),
            provider.chain_id(),
            block_number,
        )
        .await?
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
    replay_store_writer: Arc<ProcessedBlockReplayStoreWriter>,
    chain_id: u64,
    block_number: u64,
) -> Result<Option<LoadedProcessedBlock>> {
    let read_started = Instant::now();
    let key = tokio::task::spawn_blocking({
        let replay_store_writer = replay_store_writer.clone();
        move || {
            replay_store_writer
                .disk_cache_store()
                .cached_key_for_block_number(chain_id, block_number)
        }
    })
    .await
    .wrap_err("processed block disk cache key task failed")??;

    let Some(key) = key else {
        return Ok(None);
    };

    let block = tokio::task::spawn_blocking({
        let replay_store_writer = replay_store_writer.clone();
        let key = key.clone();
        move || replay_store_writer.disk_cache_store().get(&key)
    })
    .await
    .wrap_err("processed block disk cache read task failed")??;

    let Some(block) = block else {
        return Ok(None);
    };

    let read_ms = read_started.elapsed().as_millis();
    Ok(Some(LoadedProcessedBlock {
        block,
        upstream_ms: read_ms,
        disk_cache_hit: true,
        disk_cache_read_ms: read_ms,
        disk_cache_write_ms: 0,
        source: ProcessedBlockSource::Cache.as_str(),
    }))
}

pub(super) async fn process_uncached_block_with_retry(
    tx_processor: &BlockProcessor,
    block_number: u64,
    retry: ProcessedBlockProviderRetry,
) -> Result<ProcessedBlock> {
    process_uncached_block_with_options_retry(
        tx_processor,
        block_number,
        true,
        BlockTraceEngine::default(),
        retry,
    )
    .await
}

pub(super) async fn process_uncached_block_with_options_retry(
    tx_processor: &BlockProcessor,
    block_number: u64,
    include_traces: bool,
    trace_engine: BlockTraceEngine,
    retry: ProcessedBlockProviderRetry,
) -> Result<ProcessedBlock> {
    for attempt in 0..=retry.attempts {
        match tx_processor
            .process_block_with_trace_engine(block_number, include_traces, trace_engine)
            .await
        {
            Ok(block) => return Ok(block),
            Err(error) if attempt < retry.attempts && is_transient_reth_state_lag_error(&error) => {
                tracing::warn!(
                    block_number,
                    attempt = attempt + 1,
                    max_attempts = retry.attempts + 1,
                    error = %error,
                    "processed block replay failed while Reth state may still be catching up"
                );
                if retry.delay_ms > 0 {
                    sleep(Duration::from_millis(retry.delay_ms)).await;
                }
            }
            Err(error) => return Err(error),
        }
    }

    unreachable!("retry loop always returns before exhausting attempts")
}

fn is_transient_reth_state_lag_error(error: &eyre::Report) -> bool {
    let error_chain = error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ");

    error_chain.contains("failed to trace block transaction")
        && error_chain.contains("transaction validation error")
        && (error_chain.contains("lack of funds") || error_chain.contains("nonce"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(message: &str) -> eyre::Report {
        eyre::eyre!("{message}")
    }

    #[test]
    fn classifies_trace_validation_lack_of_funds_as_transient_state_lag() {
        let error = report(
            "failed to trace block transaction block_number=25056257 tx_index=5: \
             transaction validation error: lack of funds (60968524683211705390) \
             for max fee (60968613646342838366)",
        );

        assert!(is_transient_reth_state_lag_error(&error));
    }

    #[test]
    fn does_not_retry_unrelated_processing_errors() {
        let error = report("failed to decode receipt for block 10");

        assert!(!is_transient_reth_state_lag_error(&error));
    }

    #[test]
    fn classifies_wrapped_trace_validation_error() {
        let error = report(
            "failed to trace block transaction block_number=25056257 tx_index=5: \
             transaction validation error: nonce too low",
        )
        .wrap_err("failed to process block 25056257");

        assert!(is_transient_reth_state_lag_error(&error));
    }
}
