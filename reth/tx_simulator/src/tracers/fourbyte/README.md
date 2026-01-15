Tracer: FourByte

Purpose
- Collect method IDs (4-byte selectors) and aggregate frequency across execution.

Reth references
- FourByteInspector usage path: rust/reth/crates/rpc/rpc/src/debug.rs:302-314 (via built-in tracer variant)

Planned APIs
- simulate_*_with_tracer_at_block(..., TracerKind::FourByte) -> TraceOutput::FourByte
- trace_block_with_tracer(..., TracerKind::FourByte) -> Vec<TraceOutput::FourByte>

