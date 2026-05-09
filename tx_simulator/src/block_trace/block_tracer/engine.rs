use alloy_rpc_types_trace::geth::{
    GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
};

/// Local block trace implementation to use when replaying a block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockTraceEngine {
    /// Historical implementation: build a fresh inspector for every transaction.
    FreshInspector,
    /// Reth-style implementation: reuse one call tracer inspector and reset it between txs.
    RethFusedCallTracer,
    /// Reth debug RPC style: one DebugInspector, get_result per tx, reset it between txs.
    RethDebug,
}

impl Default for BlockTraceEngine {
    fn default() -> Self {
        Self::RethFusedCallTracer
    }
}

impl BlockTraceEngine {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FreshInspector => "baseline-fresh",
            Self::RethFusedCallTracer => "tracing-fused",
            Self::RethDebug => "reth-debug",
        }
    }

    pub fn recommended_for_options(opts: &GethDebugTracingOptions) -> Self {
        if is_call_tracer_options(opts) {
            Self::RethFusedCallTracer
        } else {
            Self::RethDebug
        }
    }

    pub fn supports_options(self, opts: &GethDebugTracingOptions) -> bool {
        match self {
            Self::FreshInspector | Self::RethFusedCallTracer => is_call_tracer_options(opts),
            Self::RethDebug => true,
        }
    }
}

pub(super) fn is_call_tracer_options(opts: &GethDebugTracingOptions) -> bool {
    matches!(
        opts.tracer.as_ref(),
        Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer
        ))
    )
}
