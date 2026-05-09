use std::{env, path::PathBuf, sync::Arc};

use alloy_primitives::B256;
use alloy_rpc_types_trace::geth::PreStateFrame;
use eyre::Result;
use reth_chain_query::RethQueryProvider;
use std::str::FromStr;
use tx_simulator::{live_chain_data::ChainStateSnapshot, TxSimulator};

use crate::live::{
    block_logger::BlockProcessingLogger,
    block_notifier::RedisBlockNotifier,
    block_snapshot::{build_live_block_snapshot, LiveBlockSnapshot},
    live_block_processor::{LiveBlockProcessor, LiveBlockProcessorConfig},
    processed_block_replay_store_sink::LiveProcessedBlockReplayStoreSink,
    redis_block_publisher::RedisBlockPublisher,
};

/// High-level service that wires the live block processor with storage + notifications.
pub struct LiveBlockService {
    processor: LiveBlockProcessor,
    publisher: Option<RedisBlockPublisher>,
    state_simulator: Option<Arc<TxSimulator>>,
    notifier: Option<RedisBlockNotifier>,
    logger: Option<BlockProcessingLogger>,
    processed_block_replay_store: Option<LiveProcessedBlockReplayStoreSink>,
}

impl LiveBlockService {
    pub async fn new(
        provider: Arc<RethQueryProvider>,
        processor_config: LiveBlockProcessorConfig,
        redis_url: Option<String>,
        notifier_channel: Option<String>,
        log_path: Option<PathBuf>,
        reth_datadir: PathBuf,
    ) -> Result<Self> {
        let state_simulator = redis_url.as_ref().map(|_| provider.simulator().clone());
        let chain_id = provider.chain_id();
        let processor = LiveBlockProcessor::connect(provider, processor_config).await?;
        let publisher = match redis_url.as_ref() {
            Some(url) => Some(RedisBlockPublisher::new(
                url,
                redis_ttl_seconds(),
                redis_max_blocks(),
                redis_processed_block_stream(),
            )?),
            None => None,
        };
        let notifier = match (redis_url.as_ref(), notifier_channel.as_ref()) {
            (Some(url), Some(channel)) => Some(RedisBlockNotifier::new(url, channel.clone())?),
            _ => None,
        };
        let logger_path = log_path.unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("logs")
                .join("block_processor")
                .join("live_block_processor.log")
        });
        let logger = Some(BlockProcessingLogger::new(logger_path)?);
        let processed_block_replay_store =
            match LiveProcessedBlockReplayStoreSink::from_config(chain_id, reth_datadir) {
                Ok(sink) => Some(sink),
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "failed to initialize live processed block replay store writer"
                    );
                    None
                }
            };

        Ok(Self {
            processor,
            publisher,
            state_simulator,
            notifier,
            logger,
            processed_block_replay_store,
        })
    }

    pub async fn run(self) -> Result<()> {
        self.run_internal(None).await
    }

    pub async fn run_for_blocks(self, limit: usize) -> Result<()> {
        self.run_internal(Some(limit)).await
    }

    pub async fn warmup_recent_blocks(&mut self, block_count: usize) -> Result<()> {
        if block_count == 0 {
            return Ok(());
        }

        let latest = self.processor.latest_block_number().await?;
        let start = latest.saturating_sub(block_count.saturating_sub(1) as u64);
        tracing::info!(
            warmup_blocks = block_count,
            start_block = start,
            end_block = latest,
            "warming live block processor"
        );

        for block_number in start..=latest {
            match self.processor.process_block_number(block_number).await {
                Ok(processed) => self.handle_processed_block(processed).await,
                Err(err) => {
                    tracing::warn!(block_number, "failed to warm live block processor: {}", err);
                }
            }
        }

        Ok(())
    }

    async fn run_internal(mut self, limit: Option<usize>) -> Result<()> {
        let mut processed_count: usize = 0;
        loop {
            let processed = self.processor.next_processed_block().await?;
            self.handle_processed_block(processed).await;

            processed_count += 1;
            if let Some(max) = limit {
                if processed_count >= max {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_processed_block(&self, processed: crate::live::LiveProcessedBlock) {
        let mut redis_published = self.publisher.is_none();
        if let Some(publisher) = &self.publisher {
            match build_live_block_snapshot(&processed.processed_block) {
                Ok(snapshot) => {
                    let state_snapshot = match (
                        self.state_simulator.as_ref(),
                        processed.state_diffs.as_deref(),
                    ) {
                        (Some(simulator), Some(state_diffs)) => {
                            match build_chain_state_snapshot(simulator, &snapshot, state_diffs)
                                .await
                            {
                                Ok(snapshot) => Some(snapshot),
                                Err(err) => {
                                    tracing::warn!(
                                        block_number = processed.execution_info.block_number,
                                        "failed to build tracked live state: {}",
                                        err
                                    );
                                    None
                                }
                            }
                        }
                        (Some(_), None) => {
                            tracing::warn!(
                                block_number = processed.execution_info.block_number,
                                "state diffs unavailable; tracked live state will not be advanced"
                            );
                            None
                        }
                        (None, _) => None,
                    };
                    if let Err(err) = publisher
                        .publish_snapshot(&snapshot, state_snapshot.as_ref())
                        .await
                    {
                        tracing::warn!(
                            block_number = processed.execution_info.block_number,
                            "failed to publish live block snapshot: {}",
                            err
                        );
                    } else {
                        redis_published = true;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        block_number = processed.execution_info.block_number,
                        "failed to build live block snapshot: {}",
                        err
                    );
                }
            }
        }

        if let Some(notifier) = &self.notifier {
            if let Err(err) = notifier
                .notify_block_processed(processed.execution_info.block_number)
                .await
            {
                tracing::warn!(
                    block_number = processed.execution_info.block_number,
                    "failed to publish live block notification: {}",
                    err
                );
            }
        }

        if let Some(logger) = &self.logger {
            if let Err(err) = logger.log_block(&processed) {
                tracing::warn!(
                    block_number = processed.execution_info.block_number,
                    "failed to write live block log: {}",
                    err
                );
            }
        }

        if redis_published {
            if let Some(replay_store) = &self.processed_block_replay_store {
                replay_store.try_enqueue(processed.processed_block);
            }
        }
    }
}

fn redis_ttl_seconds() -> Option<usize> {
    env_usize(&["ETH_LIVE_REDIS_TTL_SECONDS", "LIVE_BLOCK_REDIS_TTL_SECONDS"])
}

fn redis_max_blocks() -> Option<usize> {
    env_usize(&["ETH_LIVE_REDIS_MAX_BLOCKS", "LIVE_BLOCK_REDIS_MAX_BLOCKS"]).or(Some(10))
}

fn redis_processed_block_stream() -> String {
    env::var("ETH_PROCESSED_BLOCK_STREAM")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            tx_simulator::live_chain_data::live_data_registry::keys::processed_block_stream_key()
                .to_string()
        })
}

async fn build_chain_state_snapshot(
    simulator: &TxSimulator,
    block_snapshot: &LiveBlockSnapshot,
    state_diffs: &[PreStateFrame],
) -> Result<ChainStateSnapshot> {
    let header_payload = block_snapshot.header_json.as_deref().ok_or_else(|| {
        eyre::eyre!(
            "live block {} did not include a header payload",
            block_snapshot.block_number
        )
    })?;
    let parent_hash = block_snapshot.parent_hash.as_deref().ok_or_else(|| {
        eyre::eyre!(
            "live block {} did not include a parent hash",
            block_snapshot.block_number
        )
    })?;
    let block_hash = parse_b256(&block_snapshot.block_hash, "block hash")?;
    let parent_hash = parse_b256(parent_hash, "parent hash")?;

    simulator
        .build_live_state_snapshot_from_prestate_diffs(
            block_snapshot.block_number,
            block_hash,
            parent_hash,
            header_payload,
            state_diffs,
        )
        .await
}

fn parse_b256(value: &str, label: &str) -> Result<B256> {
    B256::from_str(value).map_err(|err| eyre::eyre!("invalid {} {}: {}", label, value, err))
}

fn env_usize(keys: &[&str]) -> Option<usize> {
    keys.iter().find_map(|key| {
        env::var(key)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
    })
}
