## Stateful Transaction Chain Simulation

The `tx_chain` module extends the single-transaction helpers with stateful execution paths.
It lets you simulate multi-step workflows (buy → approve → sell, MEV bundles, protocol
setups) against a forked view of the canonical chain while reusing inspectors and database
overlays for speed.

### Execution Pipeline

1. **Resolve fork context** – either reuse a caller-supplied `SealedHeader`/state snapshot or
   fetch the canonical header/state for the requested block. All helpers accept optional
   headers so repeated simulations avoid redundant MDBX lookups.
2. **Maintain forked state** – every call reads base state from Reth’s MDBX and writes into an
   in-memory `CacheDB`. Successful transactions persist their changes for subsequent steps.
3. **Reuse inspectors** – interactive chains keep a fused `TracingInspector` alive across steps
   so bundles cost roughly one `debug_traceBlock` instead of N independent traces.

### Public Entry Points

| Helper | Returns | Description | Typical Usage |
| --- | --- | --- | --- |
| `TxSimulator::start_simulation_chain(at_block?, header?)` | `UnsignedTxChainSimulation` | Interactive unsigned chain pinned to an optional block or supplied header. | Build stateful scenarios step-by-step (buy → approve → sell). |
| `UnsignedTxChainSimulation::step(unsigned)` | `SimulationResult` | Executes an unsigned tx, persists state, auto-manages nonces. | Iterative workflows where you inspect each result. |
| `UnsignedTxChainSimulation::step_with_trace(unsigned)` | `FullSimulationResult` | Same as `step` but returns call tree + `struct_logs`. | Debugging multi-step flows or feeding tx_processor. |
| `TxSimulator::start_signed_chain(at_block?)` | `SignedTxChainSimulation` | Interactive chain for signed transactions (recovers signer). | Replay signed bundles or replicate on-chain tx order. |
| `SignedTxChainSimulation::step(tx)` | `SimulationResult` | Executes and commits a signed transaction. | Deterministic replays without tracing overhead. |
| `SignedTxChainSimulation::step_with_trace(tx)` | `FullSimulationResult` | Signed analogue with geth-style call trace (no step logs). | When you need decoded internal calls for signed bundles. |
| `SignedTxChainSimulation::view_call_on_fork*` | `ViewFunctionResult` | Read-only helpers that reuse the forked state. | Fetch balances/allowances between steps. |
| `TxSimulator::simulate_unsigned_tx_sequence(txs, options)` | `SequentialSimulationResult` | One-shot batch execution with fused inspector. Honors optional header via `SequentialSimulationOptions::block_header`. | MEV bundles, regression suites, or any “all-at-once” replay. |

### Return Types

* `SimulationResult` – `{ success, gas_used, revert_reason }` for lightweight steps.
* `FullSimulationResult` – extends the above with `call_trace: CallFrame` and, when tracing is
  enabled, `struct_logs: Vec<StructLog>` so downstream processors can inspect opcode-level
  execution.
* `SequentialSimulationResult` – aggregate stats for batch helpers (per-tx results, totals,
  success/failure counts).

### Why Multiple Surfaces?

* **Interactive unsigned chain** – best when you need fine-grained control, dynamic branching,
  or immediate inspection/modification between steps. Nonces are auto-detected and updated for
  you.
* **Interactive signed chain** – mirrors the unsigned chain but keeps signatures intact. Useful
  for replaying real bundles exactly as mined while still observing state changes.
* **Batch (simulate_unsigned_tx_sequence)** – maximizes throughput when you already know the
  full sequence upfront. Optional block headers avoid repeated canonical lookups, and the fused
  inspector keeps tracing overhead low.

### Inspector & State Notes

* Forked state writes never touch the canonical MDBX; they live entirely in the in-memory cache.
* Inspectors are “fused” after each transaction so they shed per-tx buffers but retain internal
  allocations. This is the same strategy Reth uses for block tracing.
* `SequentialSimulationOptions` exposes knobs for stop-on-failure, custom gas limits, automatic
  nonce increments, and an optional `block_header` for zero-lookups execution.

Use `tx_chain` whenever the effect of one transaction influences the next. For isolated calls
or simple single-step checks, stick to the `single_tx` module.
