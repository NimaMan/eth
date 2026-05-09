## Single-Transaction Simulation API

The `single_tx` module is the lowest-level surface area for probing Ethereum
transactions against the local Reth database. Every entry point follows the same
three-stage pipeline:

1. **Resolve block context**  
   - If a caller provides a `SealedHeader`, we reuse it. Otherwise the simulator loads a
     `BlockContext` from MDBX or tracked live state when MDBX is behind the requested block.

2. **Build the EVM environment**  
   - `create_tx_env` converts an `UnsignedTransaction` into a `revm::TxEnv`.  
   - Missing fields (nonce, gas price / fee, gas limit) are populated from canonical state and
     simulator defaults. Nonces are auto-detected unless explicitly provided.

3. **Execute via revm**  
   - Lightweight helpers use plain EVM execution. Trace helpers wire up a Geth call tracer or
     full step recorder depending on the helper being called.
   - Execution happens inside a `spawn_blocking` to guarantee it runs off the async executor.
   - Results are normalized to either a lightweight `SimulationResult` or a rich `FullSimulationResult`.
Equivalent RPCs
-----------------
* `simulate_unsigned_transaction` ↔ `eth_call` (latest block)
* `simulate_unsigned_transaction_at_block` ↔ `eth_call` with block parameter
* `simulate_unsigned_transaction_with_trace` ↔ reth `/debug/trace_call` (callTracer)
* `simulate_unsigned_transaction_with_full_trace_at_block` ↔ reth `/debug/trace_transaction` (callTracer with step recording)
* `simulate_unsigned_transaction_with_full_trace_on_state` ↔ reth `/debug/trace_transaction` with state overrides

The simulator runs these traces locally via reth’s MDBX database, avoiding RPC round trips while
producing identical call-frame/log structures (including revert data).

### Trace Fidelity

There are three distinct fidelity levels, mirroring the knobs exposed by reth’s debug API:

* **`simulate_unsigned_transaction`** – no-trace EVM execution. Equivalent to `eth_call`; returns only `success`, `gas_used`, and an optional revert string.
* **`simulate_unsigned_transaction_with_trace`** – Geth call tracer (`TracingInspectorConfig::default_geth()` with logs enabled and step recording disabled). Matches the shape of `/debug/trace_call?tracer=callTracer`, yielding the call tree, delegatecalls, emitted logs, and top-level return data. `FullSimulationResult::struct_logs` stays `None` so the payload remains light.
* **`simulate_unsigned_transaction_with_full_trace_*`** – same call tracer, but with `set_steps(true)` before execution. This populates `FullSimulationResult::struct_logs` with per-opcode step data (gas before/after, stack, memory, storage touches), reproducing what `/debug/trace_transaction` returns when step tracing is toggled on.

Keeping these helpers separate lets downstream callers opt into the heavier step recording only when they need it.

### Public Entry Points

| Method | Returns | Description | Typical Usage |
| --- | --- | --- | --- |
| `simulate_unsigned_transaction(unsigned)` | `SimulationResult` | Runs the call against the latest canonical block. | Quick “does this succeed and how much gas?” checks. |
| `simulate_unsigned_transaction_at_block(unsigned, block_number)` | `SimulationResult` | Same as above but pinned to a historical block. | Deterministic replays or comparisons. |
| `simulate_unsigned_transaction_on_state(unsigned, block_header, state)` | `SimulationResult` | Bypasses context resolution and executes on the supplied header/state snapshot. | Fork-aware tooling (bundle simulators, pipeline stages). |
| `simulate_unsigned_transaction_with_trace(unsigned, block_number?, block_header?)` | `FullSimulationResult` | Captures a call trace with `CallFrame` hierarchy (logs/returns, `struct_logs = None`). Context is resolved if omitted. | Diagnostics similar to `debug_traceCall`. |
| `simulate_unsigned_transaction_with_trace_on_state(unsigned, block_header, state)` | `FullSimulationResult` | Trace execution using a pre-fetched context. | When the caller already holds the snapshot. |
| `simulate_unsigned_transaction_with_full_trace_at_block(unsigned, block_number)` | `FullSimulationResult` | Highest fidelity (call tree + `struct_logs` step trace) at a specific block. | Deep debugging, MEV/DeFi research. |
| `simulate_unsigned_transaction_with_full_trace_on_state(unsigned, block_header, state)` | `FullSimulationResult` | Full trace with external context (retains `struct_logs`). | Batch systems or forks that manage their own state. |

### Return Types

* `SimulationResult` – `{ success: bool, gas_used: u64, revert_reason: Option<String> }`
* `FullSimulationResult` – extends the above with `call_trace: CallFrame` and an optional
  `struct_logs: Vec<StructLog>`. When produced by the full-trace helpers the struct logs mirror the
  `structLogs` array from Geth’s `debug_traceTransaction` (and reth’s `/debug/trace_transaction`).

### Internal Helpers

* `prepare_block_context` – orchestrates header/state resolution through `BlockContextLoader`,
  including tracked live state for blocks ahead of persisted MDBX.
* `create_tx_env` – fills in missing nonce / gas / fee data and builds the revm transaction
  environment.
* `run_unsigned_transaction` / `run_unsigned_transaction_with_trace` – the shared execution paths
  used by every public helper, ensuring consistent semantics regardless of which wrapper a caller
  chooses.

### Downstream Usage

* `tx_simulator/src/contract_method_simulator.rs` – uses the no-trace output path for
  read-only calls so view helpers do not pay call-tracer overhead.
* `tx_processor/src/processed_tx_provider/provider.rs` – calls the full-trace variants to populate
  `ProcessedTransaction` with per-opcode `struct_logs`, letting Python pipelines and DeFi analyses
  run richer post-processing without another tracing pass.
