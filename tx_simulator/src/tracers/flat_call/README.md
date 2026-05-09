Tracer: Flat Call (Parity)

Purpose
- Produce flattened call traces compatible with parity’s flat call tracer and downstream tools.

Reth references
- Inspector config from flat call: rust/reth/crates/rpc/rpc/src/debug.rs:816-827
- Export flat call frame (parity builder): rust/reth/crates/rpc/rpc/src/debug.rs:835-846

Planned APIs
- simulate_*_with_tracer_at_block(..., TracerKind::FlatCall) -> TraceOutput::FlatCall
- trace_block_with_tracer(..., TracerKind::FlatCall) -> Vec<TraceOutput::FlatCall>

Notes
- Builds via inspector.into_parity_builder().into_localized_transaction_traces(...)

