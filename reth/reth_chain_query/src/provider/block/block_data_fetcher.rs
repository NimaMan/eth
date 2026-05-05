use std::sync::Arc;

use alloy_primitives::B256;
use eyre::Result;

use crate::provider::{
    block::{rpc_fetcher::RpcBlockDataFetcher, RawBlockData},
    RethQueryProvider,
};

/// Unified entry-point for fetching raw block data from either MDBX or RPC.
///
/// Higher-level processors can hold a single [`BlockDataFetcher`] and decide
/// which backing source (database vs RPC) they want to use for each block.
pub struct BlockDataFetcher {
    provider: Option<Arc<RethQueryProvider>>,
    rpc_fetcher: Option<RpcBlockDataFetcher>,
}

impl BlockDataFetcher {
    /// Build a new fetcher backed by the local MDBX database.
    pub fn new(provider: Arc<RethQueryProvider>) -> Self {
        Self {
            provider: Some(provider),
            rpc_fetcher: None,
        }
    }

    /// Build a fetcher that only talks to RPC (no MDBX access).
    pub fn rpc_only(rpc_fetcher: RpcBlockDataFetcher) -> Self {
        Self {
            provider: None,
            rpc_fetcher: Some(rpc_fetcher),
        }
    }

    /// Attach an RPC fetcher. Callers should configure this before sharing
    /// the fetcher (e.g. before wrapping it inside an `Arc`).
    pub fn with_rpc_fetcher(mut self, rpc_fetcher: RpcBlockDataFetcher) -> Self {
        self.rpc_fetcher = Some(rpc_fetcher);
        self
    }

    /// Access the underlying provider handle.
    pub fn provider(&self) -> Option<&Arc<RethQueryProvider>> {
        self.provider.as_ref()
    }

    /// Access the RPC fetcher if configured.
    pub fn rpc_fetcher(&self) -> Option<&RpcBlockDataFetcher> {
        self.rpc_fetcher.as_ref()
    }

    /// Fetch block data directly from MDBX.
    pub async fn fetch_db_block(&self, block_number: u64) -> Result<RawBlockData> {
        self.fetch_db_block_with_traces(block_number, true).await
    }

    /// Fetch block data directly from MDBX, optionally including simulated traces.
    pub async fn fetch_db_block_with_traces(
        &self,
        block_number: u64,
        include_traces: bool,
    ) -> Result<RawBlockData> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| eyre::eyre!("database access not configured for this fetcher"))?;
        provider
            .fetch_raw_block_data(block_number, include_traces)
            .await
    }

    /// Fetch block data from RPC using a block hash (and number for logging).
    pub async fn fetch_rpc_block_by_hash(
        &self,
        block_hash: B256,
        block_number: u64,
    ) -> Result<RawBlockData> {
        self.fetch_rpc_block_by_hash_with_traces(block_hash, block_number, true)
            .await
    }

    /// Fetch block data from RPC using a block hash, optionally including call traces.
    pub async fn fetch_rpc_block_by_hash_with_traces(
        &self,
        block_hash: B256,
        block_number: u64,
        include_traces: bool,
    ) -> Result<RawBlockData> {
        let rpc = self
            .rpc_fetcher
            .as_ref()
            .ok_or_else(|| eyre::eyre!("RPC block fetcher not configured"))?;
        rpc.fetch_raw_block_data(block_hash, block_number, include_traces)
            .await
    }
}
