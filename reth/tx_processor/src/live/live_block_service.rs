use std::{env, path::PathBuf, sync::Arc, time::Instant};

use alloy_primitives::B256;
use eyre::Result;
use reth_chain_query::RethQueryProvider;
use std::str::FromStr;
use tx_simulator::{live_chain_data::ChainStateSnapshot, TxSimulator};

use crate::live::{
    block_logger::BlockProcessingLogger,
    block_notifier::RedisBlockNotifier,
    block_snapshot::{build_live_block_snapshot, LiveBlockSnapshot},
    live_block_processor::{LiveBlockProcessor, LiveBlockProcessorConfig},
    redis_block_publisher::RedisBlockPublisher,
};

/// High-level service that wires the live block processor with storage + notifications.
pub struct LiveBlockService {
    processor: LiveBlockProcessor,
    publisher: Option<RedisBlockPublisher>,
    state_simulator: Option<Arc<TxSimulator>>,
    notifier: Option<RedisBlockNotifier>,
    logger: Option<BlockProcessingLogger>,
}

impl LiveBlockService {
    pub async fn new(
        provider: Arc<RethQueryProvider>,
        processor_config: LiveBlockProcessorConfig,
        redis_url: Option<String>,
        notifier_channel: Option<String>,
        log_path: Option<PathBuf>,
    ) -> Result<Self> {
        let state_simulator = redis_url.as_ref().map(|_| provider.simulator().clone());
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

        Ok(Self {
            processor,
            publisher,
            state_simulator,
            notifier,
            logger,
        })
    }

    pub async fn run(self) -> Result<()> {
        self.run_internal(None).await
    }

    pub async fn run_for_blocks(self, limit: usize) -> Result<()> {
        self.run_internal(Some(limit)).await
    }

    async fn run_internal(mut self, limit: Option<usize>) -> Result<()> {
        let mut processed_count: usize = 0;
        loop {
            let processed = self.processor.next_processed_block().await?;

            if let Some(publisher) = &self.publisher {
                match build_live_block_snapshot(&processed.processed_block) {
                    Ok(snapshot) => {
                        let state_snapshot = match self.state_simulator.as_ref() {
                            Some(simulator) => {
                                let started = Instant::now();
                                match build_chain_state_snapshot(simulator, &snapshot).await {
                                    Ok(snapshot) => {
                                        tracing::info!(
                                            block_number = processed.execution_info.block_number,
                                            state_snapshot_build_ms = started.elapsed().as_millis(),
                                            base_block_number = snapshot.base_block_number,
                                            accounts = snapshot.account_count(),
                                            contracts = snapshot.contract_count(),
                                            "built live state overlay snapshot"
                                        );
                                        Some(snapshot)
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            block_number = processed.execution_info.block_number,
                                            "failed to build live state overlay snapshot: {}",
                                            err
                                        );
                                        None
                                    }
                                }
                            }
                            None => None,
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

            processed_count += 1;
            if let Some(max) = limit {
                if processed_count >= max {
                    break;
                }
            }
        }
        Ok(())
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
    let mut tx_entries: Vec<_> = block_snapshot.tx_entries.iter().collect();
    tx_entries.sort_by_key(|entry| entry.tx_index);
    let tx_payloads: Vec<&str> = tx_entries
        .into_iter()
        .map(|entry| entry.payload_json.as_str())
        .collect();

    simulator
        .build_live_state_snapshot_from_processed_payloads(
            block_snapshot.block_number,
            block_hash,
            parent_hash,
            header_payload,
            &tx_payloads,
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
