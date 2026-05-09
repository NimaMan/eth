use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use reth_chain_query::reth_index::{AddressBlockParticipationWriter, RethIndexDB};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::address_participations_from_processed_block;
use crate::ProcessedBlock;

const DEFAULT_QUEUE_BLOCKS: usize = 256;

/// Non-blocking background worker for the persistent address -> block
/// participation index.
pub struct LiveAddressBlockParticipationIndexWorker {
    sender: mpsc::Sender<ProcessedBlock>,
    _worker: JoinHandle<()>,
}

impl LiveAddressBlockParticipationIndexWorker {
    pub fn from_reth_datadir(reth_datadir: impl AsRef<Path>) -> eyre::Result<Self> {
        let index_dir = reth_index_dir(reth_datadir.as_ref());
        Self::new(index_dir, DEFAULT_QUEUE_BLOCKS)
    }

    pub fn new(index_dir: PathBuf, queue_blocks: usize) -> eyre::Result<Self> {
        let db = Arc::new(RethIndexDB::open(&index_dir)?);
        let writer = AddressBlockParticipationWriter::new(db);
        let (sender, receiver) = mpsc::channel(queue_blocks.max(1));
        let worker = tokio::spawn(run_address_participation_index_writer(writer, receiver));

        tracing::info!(
            index_dir = %index_dir.display(),
            queue_blocks = queue_blocks.max(1),
            "enabled live address block participation index worker"
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
                    "live address block participation index queue is full; dropping index write"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::warn!(
                    block_number,
                    "live address block participation index worker is closed; dropping index write"
                );
            }
        }
    }
}

async fn run_address_participation_index_writer(
    writer: AddressBlockParticipationWriter,
    mut receiver: mpsc::Receiver<ProcessedBlock>,
) {
    let mut indexed_blocks = 0_u64;
    let mut failed_blocks = 0_u64;

    while let Some(block) = receiver.recv().await {
        let block_number = block.header.number;
        let tx_count = block.transactions.len();
        let writer = writer.clone();

        let result =
            tokio::task::spawn_blocking(move || -> eyre::Result<AddressIndexWriteStats> {
                let participations = address_participations_from_processed_block(&block);
                let participating_txs = participations.len();
                let started = Instant::now();
                let inserted = writer.ingest_block_participation(block_number, participations)?;
                Ok(AddressIndexWriteStats {
                    participating_txs,
                    inserted,
                    write_ms: started.elapsed().as_millis(),
                })
            })
            .await;

        match result {
            Ok(Ok(stats)) => {
                indexed_blocks += 1;
                tracing::info!(
                    block_number,
                    tx_count,
                    participating_txs = stats.participating_txs,
                    inserted = stats.inserted,
                    write_ms = stats.write_ms,
                    indexed_blocks,
                    "indexed live address block participation"
                );
            }
            Ok(Err(error)) => {
                failed_blocks += 1;
                tracing::warn!(
                    block_number,
                    tx_count,
                    failed_blocks,
                    error = %error,
                    "failed to index live address block participation"
                );
            }
            Err(error) => {
                failed_blocks += 1;
                tracing::warn!(
                    block_number,
                    tx_count,
                    failed_blocks,
                    error = %error,
                    "live address block participation index task failed"
                );
            }
        }
    }
}

struct AddressIndexWriteStats {
    participating_txs: usize,
    inserted: usize,
    write_ms: u128,
}

fn reth_index_dir(reth_datadir: &Path) -> PathBuf {
    reth_datadir.join("reth_index")
}
