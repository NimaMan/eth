use std::sync::Arc;
use std::time::Instant;

use eyre::{Result, WrapErr};
use reth_chain_query::RethQueryProvider;
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

/// Regular processed-block provider.
///
/// This is the historical path: read the local disk replay store first, and
/// process the block from Reth exactly once when the cache is missing.
#[derive(Clone)]
pub struct ProcessedBlockProvider {
    tx_processor: BlockProcessor,
    provider: Arc<RethQueryProvider>,
    replay_store_writer: Option<Arc<ProcessedBlockReplayStoreWriter>>,
}

impl ProcessedBlockProvider {
    pub fn new(
        tx_processor: BlockProcessor,
        provider: Arc<RethQueryProvider>,
        replay_store_writer: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        Self {
            tx_processor,
            provider,
            replay_store_writer,
        }
    }

    pub async fn load_block(&self, block_number: u64) -> Result<LoadedProcessedBlock> {
        load_processed_block(
            &self.tx_processor,
            self.provider.as_ref(),
            self.replay_store_writer.clone(),
            block_number,
        )
        .await
    }
}

pub async fn load_processed_block(
    tx_processor: &BlockProcessor,
    provider: &RethQueryProvider,
    replay_store_writer: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    block_number: u64,
) -> Result<LoadedProcessedBlock> {
    if let Some(replay_store_writer) = replay_store_writer {
        if let Some(cached) =
            load_cached_processed_block(provider, replay_store_writer.clone(), block_number).await?
        {
            return Ok(cached);
        }

        let started = Instant::now();
        let block = process_uncached_block(tx_processor, block_number)
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
    let block = process_uncached_block(tx_processor, block_number)
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

pub async fn load_cached_processed_block(
    provider: &RethQueryProvider,
    replay_store_writer: Arc<ProcessedBlockReplayStoreWriter>,
    block_number: u64,
) -> Result<Option<LoadedProcessedBlock>> {
    read_cached_block(
        replay_store_writer.clone(),
        provider.chain_id(),
        block_number,
    )
    .await
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

pub(super) async fn process_uncached_block(
    tx_processor: &BlockProcessor,
    block_number: u64,
) -> Result<ProcessedBlock> {
    process_uncached_block_with_options(
        tx_processor,
        block_number,
        true,
        BlockTraceEngine::default(),
    )
    .await
}

pub(super) async fn process_uncached_block_with_options(
    tx_processor: &BlockProcessor,
    block_number: u64,
    include_traces: bool,
    trace_engine: BlockTraceEngine,
) -> Result<ProcessedBlock> {
    match tx_processor
        .process_block_with_trace_engine(block_number, include_traces, trace_engine)
        .await
    {
        Ok(block) => Ok(block),
        Err(error) => {
            log_processed_block_replay_error(
                tx_processor,
                block_number,
                include_traces,
                trace_engine,
                &error,
            );
            Err(error)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProcessedBlockReplayErrorKind {
    LiveHistoricalContextLag,
    HistoricalTraceReplayMismatch,
    Other,
}

impl ProcessedBlockReplayErrorKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::LiveHistoricalContextLag => "live_historical_context_lag",
            Self::HistoricalTraceReplayMismatch => "historical_trace_replay_mismatch",
            Self::Other => "other",
        }
    }
}

fn latest_reth_block_number(tx_processor: &BlockProcessor) -> Option<u64> {
    tx_processor
        .provider()
        .and_then(|provider| provider.get_latest_block().ok())
}

fn classify_processed_block_replay_error(error: &eyre::Report) -> ProcessedBlockReplayErrorKind {
    let error_chain = error_chain(error);

    if is_live_historical_context_lag(&error_chain) {
        return ProcessedBlockReplayErrorKind::LiveHistoricalContextLag;
    }

    if is_historical_trace_replay_mismatch(&error_chain) {
        return ProcessedBlockReplayErrorKind::HistoricalTraceReplayMismatch;
    }

    ProcessedBlockReplayErrorKind::Other
}

fn log_processed_block_replay_error(
    tx_processor: &BlockProcessor,
    block_number: u64,
    include_traces: bool,
    trace_engine: BlockTraceEngine,
    error: &eyre::Report,
) {
    let error_kind = classify_processed_block_replay_error(error);
    if error_kind == ProcessedBlockReplayErrorKind::Other {
        return;
    }

    let latest_reth_block = latest_reth_block_number(tx_processor);
    tracing::error!(
        block_number,
        latest_reth_block,
        blocks_behind_latest = latest_reth_block.map(|latest| latest.saturating_sub(block_number)),
        include_traces,
        trace_engine = trace_engine.as_str(),
        replay_error_kind = error_kind.as_str(),
        error = %error,
        "processed block replay failed"
    );
}

fn error_chain(error: &eyre::Report) -> String {
    error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

fn is_live_historical_context_lag(error_chain: &str) -> bool {
    [
        "cannot restore live state snapshot",
        "not yet available as local historical context",
        "missing live block header",
        "Redis live state snapshot is missing",
        "unavailable from both Reth historical state and Redis live state",
        "failed to fetch historical state",
        "Reth historical state",
    ]
    .iter()
    .any(|needle| error_chain.contains(needle))
}

fn is_historical_trace_replay_mismatch(error_chain: &str) -> bool {
    if !error_chain.contains("failed to trace block transaction") {
        return false;
    }

    error_chain.contains("transaction validation error")
        || error_chain.contains("lack of funds")
        || error_chain.contains("nonce")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(message: &str) -> eyre::Report {
        eyre::eyre!("{message}")
    }

    #[test]
    fn classifies_trace_validation_lack_of_funds_as_replay_mismatch() {
        let error = report(
            "failed to trace block transaction block_number=25056257 tx_index=5: \
             transaction validation error: lack of funds (60968524683211705390) \
             for max fee (60968613646342838366)",
        );

        assert_eq!(
            classify_processed_block_replay_error(&error),
            ProcessedBlockReplayErrorKind::HistoricalTraceReplayMismatch
        );
    }

    #[test]
    fn does_not_retry_unrelated_processing_errors() {
        let error = report("failed to decode receipt for block 10");

        assert_eq!(
            classify_processed_block_replay_error(&error),
            ProcessedBlockReplayErrorKind::Other
        );
    }

    #[test]
    fn classifies_wrapped_trace_validation_error_as_replay_mismatch() {
        let error = report(
            "failed to trace block transaction block_number=25056257 tx_index=5: \
             transaction validation error: nonce too low",
        )
        .wrap_err("failed to process block 25056257");

        assert_eq!(
            classify_processed_block_replay_error(&error),
            ProcessedBlockReplayErrorKind::HistoricalTraceReplayMismatch
        );
    }

    #[test]
    fn classifies_live_snapshot_context_lag_as_transient() {
        let error = report(
            "cannot restore live state snapshot for block 25067028: \
             state for block 25067027 is unavailable from both Reth historical state \
             and Redis live state",
        )
        .wrap_err("failed to process uncached block 25067028");

        assert_eq!(
            classify_processed_block_replay_error(&error),
            ProcessedBlockReplayErrorKind::LiveHistoricalContextLag
        );
    }

    #[test]
    fn classifies_missing_live_header_as_transient() {
        let error = report(
            "missing live block header for 25067028 \
             (latest live Some(25067030), available [25067030, 25067029])",
        );

        assert_eq!(
            classify_processed_block_replay_error(&error),
            ProcessedBlockReplayErrorKind::LiveHistoricalContextLag
        );
    }
}
