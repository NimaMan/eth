use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use eyre::Result;
use reth_chain_query::reth_index::{AddressBlockParticipationWriter, RethIndexDB};

use crate::{
    address_participations_from_processed_block, ProcessedBlock, ProcessedBlockDiskCacheStore,
    ProcessedBlockDiskCacheWrite, ProcessedBlockDiskCacheWriter,
};

#[derive(Clone)]
pub struct ProcessedBlockReplayStoreWriter {
    disk_cache_store: ProcessedBlockDiskCacheStore,
    disk_cache_writer: ProcessedBlockDiskCacheWriter,
    chain_id: u64,
    address_block_index: Option<AddressBlockParticipationWriter>,
}

#[derive(Debug, Clone)]
pub struct ProcessedBlockReplayStoreWrite {
    pub disk_cache: ProcessedBlockDiskCacheWrite,
    pub address_block_index: Option<ProcessedBlockAddressIndexWrite>,
    pub address_block_index_error: Option<ProcessedBlockAddressIndexError>,
}

#[derive(Debug, Clone)]
pub struct ProcessedBlockAddressIndexWrite {
    pub participating_txs: usize,
    pub inserted: usize,
    pub write_ms: u128,
}

#[derive(Debug, Clone)]
pub struct ProcessedBlockAddressIndexError {
    pub participating_txs: usize,
    pub write_ms: u128,
    pub error: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessedBlockAddressIndexFailurePolicy {
    Strict,
    BestEffort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessedBlockReplayStoreWriteOptions {
    pub address_index_failure_policy: ProcessedBlockAddressIndexFailurePolicy,
}

impl Default for ProcessedBlockReplayStoreWriteOptions {
    fn default() -> Self {
        Self {
            address_index_failure_policy: ProcessedBlockAddressIndexFailurePolicy::Strict,
        }
    }
}

impl ProcessedBlockReplayStoreWriteOptions {
    pub fn best_effort_address_index() -> Self {
        Self {
            address_index_failure_policy: ProcessedBlockAddressIndexFailurePolicy::BestEffort,
        }
    }

    pub fn with_address_index_failure_policy(
        mut self,
        policy: ProcessedBlockAddressIndexFailurePolicy,
    ) -> Self {
        self.address_index_failure_policy = policy;
        self
    }
}

impl ProcessedBlockReplayStoreWriter {
    pub fn new(
        disk_cache_store: ProcessedBlockDiskCacheStore,
        chain_id: u64,
        address_block_index: Option<AddressBlockParticipationWriter>,
    ) -> Self {
        let disk_cache_writer = disk_cache_store.writer(chain_id);
        Self {
            disk_cache_store,
            disk_cache_writer,
            chain_id,
            address_block_index,
        }
    }

    pub fn from_dirs(
        disk_cache_dir: impl AsRef<Path>,
        chain_id: u64,
        reth_index_dir: Option<&Path>,
    ) -> Result<Self> {
        let disk_cache_store = ProcessedBlockDiskCacheStore::open(disk_cache_dir)?;
        let address_block_index = match reth_index_dir {
            Some(path) => {
                let db = Arc::new(RethIndexDB::open(path)?);
                Some(AddressBlockParticipationWriter::new(db))
            }
            None => None,
        };
        Ok(Self::new(disk_cache_store, chain_id, address_block_index))
    }

    pub fn from_reth_datadir(
        disk_cache_dir: impl AsRef<Path>,
        chain_id: u64,
        reth_datadir: impl AsRef<Path>,
    ) -> Result<Self> {
        let reth_index_dir = reth_datadir.as_ref().join("reth_index");
        Self::from_dirs(disk_cache_dir, chain_id, Some(reth_index_dir.as_path()))
    }

    pub fn disk_cache_store(&self) -> &ProcessedBlockDiskCacheStore {
        &self.disk_cache_store
    }

    pub fn write_processed_block(
        &self,
        block: &ProcessedBlock,
    ) -> Result<ProcessedBlockReplayStoreWrite> {
        self.write_processed_block_with_options(
            block,
            ProcessedBlockReplayStoreWriteOptions::default(),
        )
    }

    pub fn write_processed_block_with_options(
        &self,
        block: &ProcessedBlock,
        options: ProcessedBlockReplayStoreWriteOptions,
    ) -> Result<ProcessedBlockReplayStoreWrite> {
        let disk_cache = self.disk_cache_writer.write_processed_block(block)?;
        let (address_block_index, address_block_index_error) =
            self.index_processed_block_with_options(block, options)?;
        Ok(ProcessedBlockReplayStoreWrite {
            disk_cache,
            address_block_index,
            address_block_index_error,
        })
    }

    pub fn write_processed_block_if_missing(
        &self,
        block: &ProcessedBlock,
    ) -> Result<Option<ProcessedBlockReplayStoreWrite>> {
        self.write_processed_block_if_missing_with_options(
            block,
            ProcessedBlockReplayStoreWriteOptions::default(),
        )
    }

    pub fn write_processed_block_if_missing_with_options(
        &self,
        block: &ProcessedBlock,
        options: ProcessedBlockReplayStoreWriteOptions,
    ) -> Result<Option<ProcessedBlockReplayStoreWrite>> {
        let key = self.disk_cache_store.key_for_block(self.chain_id, block);
        if self.disk_cache_store.contains(&key) {
            return Ok(None);
        }

        self.write_processed_block_with_options(block, options)
            .map(Some)
    }

    pub fn index_processed_block(
        &self,
        block: &ProcessedBlock,
    ) -> Result<Option<ProcessedBlockAddressIndexWrite>> {
        let Some(index_writer) = &self.address_block_index else {
            return Ok(None);
        };

        let participations = address_participations_from_processed_block(block);
        let participating_txs = participations.len();
        let started = Instant::now();
        let inserted =
            index_writer.ingest_block_participation(block.header.number, participations)?;
        Ok(Some(ProcessedBlockAddressIndexWrite {
            participating_txs,
            inserted,
            write_ms: started.elapsed().as_millis(),
        }))
    }

    fn index_processed_block_with_options(
        &self,
        block: &ProcessedBlock,
        options: ProcessedBlockReplayStoreWriteOptions,
    ) -> Result<(
        Option<ProcessedBlockAddressIndexWrite>,
        Option<ProcessedBlockAddressIndexError>,
    )> {
        let Some(index_writer) = &self.address_block_index else {
            return Ok((None, None));
        };

        let participations = address_participations_from_processed_block(block);
        let participating_txs = participations.len();
        let started = Instant::now();
        match index_writer.ingest_block_participation(block.header.number, participations) {
            Ok(inserted) => Ok((
                Some(ProcessedBlockAddressIndexWrite {
                    participating_txs,
                    inserted,
                    write_ms: started.elapsed().as_millis(),
                }),
                None,
            )),
            Err(error)
                if options.address_index_failure_policy
                    == ProcessedBlockAddressIndexFailurePolicy::BestEffort =>
            {
                let write_ms = started.elapsed().as_millis();
                tracing::warn!(
                    block_number = block.header.number,
                    participating_txs,
                    write_ms,
                    error = %error,
                    "processed block address index write failed; continuing with disk cache entry"
                );
                Ok((
                    None,
                    Some(ProcessedBlockAddressIndexError {
                        participating_txs,
                        write_ms,
                        error: error.to_string(),
                    }),
                ))
            }
            Err(error) => Err(error),
        }
    }

    pub fn prune_disk_cache_to_recent_blocks(
        &self,
        chain_id: u64,
        retain_blocks: u64,
    ) -> Result<usize> {
        self.disk_cache_store
            .prune_chain_to_recent_blocks(chain_id, retain_blocks)
    }
}

impl ProcessedBlockReplayStoreWrite {
    pub fn total_write_ms(&self) -> u128 {
        self.disk_cache.write_ms
            + self
                .address_block_index
                .as_ref()
                .map(|write| write.write_ms)
                .unwrap_or(0)
            + self
                .address_block_index_error
                .as_ref()
                .map(|write| write.write_ms)
                .unwrap_or(0)
    }
}
