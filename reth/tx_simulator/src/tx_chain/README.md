## Stateful Transaction Chain Simulation

The `tx_chain` module extends the single-transaction helpers with stateful execution paths.
It lets you simulate multi-step workflows (buy → approve → sell, MEV bundles, protocol
setups) against a forked view of the canonical or live-overlay chain while keeping all writes
in an in-memory database overlay.

For new mixed signed/unsigned workflows, prefer `TxSimulator::simulation_session*`. The older
unsigned and signed chain types remain useful focused surfaces, while `SimulationSession` wraps
the same fork loading behavior behind one API.

### Execution Pipeline

1. **Resolve fork context** – reuse a caller-supplied `SealedHeader`/state snapshot when
   available or let the simulator fetch canonical header/state (including live replay when
   MDBX lags). Headers are only required when callers inject their own fork material.
2. **Maintain forked state** – every call reads base state from Reth’s MDBX or a Redis live
   overlay and writes into an in-memory `CacheDB`. Executed transactions persist their changes
   for subsequent steps, including reverting transactions that still consume nonce and gas.
3. **Avoid tracing by default** – lightweight sequence paths use plain EVM execution. Trace
   helpers allocate inspectors only when call traces or struct logs are requested.

### Public Entry Points

| Helper | Returns | Description | Typical Usage |
| --- | --- | --- | --- |
| `TxSimulator::simulation_session*` | `SimulationSession` | Mixed signed/unsigned session pinned to a canonical or live-overlay fork. | Any arbitrary tx sequence with one warm state. |
| `TxSimulator::start_simulation_chain(at_block?)` | `UnsignedTxChainSimulation` | Interactive unsigned chain pinned to an optional block with automatic header/state loading. | Build stateful scenarios step-by-step (buy → approve → sell). |
| `UnsignedTxChainSimulation::step(unsigned)` | `SimulationResult` | Executes an unsigned tx, persists state, auto-manages nonces. | Iterative workflows where you inspect each result. |
| `UnsignedTxChainSimulation::step_with_trace(unsigned)` | `FullSimulationResult` | Same as `step` but returns call tree + `struct_logs`. | Debugging multi-step flows or feeding tx_processor. |
| `TxSimulator::start_signed_chain(at_block?)` | `SignedTxChainSimulation` | Interactive chain for signed transactions (recovers signer). | Replay signed bundles or replicate on-chain tx order. |
| `SignedTxChainSimulation::step(tx)` | `SimulationResult` | Executes and commits a signed transaction. | Deterministic replays without tracing overhead. |
| `SignedTxChainSimulation::step_with_trace(tx)` | `FullSimulationResult` | Signed analogue with geth-style call trace (no step logs). | When you need decoded internal calls for signed bundles. |
| `SignedTxChainSimulation::view_call_on_fork*` | `ViewFunctionResult` | Read-only helpers that reuse the forked state. | Fetch balances/allowances between steps. |
| `TxSimulator::simulate_unsigned_tx_sequence(txs, options)` | `SequentialSimulationResult` | One-shot batch execution on the no-trace path. Resolves block context internally (MDBX or live replay) based on `SequentialSimulationOptions::at_block`. | MEV bundles, regression suites, or any “all-at-once” replay. |

### Return Types

* `SimulationResult` – `{ success, gas_used, revert_reason }` for lightweight steps.
* `FullSimulationResult` – extends the above with `call_trace: CallFrame` and, when tracing is
  enabled, `struct_logs: Vec<StructLog>` so downstream processors can inspect opcode-level
  execution.
* `SequentialSimulationResult` – aggregate stats for batch helpers (per-tx results, totals,
  success/failure counts).

### Why Multiple Surfaces?

* **Simulation session** – best default for arbitrary sequences because it accepts signed and
  unsigned txs, exposes shared nonce/balance helpers, and keeps one warm fork.
* **Interactive unsigned chain** – best when you need a focused unsigned-only surface,
  or immediate inspection/modification between steps. Nonces are auto-detected and updated for
  you.
* **Interactive signed chain** – mirrors the unsigned chain but keeps signatures intact. Useful
  for replaying real bundles exactly as mined while still observing state changes.
* **Batch (simulate_unsigned_tx_sequence)** – maximizes throughput when you already know the
  full sequence upfront. Automatic block context resolution avoids redundant canonical lookups,
  and the no-trace path avoids inspector overhead.

### Inspector & State Notes

* Forked state writes never touch the canonical MDBX; they live entirely in the in-memory cache.
* View-call helpers run through a no-commit overlay, so reads between steps do not mutate the
  session/chain state.
* Block tracing still uses inspector fusing for callTracer replay. Plain sequence simulation does
  not allocate inspectors.
* `SequentialSimulationOptions` exposes knobs for stop-on-failure, custom gas limits, automatic
  nonce increments, avoiding redundant DB work even when simulating ahead of the last persisted block.

Use `tx_chain` whenever the effect of one transaction influences the next. For isolated calls
or simple single-step checks, stick to the `single_tx` module.
