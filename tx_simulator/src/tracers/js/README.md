Tracer: JS Tracers

Purpose
- Execute user-supplied JavaScript tracers (Geth-style) for custom trace outputs.

Reth references
- JS tracer path (feature gated): rust/reth/crates/rpc/rpc/src/debug.rs:936-979

Planned APIs
- simulate_*_with_tracer_at_block(..., TracerKind::Js { code, config }) -> TraceOutput::Js
- trace_block_with_tracer(..., TracerKind::Js { .. }) -> Vec<TraceOutput::Js>

Notes
- Implementation likely needs to reuse revm_inspectors::tracing::js::JsInspector; this is optional and can be gated.

