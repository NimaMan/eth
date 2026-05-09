Tracer: Geth Call (Current Default)

Purpose
- Produce geth-compatible CallFrame traces (optionally including logs) equivalent to debug_traceCall/Transaction/Block with callTracer.

Reth references
- Create inspector from geth call config: rust/reth/crates/rpc/rpc/src/debug.rs:298
- Export call traces: rust/reth/crates/rpc/rpc/src/debug.rs:760-772

Proposed config
- TracerKind::GethCall { with_log: bool }
- When with_log = true, set TracingInspectorConfig::from_geth_call_config(&call_config_with_log).

Planned APIs
- simulate_unsigned_with_tracer_at_block(..., TracerKind::GethCall { .. }) -> TraceOutput::GethCall
- simulate_signed_with_tracer_at_block(..., TracerKind::GethCall { .. }) -> TraceOutput::GethCall
- trace_block_with_tracer(..., TracerKind::GethCall { .. }) -> Vec<TraceOutput::GethCall>

Notes
- Inspector trace buffers are reset between sequential txs for chain and block tracing for parity and performance. Upstream calls that reset `fuse`.
