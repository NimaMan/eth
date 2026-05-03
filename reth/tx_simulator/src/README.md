tx_simulator/src — Reth‑Backed Local EVM Simulation

Purpose
- Bypass RPC completely and simulate transactions against a local Reth database (MDBX) with the same execution environment and tracers that Reth uses internally.
- Produce geth‑compatible traces and results that are equivalent to Reth’s debug RPC, while avoiding network and JSON overhead.

What This Module Provides
- Direct state access: Builds a read‑only `StateProvider` over your local Reth DB and wraps it in a cached overlay for writes during simulation.
- Deterministic EVM setup: Derives `BlockEnv` and chain spec from canonical headers at a chosen block number.
- Unsigned and signed simulation: Single‑call helpers, plus stateful chain simulators that persist changes between steps.
- Geth‑compatible traces: Uses `TracingInspector::default_geth()` (with logs for trace variants) and exports geth `CallFrame`s.
- Inspector fusing: Reuses a single tracing inspector and fuses it between steps for Reth‑equivalent performance and behavior.

How This Compares To Reth
- Reth debug RPC (trace) constructs an EVM env from canonical headers, executes with `TracingInspector`, and fuses the inspector across sequential transactions for block/bundle tracing.
- We do the same locally via Reth crates, bypassing only the RPC layer.

Relevant Reth Source (for parity)
- Block and bundle tracing use a fused inspector between txs: rust/reth/crates/rpc/rpc/src/debug.rs:124
- Fusing pattern after each tx: rust/reth/crates/rpc/rpc/src/debug.rs:574
- Default geth structlog tracer setup: rust/reth/crates/rpc/rpc/src/debug.rs:872
- Lower‑level helpers that construct `TracingInspector`: rust/reth/crates/rpc/rpc-eth-api/src/helpers/trace.rs:72

Key Building Blocks Here
- TxSimulator (core): rust/tx_simulator/src/simulator.rs:1
  - Block metadata, provider factory, fork creation, base fee, and low‑level on‑fork execution helpers.
- Unsigned single‑call: rust/tx_simulator/src/single_tx/unsigned.rs:1
- Signed single‑call: rust/tx_simulator/src/single_tx/signed.rs:1
- Stateful unsigned chain: rust/tx_simulator/src/tx_chain/unsigned.rs:1
  - Persists state and nonces; fuses inspector between steps for performance and parity.
- Stateful signed chain: rust/tx_simulator/src/tx_chain/signed.rs:1
  - Recovers signer, persists state; fuses inspector between steps.
- Batch sequence (bundle): rust/tx_simulator/src/tx_chain/sequential.rs:1
  - Creates a fork, reuses a single inspector across the bundle, and fuses between txs.
- Trace decoding helpers: rust/tx_simulator/src/simulation_revert_decoder.rs:1

Equivalence Guarantees and Caveats
- Canonical headers: All at‑block methods read headers via `HeaderProvider::header_by_number`; immediately after import there can be a short canonicalization window where this returns None.
- Fees and gas: For signed txs we use tx‑provided gas and fees; for unsigned we allow EIP‑1559 or legacy fee fields and can derive safe defaults with base fee when needed.
- Trace format: Exported via geth builders; shape is intended to match `debug_*` RPC traces (including `withLog` when enabled).
- Inspector fusing: Chain and bundle simulators explicitly fuse the inspector after each tx, matching Reth’s block/bundle tracing behavior.

Typical Uses
- Replace `debug_traceCall`/`debug_traceBlockByNumber` with local, zero‑RPC equivalents.
- Evaluate multi‑tx workflows (buy → approve → sell) interactively with persisted state.
- Run high‑throughput offline analyses and benchmarks.

Quick Checks
- Verify database/setup: rust/tx_simulator/examples/general/verify_database_setup.rs:1
- Compare vs RPC: rust/tx_simulator/examples/block/verify_block_trace_rpc_equivalence.rs:1
- Contract reads: rust/tx_simulator/examples/general/contract_method_simulation.rs:1
- Signed chain demo: rust/tx_simulator/examples/sequential/buy_approve_sell_signed_chain_uniswap_v2.rs:1

Setup Notes
- Reth DB default: resolved from `RETH_DATADIR`, then `RETH_DB_PATH`, then `../../config.env` (`/home/nima/storage/samsung8tb/ethereum/reth` by default). Ensure it is synced and canonicalized to the block heights you simulate.
- Safe for concurrent use: We operate read‑only on MDBX; writes happen in an in‑memory overlay.

Scope
- tx_simulator focuses on fast and faithful execution/tracing only. Any higher‑level enrichment (log decoding, balance deltas, tax logic) lives in sibling crates like `tx_processor`.
