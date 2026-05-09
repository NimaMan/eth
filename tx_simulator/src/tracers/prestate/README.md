Tracer: Prestate (Parity)

Purpose
- Emit prestate traces capturing account/storage pre-state touched by the transaction. Used by parity-style RPC and tooling.

Reth references
- Inspector config from prestate: rust/reth/crates/rpc/rpc/src/debug.rs:321-333
- Export prestate traces: rust/reth/crates/rpc/rpc/src/debug.rs:780-793

Planned APIs
- simulate_*_with_tracer_at_block(..., TracerKind::Prestate) -> TraceOutput::Prestate
- trace_block_with_tracer(..., TracerKind::Prestate) -> Vec<TraceOutput::Prestate>

Output
- PrestateFrame (to be defined): parity-compatible structure. We will mirror Reth’s builder output.

Notes
- Inspector fusing applies across sequential transactions.

