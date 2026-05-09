use std::sync::Arc;

use alloy_primitives::B256;
use alloy_rpc_types_trace::geth::PreStateFrame;
use eyre::Result;
use tx_simulator::block_simulation::BlockTraceEngine;

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
        self.fetch_db_block_with_trace_engine(
            block_number,
            include_traces,
            BlockTraceEngine::default(),
        )
        .await
    }

    /// Fetch block data directly from MDBX using an explicit local trace engine.
    pub async fn fetch_db_block_with_trace_engine(
        &self,
        block_number: u64,
        include_traces: bool,
        trace_engine: BlockTraceEngine,
    ) -> Result<RawBlockData> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| eyre::eyre!("database access not configured for this fetcher"))?;
        provider
            .fetch_raw_block_data_with_trace_engine(block_number, include_traces, trace_engine)
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

    /// Return the latest execution block number from the configured RPC.
    pub async fn latest_rpc_block_number(&self) -> Result<u64> {
        let rpc = self
            .rpc_fetcher
            .as_ref()
            .ok_or_else(|| eyre::eyre!("RPC block fetcher not configured"))?;
        rpc.latest_block_number().await
    }

    /// Fetch block data from RPC by block number.
    pub async fn fetch_rpc_block_by_number_with_traces(
        &self,
        block_number: u64,
        include_traces: bool,
    ) -> Result<RawBlockData> {
        let rpc = self
            .rpc_fetcher
            .as_ref()
            .ok_or_else(|| eyre::eyre!("RPC block fetcher not configured"))?;
        rpc.fetch_raw_block_by_number_data(block_number, include_traces)
            .await
    }

    /// Fetch exact per-transaction post-state diffs for a block from RPC.
    pub async fn fetch_rpc_state_diffs_by_number(
        &self,
        block_number: u64,
    ) -> Result<Vec<PreStateFrame>> {
        let rpc = self
            .rpc_fetcher
            .as_ref()
            .ok_or_else(|| eyre::eyre!("RPC block fetcher not configured"))?;
        rpc.trace_block_state_diffs_by_number(block_number).await
    }
}
