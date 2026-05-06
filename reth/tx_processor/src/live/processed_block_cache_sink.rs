use std::{env, path::PathBuf};

use tokio::{sync::mpsc, task::JoinHandle};

use crate::{ProcessedBlock, TokenProcessedBlockCacheStore, TokenProcessedBlockCacheWriter};

const DEFAULT_RETAIN_BLOCKS: u64 = 100_000;
const DEFAULT_QUEUE_BLOCKS: usize = 256;
const PRUNE_INTERVAL_WRITES: u64 = 1_000;

/// Non-blocking background sink for writing live processed blocks into the
/// token-server disk cache after Redis publication has succeeded.
pub struct LiveProcessedBlockCacheSink {
    sender: mpsc::Sender<ProcessedBlock>,
    _worker: JoinHandle<()>,
}

impl LiveProcessedBlockCacheSink {
    pub fn from_config(chain_id: u64) -> eyre::Result<Self> {
        let cache_dir = processed_block_cache_dir()?;
        let retain_blocks = processed_block_cache_blocks();
        Self::new(cache_dir, chain_id, retain_blocks, DEFAULT_QUEUE_BLOCKS)
    }

    pub fn new(
        cache_dir: PathBuf,
        chain_id: u64,
        retain_blocks: u64,
        queue_blocks: usize,
    ) -> eyre::Result<Self> {
        let store = TokenProcessedBlockCacheStore::open(&cache_dir)?;
        let writer = store.writer(chain_id);
        let (sender, receiver) = mpsc::channel(queue_blocks.max(1));
        let worker = tokio::spawn(run_cache_writer(
            store,
            writer,
            receiver,
            chain_id,
            retain_blocks,
        ));

        tracing::info!(
            cache_dir = %cache_dir.display(),
            retain_blocks,
            queue_blocks = queue_blocks.max(1),
            "enabled live processed block disk cache writer"
        );

        Ok(Self {
            sender,
            _worker: worker,
        })
    }

    pub fn try_enqueue(&self, block: ProcessedBlock) {
        let block_number = block.header.number;
        match self.sender.try_send(block) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!(
                    block_number,
                    "live processed block cache queue is full; dropping cache write"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::warn!(
                    block_number,
                    "live processed block cache writer is closed; dropping cache write"
                );
            }
        }
    }
}

async fn run_cache_writer(
    store: TokenProcessedBlockCacheStore,
    writer: TokenProcessedBlockCacheWriter,
    mut receiver: mpsc::Receiver<ProcessedBlock>,
    chain_id: u64,
    retain_blocks: u64,
) {
    let mut successful_writes = 0_u64;
    while let Some(block) = receiver.recv().await {
        let block_number = block.header.number;
        let store = store.clone();
        let writer = writer.clone();
        let should_prune = (successful_writes + 1) % PRUNE_INTERVAL_WRITES == 0;

        let result = tokio::task::spawn_blocking(move || -> eyre::Result<(u128, usize)> {
            let write = writer.write_processed_block(&block)?;
            let pruned = if should_prune {
                store.prune_chain_to_recent_blocks(chain_id, retain_blocks)?
            } else {
                0
            };
            Ok((write.write_ms, pruned))
        })
        .await;

        match result {
            Ok(Ok((_write_ms, pruned))) => {
                successful_writes += 1;
                if pruned > 0 {
                    tracing::info!(
                        block_number,
                        pruned,
                        "pruned live processed block disk cache"
                    );
                }
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    block_number,
                    error = %error,
                    "failed to write live processed block to disk cache"
                );
            }
            Err(error) => {
                tracing::warn!(
                    block_number,
                    error = %error,
                    "live processed block cache writer task failed"
                );
            }
        }
    }
}

fn processed_block_cache_dir() -> eyre::Result<PathBuf> {
    if let Some(value) = non_empty_env("ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_DIR") {
        return Ok(PathBuf::from(value));
    }

    Ok(PathBuf::from(tx_simulator::config::repo::eth_node_root()?).join("processed_block_cache"))
}

fn processed_block_cache_blocks() -> u64 {
    non_empty_env("ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_BLOCKS")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_RETAIN_BLOCKS)
}

fn non_empty_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
