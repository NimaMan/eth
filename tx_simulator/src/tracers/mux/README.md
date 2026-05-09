Tracer: Mux (Aggregate)

Purpose
- Combine multiple sub-tracers into one aggregate output (e.g., collect structlog + call trace simultaneously).

Reth references
- Mux tracer construction/export: rust/reth/crates/rpc/rpc/src/debug.rs:900-919

Planned APIs
- simulate_*_with_tracer_at_block(..., TracerKind::Mux) -> TraceOutput::Mux
- trace_block_with_tracer(..., TracerKind::Mux) -> Vec<TraceOutput::Mux>

Notes
- Requires mapping a mux config from user input to the builder used in Reth.

