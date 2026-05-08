use crate::provider::RethProviderFactory;
use crate::reth_index::database::RethIndexDB;
use alloy_primitives::B256;
use eyre::Result;
use reth_provider::{BlockReader, TransactionsProvider};
use std::sync::Arc;

/// High-level writer that resolves tx hashes to txumbers and writes arrival times (ms)
/// to the mempool_tx_arrival_times table in a single batch.
pub struct MempoolArrivalWriter {
    db: Arc<RethIndexDB>,
    provider_factory: Arc<RethProviderFactory>,
}

impl MempoolArrivalWriter {
    pub fn new(db: Arc<RethIndexDB>, provider_factory: Arc<RethProviderFactory>) -> Self {
        Self {
            db,
            provider_factory,
        }
    }

    /// Resolve a batch of (hash, first_seen_ms) to (tx_number, first_seen_ms) and write in one tx.
    /// Returns number of entries written.
    pub fn write_arrivals_by_hashes_ms(&self, entries: &[(B256, u64)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }
        self.provider_factory.caught_up_static_file_provider()?;
        let provider = self.provider_factory.provider()?;
        let mut resolved: Vec<(u64, u64)> = Vec::with_capacity(entries.len());
        for (hash, first_seen_ms) in entries.iter().copied() {
            match Self::resolve_tx_number(provider.as_ref(), hash)? {
                Some(tx_number) => resolved.push((tx_number, first_seen_ms)),
                None => { /* not mined yet */ }
            }
        }
        self.db.put_tx_arrivals_ms(&resolved)
    }

    /// Resolve and write arrivals, returning the subset of hashes that were resolved and written.
    pub fn write_arrivals_by_hashes_ms_return_resolved(
        &self,
        entries: &[(B256, u64)],
    ) -> Result<Vec<B256>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        self.provider_factory.caught_up_static_file_provider()?;
        let provider = self.provider_factory.provider()?;
        let mut resolved_pairs: Vec<(u64, u64)> = Vec::with_capacity(entries.len());
        let mut resolved_hashes: Vec<B256> = Vec::new();
        for (hash, first_seen_ms) in entries.iter().copied() {
            match Self::resolve_tx_number(provider.as_ref(), hash)? {
                Some(tx_number) => {
                    resolved_pairs.push((tx_number, first_seen_ms));
                    resolved_hashes.push(hash);
                }
                None => { /* not mined yet */ }
            }
        }
        if resolved_pairs.is_empty() {
            return Ok(Vec::new());
        }
        let _ = self.db.put_tx_arrivals_ms(&resolved_pairs)?;
        Ok(resolved_hashes)
    }

    fn resolve_tx_number<Provider>(provider: &Provider, hash: B256) -> Result<Option<u64>>
    where
        Provider: TransactionsProvider + BlockReader,
    {
        if let Some(tx_id) = provider.transaction_id(hash)? {
            return Ok(Some(tx_id as u64));
        }

        let Some((_tx, meta)) = provider.transaction_by_hash_with_meta(hash)? else {
            return Ok(None);
        };
        let Some(indices) = provider.block_body_indices(meta.block_number)? else {
            return Ok(None);
        };
        Ok(Some(indices.first_tx_num + meta.index as u64))
    }
}
