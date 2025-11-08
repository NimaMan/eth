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

        self.flush_pending_blocks()?;
        let inserted = match self.process_block(block_number, participations_vec)? {
            Some(inserted) => inserted,
            None => 0,
        };
        self.flush_pending_blocks()?;
        Ok(inserted)
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

    fn flush_pending_blocks(&self) -> Result<()> {
        loop {
            let mut to_process = Vec::new();
            {
                let mut cache = self.pending_blocks.lock().expect("pending lock");
                let keys: Vec<u64> = cache.iter().map(|(&k, _)| k).collect();
                for block in keys {
                    if self.provider.get_block_tx_indices(block).is_ok() {
                        if let Some(entry) = cache.pop(&block) {
                            to_process.push((block, entry));
                        }
                    }
                }
            }

            if to_process.is_empty() {
                break;
            }

            for (block, participations) in to_process {
                let _ = self.process_block(block, participations)?;
            }
        }
        Ok(())
    }

    fn process_block(
        &self,
        block_number: u64,
        participations_vec: Vec<AddressParticipation>,
    ) -> Result<Option<usize>> {
        if participations_vec.is_empty() {
            return Ok(Some(0));
        }

        let indices = match self.provider.get_block_tx_indices(block_number) {
            Ok(indices) => indices,
            Err(_) => {
                warn!(
                    block_number,
                    "Deferring address index write until transaction indices are available"
                );
                self.store_pending(block_number, participations_vec);
                self.flush_pending_blocks()?;
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
            return Ok(Some(0));
        }

        let mut entries: Vec<(Address, Vec<Txumber>)> = map.into_iter().collect();
        entries.sort_by_key(|(address, _)| *address);

        for (_, txs) in entries.iter_mut() {
            txs.sort_unstable();
            txs.dedup();
        }
        let inserted = self.db.append_address_transactions_batch(&entries)?;

        Ok(Some(inserted))
    }
}
