use crate::provider::RethProviderFactory;
use crate::reth_index::database::RethIndexDB;
use alloy_primitives::B256;
use eyre::Result;
use reth_provider::{BlockBodyIndicesProvider, BlockReader, TransactionsProvider};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tx_simulator::TxSimulator;

const RPC_RESOLVE_MIN_AGE_MS: u64 = 15_000;

/// High-level writer that resolves tx hashes to txumbers and writes arrival times (ms)
/// to the mempool_tx_arrival_times table in a single batch.
pub struct MempoolArrivalWriter {
    db: Arc<RethIndexDB>,
    provider_factory: Arc<RethProviderFactory>,
    fresh_provider_fallback_datadir: Option<PathBuf>,
    rpc_fallback_url: Option<String>,
}

impl MempoolArrivalWriter {
    pub fn new(db: Arc<RethIndexDB>, provider_factory: Arc<RethProviderFactory>) -> Self {
        Self {
            db,
            provider_factory,
            fresh_provider_fallback_datadir: None,
            rpc_fallback_url: None,
        }
    }

    /// Add a fresh-provider fallback for long-running read-only processes.
    ///
    /// The normal path uses the shared provider factory. If that resolves no
    /// hashes, the fallback briefly opens a fresh Reth provider from `reth_datadir`
    /// and retries the same batch before declaring all entries unresolved.
    pub fn with_fresh_provider_fallback(mut self, reth_datadir: impl Into<PathBuf>) -> Self {
        self.fresh_provider_fallback_datadir = Some(reth_datadir.into());
        self
    }

    pub fn with_rpc_fallback(mut self, rpc_url: impl Into<String>) -> Self {
        self.rpc_fallback_url = Some(rpc_url.into());
        self
    }

    pub fn new_with_fresh_provider_fallback(
        db: Arc<RethIndexDB>,
        provider_factory: Arc<RethProviderFactory>,
        reth_datadir: impl Into<PathBuf>,
    ) -> Self {
        Self::new(db, provider_factory).with_fresh_provider_fallback(reth_datadir)
    }

    pub fn new_with_fallbacks(
        db: Arc<RethIndexDB>,
        provider_factory: Arc<RethProviderFactory>,
        reth_datadir: impl Into<PathBuf>,
        rpc_url: impl Into<String>,
    ) -> Self {
        Self::new(db, provider_factory)
            .with_fresh_provider_fallback(reth_datadir)
            .with_rpc_fallback(rpc_url)
    }

    pub fn fresh_provider_fallback_datadir(&self) -> Option<&Path> {
        self.fresh_provider_fallback_datadir.as_deref()
    }

    fn resolve_entries(&self, entries: &[(B256, u64)]) -> Result<Vec<(B256, u64, u64)>> {
        let resolved =
            Self::resolve_entries_with_provider_factory(self.provider_factory.as_ref(), entries)?;
        if !resolved.is_empty() {
            return Ok(resolved);
        }

        let Some(reth_datadir) = self.fresh_provider_fallback_datadir.as_ref() else {
            return Ok(resolved);
        };

        let Some(reth_datadir) = reth_datadir.to_str() else {
            return Ok(resolved);
        };

        let fallback_resolved = match TxSimulator::new(reth_datadir) {
            Ok(simulator) => {
                let fresh_provider_factory = simulator.provider_factory().clone();
                Self::resolve_entries_with_provider_factory(&fresh_provider_factory, entries)?
            }
            Err(error) => {
                tracing::debug!(
                    error = %error,
                    "mempool arrival writer fresh provider fallback unavailable"
                );
                Vec::new()
            }
        };
        if !fallback_resolved.is_empty() {
            tracing::debug!(
                primary_resolved = 0,
                fallback_resolved = fallback_resolved.len(),
                "mempool arrival writer resolved hashes through fresh provider fallback"
            );
            return Ok(fallback_resolved);
        }

        let Some(rpc_url) = self.rpc_fallback_url.as_ref() else {
            return Ok(fallback_resolved);
        };

        let rpc_candidates = aged_entries(entries, RPC_RESOLVE_MIN_AGE_MS);
        if rpc_candidates.is_empty() {
            return Ok(fallback_resolved);
        }

        match Self::resolve_entries_with_rpc(
            self.provider_factory.as_ref(),
            rpc_url,
            &rpc_candidates,
        ) {
            Ok(rpc_resolved) => {
                if !rpc_resolved.is_empty() {
                    tracing::debug!(
                        rpc_resolved = rpc_resolved.len(),
                        "mempool arrival writer resolved hashes through RPC fallback"
                    );
                }
                Ok(rpc_resolved)
            }
            Err(error) => {
                tracing::debug!(
                    error = %error,
                    "mempool arrival writer RPC fallback unavailable"
                );
                Ok(fallback_resolved)
            }
        }
    }

    fn resolve_entries_with_provider_factory(
        provider_factory: &RethProviderFactory,
        entries: &[(B256, u64)],
    ) -> Result<Vec<(B256, u64, u64)>> {
        provider_factory.caught_up_static_file_provider()?;
        let provider = provider_factory.provider()?;
        let mut resolved: Vec<(B256, u64, u64)> = Vec::with_capacity(entries.len());
        for (hash, first_seen_ms) in entries.iter().copied() {
            match Self::resolve_tx_number(provider.as_ref(), hash)? {
                Some(tx_number) => resolved.push((hash, tx_number, first_seen_ms)),
                None => { /* not mined yet */ }
            }
        }
        Ok(resolved)
    }

    fn resolve_entries_with_rpc(
        provider_factory: &RethProviderFactory,
        rpc_url: &str,
        entries: &[(B256, u64)],
    ) -> Result<Vec<(B256, u64, u64)>> {
        provider_factory.caught_up_static_file_provider()?;
        let provider = provider_factory.provider()?;
        let client = reqwest::blocking::Client::new();
        let request = entries
            .iter()
            .enumerate()
            .map(|(idx, (hash, _))| {
                json!({
                    "jsonrpc": "2.0",
                    "id": idx,
                    "method": "eth_getTransactionByHash",
                    "params": [format!("0x{hash:x}")],
                })
            })
            .collect::<Vec<_>>();

        let response = client
            .post(rpc_url)
            .json(&request)
            .send()?
            .error_for_status()?;
        let payload: Value = response.json()?;
        let Some(items) = payload.as_array() else {
            return Ok(Vec::new());
        };

        let mut resolved = Vec::new();
        for item in items {
            let Some(id) = item.get("id").and_then(Value::as_u64).map(|id| id as usize) else {
                continue;
            };
            let Some((hash, first_seen_ms)) = entries.get(id).copied() else {
                continue;
            };
            let Some(result) = item.get("result").filter(|value| !value.is_null()) else {
                continue;
            };
            let Some(block_number) = result
                .get("blockNumber")
                .and_then(Value::as_str)
                .and_then(parse_hex_u64)
            else {
                continue;
            };
            let Some(tx_index) = result
                .get("transactionIndex")
                .and_then(Value::as_str)
                .and_then(parse_hex_u64)
            else {
                continue;
            };
            let Some(indices) = provider.block_body_indices(block_number)? else {
                continue;
            };
            resolved.push((hash, indices.first_tx_num + tx_index, first_seen_ms));
        }

        Ok(resolved)
    }

    /// Resolve a batch of (hash, first_seen_ms) to (tx_number, first_seen_ms) and write in one tx.
    /// Returns number of entries written.
    pub fn write_arrivals_by_hashes_ms(&self, entries: &[(B256, u64)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }
        let resolved = self.resolve_entries(entries)?;
        let resolved_pairs = resolved
            .into_iter()
            .map(|(_, tx_number, first_seen_ms)| (tx_number, first_seen_ms))
            .collect::<Vec<_>>();
        self.db.put_tx_arrivals_ms(&resolved_pairs)
    }

    /// Resolve and write arrivals, returning the subset of hashes that were resolved and written.
    pub fn write_arrivals_by_hashes_ms_return_resolved(
        &self,
        entries: &[(B256, u64)],
    ) -> Result<Vec<B256>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        let resolved = self.resolve_entries(entries)?;
        let mut resolved_pairs: Vec<(u64, u64)> = Vec::with_capacity(resolved.len());
        let mut resolved_hashes: Vec<B256> = Vec::with_capacity(resolved.len());
        for (hash, tx_number, first_seen_ms) in resolved {
            resolved_pairs.push((tx_number, first_seen_ms));
            resolved_hashes.push(hash);
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

fn aged_entries(entries: &[(B256, u64)], min_age_ms: u64) -> Vec<(B256, u64)> {
    let now_ms = now_ms();
    entries
        .iter()
        .copied()
        .filter(|(_, first_seen_ms)| now_ms.saturating_sub(*first_seen_ms) >= min_age_ms)
        .collect()
}

fn now_ms() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs()
        .saturating_mul(1000)
        .saturating_add(u64::from(now.subsec_millis()))
}

fn parse_hex_u64(value: &str) -> Option<u64> {
    u64::from_str_radix(value.trim_start_matches("0x"), 16).ok()
}
