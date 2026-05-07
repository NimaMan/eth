use std::sync::Arc;

use alloy_rpc_types_trace::geth::{GethDebugTracingOptions, TraceResult};
use eyre::Result;

use crate::{
    block_trace::{
        block_tracer::{BlockTraceEngine, BlockTracer},
        types::{ProfiledBlockTrace, ReplayProfileConfig},
    },
    simulator::TxSimulator,
};

/// Stateful builder for replaying, tracing, and profiling a specific block.
#[derive(Clone)]
pub struct BlockReplaySession {
    simulator: Arc<TxSimulator>,
    block_number: u64,
    tracing_options: Option<GethDebugTracingOptions>,
    engine: Option<BlockTraceEngine>,
    profile_config: ReplayProfileConfig,
}

impl BlockReplaySession {
    pub(crate) fn new(simulator: Arc<TxSimulator>, block_number: u64) -> Self {
        Self {
            simulator,
            block_number,
            tracing_options: None,
            engine: None,
            profile_config: ReplayProfileConfig::default(),
        }
    }

    /// Block number this replay session is pinned to.
    pub const fn block_number(&self) -> u64 {
        self.block_number
    }

    /// Override geth debug tracing options for traced/profiled replays.
    pub fn with_tracing_options(mut self, options: GethDebugTracingOptions) -> Self {
        self.tracing_options = Some(options);
        self
    }

    /// Select the local replay engine. If omitted, callTracer options use the fused engine.
    pub fn with_engine(mut self, engine: BlockTraceEngine) -> Self {
        self.engine = Some(engine);
        self
    }

    /// Configure profiling details such as state-key recording or prewarm keys.
    pub fn with_profile_config(mut self, config: ReplayProfileConfig) -> Self {
        self.profile_config = config;
        self
    }

    /// Trace every transaction in the block and return Reth/geth-style trace results.
    pub async fn trace(&self) -> Result<Vec<TraceResult>> {
        let tracer = BlockTracer::new(&self.simulator);
        tracer
            .trace_block_by_number_with_engine(
                self.block_number,
                self.tracing_options.clone(),
                self.resolved_engine(),
            )
            .await
    }

    /// Trace the block and include replay timing/state-read profile data.
    pub async fn profile(&self) -> Result<ProfiledBlockTrace> {
        let tracer = BlockTracer::new(&self.simulator);
        tracer
            .trace_block_by_number_profiled_with_config(
                self.block_number,
                self.tracing_options.clone(),
                self.resolved_engine(),
                self.profile_config.clone(),
            )
            .await
    }

    /// Execute the block without an inspector and return the no-trace lower-bound profile.
    pub async fn execute_only_profile(&self) -> Result<ProfiledBlockTrace> {
        let tracer = BlockTracer::new(&self.simulator);
        tracer
            .execute_block_by_number_profiled(self.block_number, self.profile_config.clone())
            .await
    }

    fn resolved_engine(&self) -> BlockTraceEngine {
        self.engine.unwrap_or_else(|| {
            self.tracing_options
                .as_ref()
                .map(BlockTraceEngine::recommended_for_options)
                .unwrap_or_default()
        })
    }
}

impl TxSimulator {
    /// Start a block replay session pinned to `block_number`.
    pub fn block_replay_session(&self, block_number: u64) -> BlockReplaySession {
        BlockReplaySession::new(Arc::new(self.clone()), block_number)
    }
}
