parity_traces — Parity Trace APIs (Planned)

Goal
- Provide equivalents for parity trace endpoints (trace_call, trace_transaction, trace_block, trace_filter) using local simulation.

Why
- Some downstream tools consume parity-style frames (flat/prestate). Supporting these completes our RPC parity story.

Planned APIs
- trace_call_parity(unsigned: UnsignedTransaction, block: Option<u64>, overrides: Option<...>, tracer: TracerKind::FlatCall|Prestate) -> eyre::Result<TraceOutput>
- trace_transaction_parity(tx_hash: B256, tracer: ...) -> eyre::Result<TraceOutput>
- trace_block_parity(block: BlockId, tracer: ...) -> eyre::Result<Vec<TraceOutput>>
- trace_filter_parity(filter: TraceFilter) -> eyre::Result<Vec<TraceOutput>>

Frames
- FlatCall and Prestate are the primary parity formats. We will mirror Reth’s parity builders for shapes.

Reth references
- Parity builder uses: rust/reth/crates/rpc/rpc/src/debug.rs:835-846 (flat call)
- Prestate: rust/reth/crates/rpc/rpc/src/debug.rs:780-793

Notes
- Some APIs require storage access patterns beyond a single block; we will document performance considerations and paging.

