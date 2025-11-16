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
    provider: Arc<RethQueryProvider>,
    rpc_fetcher: Option<RpcBlockDataFetcher>,
}

impl BlockDataFetcher {
    /// Build a new fetcher backed by the local MDBX database.
    pub fn new(provider: Arc<RethQueryProvider>) -> Self {
        Self {
            provider,
            rpc_fetcher: None,
        }
    }

    /// Attach an RPC fetcher. Callers should configure this before sharing
    /// the fetcher (e.g. before wrapping it inside an `Arc`).
    pub fn with_rpc_fetcher(mut self, rpc_fetcher: RpcBlockDataFetcher) -> Self {
        self.rpc_fetcher = Some(rpc_fetcher);
        self
    }

    /// Access the underlying provider handle.
    pub fn provider(&self) -> &Arc<RethQueryProvider> {
        &self.provider
    }

    /// Access the RPC fetcher if configured.
    pub fn rpc_fetcher(&self) -> Option<&RpcBlockDataFetcher> {
        self.rpc_fetcher.as_ref()
    }

    /// Fetch block data directly from MDBX.
    pub async fn fetch_db_block(
        &self,
        block_number: u64,
        include_traces: bool,
    ) -> Result<RawBlockData> {
        self.provider
            .fetch_raw_block_data(block_number, include_traces)
            .await
    }

    /// Fetch block data from RPC using a block hash (and number for logging).
    pub async fn fetch_rpc_block_by_hash(
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
