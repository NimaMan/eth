use std::{env, path::PathBuf};

use tokio::{sync::mpsc, task::JoinHandle};

use crate::{ProcessedBlock, ProcessedBlockReplayStoreWriter};

const PROCESSED_BLOCK_DISK_CACHE_DIR_ENV: &str = "ETH_CHAIN_SERVER_PROCESSED_BLOCK_DISK_CACHE_DIR";
const PROCESSED_BLOCK_DISK_CACHE_BLOCKS_ENV: &str =
    "ETH_CHAIN_SERVER_PROCESSED_BLOCK_DISK_CACHE_BLOCKS";
const PROCESSED_BLOCK_DISK_CACHE_DIR_NAME: &str = "processed-block-cache";
const DEFAULT_RETAIN_BLOCKS: u64 = 1_000_000;
const DEFAULT_QUEUE_BLOCKS: usize = 256;
const PRUNE_INTERVAL_WRITES: u64 = 1_000;

/// Non-blocking background sink for writing live processed blocks into the
/// replay store after a live processed block has been applied.
pub struct LiveProcessedBlockReplayStoreSink {
    sender: mpsc::Sender<ProcessedBlock>,
    _worker: JoinHandle<()>,
}

impl LiveProcessedBlockReplayStoreSink {
    pub fn from_config(chain_id: u64, reth_datadir: PathBuf) -> eyre::Result<Self> {
        let cache_dir = processed_block_disk_cache_dir()?;
        let retain_blocks = processed_block_disk_cache_blocks();
        Self::new(
            cache_dir,
            chain_id,
            reth_datadir,
            retain_blocks,
            DEFAULT_QUEUE_BLOCKS,
        )
    }

    pub fn new(
        cache_dir: PathBuf,
        chain_id: u64,
        reth_datadir: PathBuf,
        retain_blocks: u64,
        queue_blocks: usize,
    ) -> eyre::Result<Self> {
        let writer =
            ProcessedBlockReplayStoreWriter::from_reth_datadir(&cache_dir, chain_id, reth_datadir)?;
        let (sender, receiver) = mpsc::channel(queue_blocks.max(1));
        let worker = tokio::spawn(run_replay_store_writer(
            writer,
            receiver,
            chain_id,
            retain_blocks,
        ));

        tracing::info!(
            cache_dir = %cache_dir.display(),
            retain_blocks,
            queue_blocks = queue_blocks.max(1),
            "enabled live processed block replay store writer"
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
                    "live processed block replay store queue is full; dropping replay-store write"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::warn!(
                    block_number,
                    "live processed block replay store writer is closed; dropping replay-store write"
                );
            }
        }
    }
}

async fn run_replay_store_writer(
    writer: ProcessedBlockReplayStoreWriter,
    mut receiver: mpsc::Receiver<ProcessedBlock>,
    chain_id: u64,
    retain_blocks: u64,
) {
    let mut successful_writes = 0_u64;
    while let Some(block) = receiver.recv().await {
        let block_number = block.header.number;
        let writer = writer.clone();
        let result =
            tokio::task::spawn_blocking(move || -> eyre::Result<ReplayStoreSinkWriteResult> {
                let Some(write) = writer.write_processed_block_if_missing(&block)? else {
                    return Ok(ReplayStoreSinkWriteResult {
                        write_ms: 0,
                        address_index_inserted: 0,
                        skipped_existing_cache: true,
                        pruned: 0,
                    });
                };
                let pruned = if (successful_writes + 1) % PRUNE_INTERVAL_WRITES == 0 {
                    writer.prune_disk_cache_to_recent_blocks(chain_id, retain_blocks)?
                } else {
                    0
                };
                Ok(ReplayStoreSinkWriteResult {
                    write_ms: write.total_write_ms(),
                    address_index_inserted: write
                        .address_block_index
                        .as_ref()
                        .map(|index| index.inserted)
                        .unwrap_or(0),
                    skipped_existing_cache: false,
                    pruned,
                })
            })
            .await;

        match result {
            Ok(Ok(write_result)) => {
                if write_result.skipped_existing_cache {
                    tracing::info!(
                        block_number,
                        "skipped live processed block replay-store write; disk cache already has block"
                    );
                    continue;
                }

                successful_writes += 1;
                tracing::info!(
                    block_number,
                    write_ms = write_result.write_ms,
                    address_index_inserted = write_result.address_index_inserted,
                    successful_writes,
                    "wrote live processed block replay store"
                );
                if write_result.pruned > 0 {
                    tracing::info!(
                        block_number,
                        pruned = write_result.pruned,
                        "pruned live processed block replay store"
                    );
                }
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    block_number,
                    error = %error,
                    "failed to write live processed block to replay store"
                );
            }
            Err(error) => {
                tracing::warn!(
                    block_number,
                    error = %error,
                    "live processed block replay store writer task failed"
                );
            }
        }
    }
}

struct ReplayStoreSinkWriteResult {
    write_ms: u128,
    address_index_inserted: usize,
    skipped_existing_cache: bool,
    pruned: usize,
}

fn processed_block_disk_cache_dir() -> eyre::Result<PathBuf> {
    if let Some(value) = non_empty_env(PROCESSED_BLOCK_DISK_CACHE_DIR_ENV) {
        return Ok(PathBuf::from(value));
    }

    let root = PathBuf::from(tx_simulator::config::repo::eth_node_root()?);
    Ok(root.join(PROCESSED_BLOCK_DISK_CACHE_DIR_NAME))
}

fn processed_block_disk_cache_blocks() -> u64 {
    non_empty_env(PROCESSED_BLOCK_DISK_CACHE_BLOCKS_ENV)
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
