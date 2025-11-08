# Simulation Manager Module

This directory orchestrates everything that happens after a transaction is routed
to the “simulation” path. The current layout is intentionally split into small,
testable units, but the main `manager.rs` file still owns most of the logic.

## Current Files

| File | Responsibility |
| --- | --- |
| `mod.rs` | Module wiring and public re-exports. |
| `block_pruner.rs` | Background task that trims stale sequences so we do not replay ancient transactions. |
| `pending_sequences.rs` | Data structure that keeps the processed transaction history per creator/token pair. |
| `types.rs` | Public structs (job/result enums) shared with the rest of the crate. |
| `manager.rs` | The heavy hitter that:<br>• owns the queue and concurrency limits<br>• delegates to `MempoolSimulator` / `LiquidityRemovalSimulator`<br>• runs per-pool buy/sell tests<br>• builds processed transactions for contract creations<br>• pushes results into the signal manager. |

## Pain Points

* `manager.rs` mixes orthogonal concerns (queueing, contract creation, pool
  buy/sell orchestration, logging helpers, tests). The file is ~1.3k LOC and
  growing, which makes subtle regressions likely.
* Contract-creation handling is interwoven with creator buy/sell handling despite
  the flows being largely independent.
* Helper utilities (formatting, request builders) live at the bottom of the file
  instead of next to their call sites or in focused modules.

## Alignment with the Higher-Level Simulator Module

The parent `src/simulator` directory wraps the general-purpose simulators that
live in `tx_processor`. These wrappers (`mempool_simulator`, `pool_buy_sell_simulator`,
`liquidity_removal_simulator`, etc.) expose ergonomic APIs tailored for mempool
transactions (shared DB handles, head cache integration, retry logic). The
simulation manager should remain a coordinator that composes those wrappers,
never re-implementing the low-level AMM logic that already exists in
`tx_processor`.

## Proposed Refactor

```
simulation_manager/
├── mod.rs
├── types.rs            # unchanged; request/result structs
├── request_queue.rs    # extracted SimulationQueue helpers + submit/process orchestration
├── contract_creation_flow.rs   # contract creation handling, nonce retry, deployment detection
├── creator_buy_sell_flow.rs    # creator transaction pipeline, per-pool simulation loop
├── liquidity_removal_flow.rs   # dedicated liquidity removal processing helpers
├── logging.rs          # shared formatting / append helper
├── tests.rs            # unit tests and dummy builders
└── pending_sequences.rs / block_pruner.rs (unchanged)
```

* `manager.rs` would shrink to a thin façade that wires these flows together and
  stores shared state (queue, token cache, signal manager).
* Each flow module can unit-test its behaviour in isolation (e.g. contract
  creation can mock `MempoolSimulator` and focus on nonce handling).
* `logging.rs` keeps the context-formatting helpers and any future multi-file
  logging snippets in a single place.
* `queue.rs` can encapsulate “pop batch / submit / stats” logic and make it
  reusable if we later add additional queue consumers.

This structure keeps the ergonomics of a single public `SimulationManager` API
while removing most of the accidental complexity that currently hides inside
`manager.rs`. Once the code is separated, we can invest in targeted tests per
flow instead of relying on a monolithic integration test.
