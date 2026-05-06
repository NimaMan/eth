//! Block tracer implementation equivalent to `debug_traceBlockByNumber`.
//!
//! This module implements block-level tracing that matches Reth's
//! `debug_traceBlockByNumber` RPC method, but with direct database access for
//! improved replay performance.

mod engine;
mod execution;
mod instrumented_db;
mod metrics;
mod profiled;

pub use engine::BlockTraceEngine;

use alloy_primitives::B256;
use alloy_rpc_types_trace::geth::{GethDebugTracingOptions, TraceResult};
use eyre::Result;
use reth_provider::{BlockHashReader, TransactionsProvider};
use std::time::Instant;

use crate::block_trace::types::{ProfiledBlockTrace, ReplayProfileConfig};
use crate::TxSimulator;

use self::metrics::ms;

/// Block tracer for simulating all transactions in a block.
pub struct BlockTracer<'a> {
    simulator: &'a TxSimulator,
}

impl<'a> BlockTracer<'a> {
    /// Create a new block tracer.
    pub fn new(simulator: &'a TxSimulator) -> Self {
        Self { simulator }
    }

    /// Trace all transactions in a block by number - equivalent to debug_traceBlockByNumber.
    pub async fn trace_block_by_number(
        &self,
        block_number: u64,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<Vec<TraceResult>> {
        self.trace_block_by_number_with_engine(block_number, opts, BlockTraceEngine::default())
            .await
    }

    /// Trace all transactions in a block by number using an explicit local trace engine.
    pub async fn trace_block_by_number_with_engine(
        &self,
        block_number: u64,
        opts: Option<GethDebugTracingOptions>,
        engine: BlockTraceEngine,
    ) -> Result<Vec<TraceResult>> {
        let opts = opts.unwrap_or_default();

        let provider = self.simulator.provider_factory.provider()?;
        let block_hash = provider
            .block_hash(block_number)?
            .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;

        self.trace_block_by_hash_with_engine(block_hash, opts, engine)
            .await
    }

    /// Trace all transactions in a block and return cold-replay profiling metrics.
    pub async fn trace_block_by_number_profiled(
        &self,
        block_number: u64,
        opts: Option<GethDebugTracingOptions>,
        engine: BlockTraceEngine,
    ) -> Result<ProfiledBlockTrace> {
        self.trace_block_by_number_profiled_with_config(
            block_number,
            opts,
            engine,
            ReplayProfileConfig::default(),
        )
        .await
    }

    /// Trace all transactions in a block with explicit profiling controls.
    pub async fn trace_block_by_number_profiled_with_config(
        &self,
        block_number: u64,
        opts: Option<GethDebugTracingOptions>,
        engine: BlockTraceEngine,
        config: ReplayProfileConfig,
    ) -> Result<ProfiledBlockTrace> {
        let opts = opts.unwrap_or_else(execution::call_tracer_options);
        let provider = self.simulator.provider_factory.provider()?;

        let hash_started = Instant::now();
        let block_hash = provider
            .block_hash(block_number)?
            .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;
        let block_hash_lookup_ms = ms(hash_started.elapsed());

        let simulator = self.simulator.clone();
        tokio::task::spawn_blocking(move || {
            let mut profiled =
                Self::trace_block_sync_profiled(&simulator, block_hash, opts, engine, config)?;
            profiled.profile.block_hash_lookup_ms = block_hash_lookup_ms;
            Ok(profiled)
        })
        .await?
    }

    /// Execute all transactions in a block without an inspector. This is a diagnostic lower bound
    /// for EVM/state work and does not produce production-valid traces.
    pub async fn execute_block_by_number_profiled(
        &self,
        block_number: u64,
        config: ReplayProfileConfig,
    ) -> Result<ProfiledBlockTrace> {
        let provider = self.simulator.provider_factory.provider()?;

        let hash_started = Instant::now();
        let block_hash = provider
            .block_hash(block_number)?
            .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;
        let block_hash_lookup_ms = ms(hash_started.elapsed());

        let simulator = self.simulator.clone();
        tokio::task::spawn_blocking(move || {
            let mut profiled = Self::execute_block_sync_profiled(&simulator, block_hash, config)?;
            profiled.profile.block_hash_lookup_ms = block_hash_lookup_ms;
            Ok(profiled)
        })
        .await?
    }

    /// Trace all transactions in a block by hash.
    pub async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<Vec<TraceResult>> {
        self.trace_block_by_hash_with_engine(block_hash, opts, BlockTraceEngine::default())
            .await
    }

    /// Trace all transactions in a block by hash using an explicit local trace engine.
    pub async fn trace_block_by_hash_with_engine(
        &self,
        block_hash: B256,
        opts: GethDebugTracingOptions,
        engine: BlockTraceEngine,
    ) -> Result<Vec<TraceResult>> {
        let simulator = self.simulator.clone();

        tokio::task::spawn_blocking(move || {
            Self::trace_block_sync_with_engine(&simulator, block_hash, opts, engine)
        })
        .await?
    }

    /// Trace a single transaction within a block, replaying all prior transactions.
    pub async fn trace_transaction_in_block_by_hash(
        &self,
        block_hash: B256,
        target_tx_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<TraceResult> {
        let simulator = self.simulator.clone();
        tokio::task::spawn_blocking(move || {
            Self::trace_transaction_in_block_sync(&simulator, block_hash, target_tx_hash, opts)
        })
        .await?
    }

    /// Trace a transaction by hash, automatically resolving its block.
    pub async fn trace_transaction_by_hash(
        &self,
        tx_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<TraceResult> {
        let opts = opts.unwrap_or_default();
        let provider = self.simulator.provider_factory.provider()?;
        let (_, meta) = provider
            .transaction_by_hash_with_meta(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Transaction {:?} not found", tx_hash))?;
        let block_hash = meta.block_hash;
        self.trace_transaction_in_block_by_hash(block_hash, tx_hash, opts)
            .await
    }
}
