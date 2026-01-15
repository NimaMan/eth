use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use alloy_primitives::B256;

use crate::block_processor::ProcessedBlock;
use crate::tx_processor::data_models::ProcessedTransaction;

/// Default number of blocks to retain in the cache (~2 days on Ethereum mainnet).
const BLOCK_RETENTION: u64 = 10_000;

/// In-memory cache of processed blocks keyed by block number.
#[derive(Default)]
pub struct ProcessedBlockCache {
    blocks: BTreeMap<u64, Arc<ProcessedBlock>>,
    transactions: HashMap<B256, ProcessedTransaction>,
}

impl ProcessedBlockCache {
    pub fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
            transactions: HashMap::new(),
        }
    }

    /// Insert a batch of processed blocks into the cache.
    pub fn insert_many(&mut self, blocks: Vec<ProcessedBlock>) {
        for block in blocks {
            self.insert(block);
        }
    }

    /// Insert a processed block into the cache.
    pub fn insert(&mut self, block: ProcessedBlock) {
        let number = block.header.number;
        let block_arc = Arc::new(block);

        if let Some(existing) = self.blocks.insert(number, block_arc.clone()) {
            // If we replaced an existing block, remove its transactions from the index.
            for tx in &existing.transactions {
                self.transactions.remove(&tx.processed.hash);
            }
        }

        for tx in &block_arc.transactions {
            self.transactions
                .insert(tx.processed.hash, tx.processed.clone());
        }

        self.prune_before(number.saturating_sub(BLOCK_RETENTION));
    }

    /// Returns the list of blocks not yet cached.
    pub fn missing_blocks(&self, block_numbers: &[u64]) -> Vec<u64> {
        block_numbers
            .iter()
            .copied()
            .filter(|number| !self.blocks.contains_key(number))
            .collect()
    }

    /// Retrieve cached block if present.
    pub fn get(&self, block_number: u64) -> Option<Arc<ProcessedBlock>> {
        self.blocks.get(&block_number).cloned()
    }

    /// Iterate over all cached blocks in numeric order.
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &Arc<ProcessedBlock>)> {
        self.blocks.iter()
    }

    /// Retrieve processed transaction by hash if present in cache.
    pub fn get_transaction(&self, tx_hash: &B256) -> Option<ProcessedTransaction> {
        self.transactions.get(tx_hash).cloned()
    }

    fn prune_before(&mut self, min_block: u64) {
        if self.blocks.is_empty() {
            return;
        }

        let to_remove: Vec<u64> = self
            .blocks
            .range(..min_block)
            .map(|(block_number, _)| *block_number)
            .collect();

        for block_number in to_remove {
            if let Some(block) = self.blocks.remove(&block_number) {
                for tx in &block.transactions {
                    self.transactions.remove(&tx.processed.hash);
                }
            }
        }
    }
}
