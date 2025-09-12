Tracer: Geth StructLog

Purpose
- Emit geth structlog style traces used by some analysis pipelines.

Reth references
- Default structlog inspector: rust/reth/crates/rpc/rpc/src/debug.rs:872-878
- Export structlog frames: rust/reth/crates/rpc/rpc/src/debug.rs:880-889

Planned APIs
- simulate_*_with_tracer_at_block(..., TracerKind::StructLog) -> TraceOutput::StructLog
- trace_block_with_tracer(..., TracerKind::StructLog) -> Vec<TraceOutput::StructLog>

