/// Local block trace implementation to use when replaying a block.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BlockTraceEngine {
    /// Historical implementation: build a fresh inspector for every transaction.
    #[default]
    FreshInspector,
    /// Reth-style implementation: reuse one call tracer inspector and fuse it between txs.
    RethFusedCallTracer,
    /// Reth debug RPC style: one DebugInspector, get_result per tx, fuse between txs.
    RethDebug,
}

impl BlockTraceEngine {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FreshInspector => "baseline-fresh",
            Self::RethFusedCallTracer => "tracing-fused",
            Self::RethDebug => "reth-debug",
        }
    }
}
