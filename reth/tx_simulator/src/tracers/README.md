tx_simulator/src/tracers — Tracer Modes (Planned)

Goal
- Expose the full set of tracer modes supported by Reth’s debug APIs, so callers can request the exact trace format they need while bypassing RPC.

Why
- Today we default to geth call-trace outputs. Reth also supports prestate, flat call, structlog, fourbyte, mux, and JS tracers. Parity APIs also expect flat/prestate style outputs. This folder documents the intended surface to add those without changing existing callers.

Proposed API surface (Rust, to be implemented)
- Enum for tracer selection:
  enum TracerKind { GethCall { with_log: bool }, Prestate, FlatCall, StructLog, FourByte, Mux, Js { code: String, config: serde_json::Value } }

- Unsigned call at block with tracer:
  fn simulate_unsigned_with_tracer_at_block(tx: UnsignedTransaction, block: u64, tracer: TracerKind) -> eyre::Result<TraceOutput>;

- Signed call at block with tracer:
  fn simulate_signed_with_tracer_at_block(tx: &TransactionSigned, block: u64, tracer: TracerKind) -> eyre::Result<TraceOutput>;

- Block tracing with tracer (by number/hash):
  fn trace_block_with_tracer(block: u64, tracer: TracerKind) -> eyre::Result<Vec<TraceOutput>>;

Where TraceOutput is an enum over the expected frame types:
  enum TraceOutput { GethCall(alloy_rpc_types_trace::geth::CallFrame), Prestate(PrestateFrame), FlatCall(FlatCallFrame), StructLog(StructLogFrame), FourByte(FourByteFrame), Mux(MuxFrame), Js(serde_json::Value) }

Parity with Reth
- TracingInspector constructors/config mapping mirrors Reth:
  - From geth call config: rust/reth/crates/rpc/rpc/src/debug.rs:298
  - From prestate config: rust/reth/crates/rpc/rpc/src/debug.rs:321
  - From flat call config: rust/reth/crates/rpc/rpc/src/debug.rs:816
  - Default structlog: rust/reth/crates/rpc/rpc/src/debug.rs:872
- Inspector reset between sequential txs: rust/reth/crates/rpc/rpc/src/debug.rs:124, 574. Upstream names this reset `fuse`.

Subfolders
- call/: Geth call tracer (current default)
- prestate/: Prestate tracer (account/storage pre-state)
- flat_call/: Parity flat call tracer (flattened calls)
- structlog/: Geth structlog tracer
- fourbyte/: FourByte inspector
- mux/: Mux tracer (aggregate)
- js/: JS tracer (code + config)
