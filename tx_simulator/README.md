# tx_simulator

Agent operating map for local Reth-backed EVM execution. This crate is the
lowest-level execution and tracing layer in the ETH workspace.

## Purpose

- Read state directly from Reth MDBX, with the datadir resolved from
  `RETH_DATADIR`, `RETH_DB_PATH`, or the repository-level `config.env`.
- Simulate signed transactions, unsigned calls, view functions, stateful
  transaction chains, bundles, blocks, and live-head state without RPC overhead.
- Run unsigned transactions without signatures, matching the `debug_traceCall`
  style workflow.
- Produce raw execution results, revert reasons, logs, and geth-compatible
  `CallFrame` traces for higher layers.
- Support independent parallel simulation for high-throughput workloads.
- Keep the crate focused on simulation only; result enrichment and business
  interpretation belong in `tx_processor`.

## Architecture

`tx_simulator` is intentionally split by simulation responsibility rather than
kept as one monolithic simulator file:

| Area | Responsibility |
| --- | --- |
| `src/simulator.rs` | Core `TxSimulator`, provider setup, Reth DB access, fork creation, base fee, and shared execution helpers |
| `src/single_tx/` | Isolated signed, unsigned, and parallel transaction simulation |
| `src/tx_chain/` | Stateful signed/unsigned chains and one-shot sequential transaction batches |
| `src/session/` | Higher-level mixed signed/unsigned sessions and block replay state |
| `src/block_context/` | Header/state loading from local Reth plus direct prestate-diff overlays for live callers |
| `src/block_trace/` | Full block tracing, call-frame extraction, and profiling |
| `src/contract_simulation/` | Read-only contract calls, calldata helpers, and simple output decoding |
| `src/revert/` | Revert payload decoding and simulation revert reason normalization |
| `src/tx_builders/` | Low-level transaction builders used by simulator examples and tests |

## Owns

- `TxSimulator` / `RethTxSimulator` provider setup, forks, EVM env, base fee,
  block context, and execution helpers.
- Single-tx simulation in `src/single_tx/`.
- Sequential signed/unsigned transaction chains in `src/tx_chain/`.
- Block tracing and replay in `src/block_trace/`.
- Live/latest-state simulation selection in `src/live/` and direct block state
  sessions from caller-supplied prestate diffs in `src/block_context/`.
- Revert decoding, trace shape conversion, and low-level tx builders needed for
  simulator examples.

## Does Not Own

- Protocol/business decoding, tax math, or balance-delta interpretation; use
  `tx_processor`.
- High-level chain/entity queries and DEX state readers; use
  `reth_chain_query`.
- Token/pool lifecycle state; use `eth_token`.
- Mempool routing, signals, or strategy decisions.

## Data Flow

```text
Reth MDBX + canonical headers
  -> TxSimulator creates a read-only provider/fork
  -> REVM executes tx/call/block with optional tracers
  -> FullSimulationResult / CallFrame / chain state output
  -> tx_processor, reth_chain_query, mempool_processor, pyreth
```

Real live simulation uses `LiveTxSimulator`, which is backed only by the latest
in-memory mined block session published by the live block processor. Latest
local historical context remains available through `TxSimulator` and
`LatestHistoricalTxSimulator`.

## Persistent Store Usage

`tx_simulator` opens the Reth node datadir read-only. It does not own a separate
application database.

| Path under `RETH_DATADIR` | Access | Purpose |
| --- | --- | --- |
| `db/` | read-only MDBX | Canonical state and Reth database tables used to build simulation state providers. |
| `static_files/` | read-only static-file provider | Canonical headers, receipts, and other append-only Reth data used for block context. |
| `rocksdb/` | read-only Reth provider component | Reth provider data opened by the current `ProviderFactory` setup. |

Simulation writes are fork-local/in-memory only. A transaction chain, bundle, or
view call must never mutate the Reth datadir.

## Where To Look First

| Need | Start here |
| --- | --- |
| Public API and re-exports | `src/lib.rs` |
| Core provider/fork helpers | `src/simulator.rs` |
| Unsigned call simulation | `src/single_tx/unsigned.rs` |
| Signed tx simulation | `src/single_tx/signed.rs` |
| Stateful tx chains | `src/tx_chain/unsigned.rs`, `src/tx_chain/signed.rs` |
| Block traces and parity | `src/block_trace/`, `examples/block/` |
| Latest/direct live state sessions | `src/live/`, `src/block_context/` |
| Revert/debug helpers | `src/revert/`, `src/tracers/` |

## Tests And Commands

```bash
cargo run -p tx_simulator --example verify_database_setup
cargo run -p tx_simulator --example unsigned_transaction_example
cargo run -p tx_simulator --example verify_block_trace_rpc_equivalence
cargo test -p tx_simulator
```

## Current Hazards

- At-block methods depend on canonical headers; immediately after import there
  can be a short canonicalization window.
- Full call traces are expensive. If a caller only needs success/gas/output, use
  no-trace paths.
- Do not add decoded event semantics here. Return raw traces/results and let
  `tx_processor` interpret them.
- Keep Reth/Alloy type versions aligned with dependent crates before changing
  public simulator types.
