use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use alloy_primitives::Address;
use eyre::Result;
use lru::LruCache;
use tracing::warn;

use crate::reth_index::tables::address_index::Txumber;
use crate::reth_index::RethIndexDB;
use crate::RethQueryProvider;

/// Addresses touched by a transaction within a block.
pub struct AddressParticipation {
    pub tx_index: u64,
    pub addresses: Vec<Address>,
}

/// Writer that persists address → transaction mappings into the RethIndex DB.
pub struct AddressTxWriter {
    db: Arc<RethIndexDB>,
    provider: Arc<RethQueryProvider>,
    pending_blocks: Mutex<LruCache<u64, Vec<AddressParticipation>>>,
}

struct BlockEntries {
    entries: Vec<(Address, Vec<Txumber>)>,
}

impl AddressTxWriter {
    pub fn new(db: Arc<RethIndexDB>, provider: Arc<RethQueryProvider>) -> Self {
        Self {
            db,
            provider,
            pending_blocks: Mutex::new(LruCache::new(
                NonZeroUsize::new(100).expect("pending cache size must be non-zero"),
            )),
        }
    }

    /// Persist participations for a single block.
    pub fn ingest_block_participation<I>(
        &self,
        block_number: u64,
        participations: I,
    ) -> Result<usize>
    where
        I: IntoIterator<Item = AddressParticipation>,
    {
        let participations_vec: Vec<AddressParticipation> = participations.into_iter().collect();
        if participations_vec.is_empty() {
            return Ok(0);
        }

        let mut blocks = Vec::with_capacity(1);
        blocks.push((block_number, participations_vec));
        let mut inserted = self.ingest_block_batch(blocks)?;
        Ok(inserted.pop().unwrap_or(0))
    }

    /// Persist participations for multiple blocks in one MDBX transaction.
    pub fn ingest_block_batch(
        &self,
        blocks: Vec<(u64, Vec<AddressParticipation>)>,
    ) -> Result<Vec<usize>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let mut ready_batches = self.drain_ready_pending_blocks()?;
        let mut per_block = Vec::with_capacity(blocks.len());
        let mut block_batch_indices: Vec<(usize, usize)> = Vec::new();

        for (idx, (block_number, participations_vec)) in blocks.into_iter().enumerate() {
            per_block.push(0);

            if participations_vec.is_empty() {
                continue;
            }

            match self.prepare_block_entries(block_number, participations_vec)? {
                Some(entries) => {
                    if entries.entries.is_empty() {
                        continue;
                    } else {
                        let batch_index = ready_batches.len();
                        block_batch_indices.push((idx, batch_index));
                        ready_batches.push(entries);
                    }
                }
                None => {
                    continue;
                }
            }
        }

        if ready_batches.is_empty() {
            return Ok(per_block);
        }

        let inserted_counts = self.write_block_entries_batch(ready_batches)?;
        for (block_idx, batch_idx) in block_batch_indices {
            if let Some(inserted) = inserted_counts.get(batch_idx) {
                if let Some(target) = per_block.get_mut(block_idx) {
                    *target = *inserted;
                }
            }
        }

        Ok(per_block)
    }

    fn store_pending(&self, block_number: u64, participations: Vec<AddressParticipation>) {
        let mut cache = self.pending_blocks.lock().expect("pending lock");
        if let Some((evicted_block, _)) = cache.push(block_number, participations) {
            warn!(
                evicted_block,
                "Dropping oldest pending block from cache due to capacity limit"
            );
        }
    }

    fn prepare_block_entries(
        &self,
        block_number: u64,
        participations_vec: Vec<AddressParticipation>,
    ) -> Result<Option<BlockEntries>> {
        if participations_vec.is_empty() {
            return Ok(Some(BlockEntries {
                entries: Vec::new(),
            }));
        }

        let indices = match self.provider.get_block_tx_indices(block_number) {
            Ok(indices) => indices,
            Err(_) => {
                warn!(
                    block_number,
                    "Deferring address index write until transaction indices are available"
                );
                self.store_pending(block_number, participations_vec);
                return Ok(None);
            }
        };

        let first_tx_num = indices.first_tx_num;

        let mut map: HashMap<Address, Vec<Txumber>> = HashMap::new();

        for participation in participations_vec {
            let AddressParticipation {
                tx_index,
                addresses,
            } = participation;
            if addresses.is_empty() {
                continue;
            }

            let tx_number = first_tx_num + tx_index;
            let mut dedup = HashSet::with_capacity(addresses.len());

            for address in addresses {
                if dedup.insert(address) {
                    map.entry(address).or_default().push(tx_number);
                }
            }
        }

        if map.is_empty() {
            return Ok(Some(BlockEntries {
                entries: Vec::new(),
            }));
        }

        let mut entries: Vec<(Address, Vec<Txumber>)> = map.into_iter().collect();
        entries.sort_by_key(|(address, _)| *address);

        for (_, txs) in entries.iter_mut() {
            txs.sort_unstable();
            txs.dedup();
        }

        Ok(Some(BlockEntries { entries }))
    }

    fn drain_ready_pending_blocks(&self) -> Result<Vec<BlockEntries>> {
        let mut ready_blocks = Vec::new();
        {
            let mut cache = self.pending_blocks.lock().expect("pending lock");
            let mut keys: Vec<u64> = cache.iter().map(|(&k, _)| k).collect();
            keys.sort_unstable();
            for block in keys {
                if self.provider.get_block_tx_indices(block).is_ok() {
                    if let Some(entry) = cache.pop(&block) {
                        ready_blocks.push((block, entry));
                    }
                }
            }
        }

        let mut prepared = Vec::new();
        for (block, participations) in ready_blocks {
            if let Some(entries) = self.prepare_block_entries(block, participations)? {
                if !entries.entries.is_empty() {
                    prepared.push(entries);
                }
            }
        }
        Ok(prepared)
    }

    fn write_block_entries_batch(&self, blocks: Vec<BlockEntries>) -> Result<Vec<usize>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let block_refs: Vec<&[(Address, Vec<Txumber>)]> = blocks
            .iter()
            .map(|block| block.entries.as_slice())
            .collect();

        self.db.append_block_entry_slices(&block_refs)
    }
}
