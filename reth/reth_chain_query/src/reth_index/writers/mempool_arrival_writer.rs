use std::sync::Arc;
use eyre::Result;
use alloy_primitives::B256;
use reth_provider::{ProviderFactory, TransactionsProvider};
use reth_node_types::NodeTypesWithDBAdapter;
use reth_node_ethereum::EthereumNode;
use reth_db::DatabaseEnv;
use crate::reth_index::database::RethIndexDB;

/// High-level writer that resolves tx hashes to TxNumbers and writes arrival times (ms)
/// to the mempool_tx_arrival_times table in a single batch.
pub struct MempoolArrivalWriter {
    db: Arc<RethIndexDB>,
    provider_factory: Arc<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>>,
}

impl MempoolArrivalWriter {
    pub fn new(
        db: Arc<RethIndexDB>,
        provider_factory: Arc<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>>,
    ) -> Self {
        Self { db, provider_factory }
    }

    /// Resolve a batch of (hash, first_seen_ms) to (tx_number, first_seen_ms) and write in one txn.
    /// Returns number of entries written.
    pub fn write_arrivals_by_hashes_ms(&self, entries: &[(B256, u64)]) -> Result<usize> {
        if entries.is_empty() { return Ok(0); }
        let provider = self.provider_factory.provider()?;
        let mut resolved: Vec<(u64, u64)> = Vec::with_capacity(entries.len());
        for (hash, first_seen_ms) in entries.iter().copied() {
            match provider.transaction_id(hash)? {
                Some(tx_id) => resolved.push((tx_id as u64, first_seen_ms)),
                None => { /* not mined yet */ }
            }
        }
        self.db.put_tx_arrivals_ms(&resolved)
    }

    /// Resolve and write arrivals, returning the subset of hashes that were resolved and written.
    pub fn write_arrivals_by_hashes_ms_return_resolved(&self, entries: &[(B256, u64)]) -> Result<Vec<B256>> {
        if entries.is_empty() { return Ok(Vec::new()); }
        let provider = self.provider_factory.provider()?;
        let mut resolved_pairs: Vec<(u64, u64)> = Vec::with_capacity(entries.len());
        let mut resolved_hashes: Vec<B256> = Vec::new();
        for (hash, first_seen_ms) in entries.iter().copied() {
            match provider.transaction_id(hash)? {
                Some(tx_id) => {
                    resolved_pairs.push((tx_id as u64, first_seen_ms));
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
}
