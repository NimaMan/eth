# mempool_processor

Agent operating map for pending-transaction ingestion, classification,
simulation, and semantic signal emission.

## Purpose

- Read pending transactions from local Reth IPC/RPC.
- Detect relevant function selectors, classify creator/pool/token actions, and
  simulate effects against live token context.
- Persist and publish semantic signals such as trading enabled, high tax,
  honeypot risk, LP approval, and liquidity removal.

## Owns

- Mempool fetchers and arrival timestamp recording.
- Function detection, transaction routing, simulation queueing, and per-pool
  pending simulation orchestration.
- Token-context hydration from `eth_token_server`.
- Signal detectors, signal publishing, DB writers, and ZMQ notification output.

## Does Not Own

- Canonical token/pool state mutation; use `eth_token` via `eth_token_server`.
- Core transaction decoding or tax math; consume `tx_processor` facts and
  buy/sell viability results.
- Raw EVM implementation; use `tx_simulator`.
- Strategy state or order placement; use `alpha` and `tx_executor`.

## Data Flow

```text
Reth IPC pending tx
  -> mempool_fetcher records arrival
  -> function_detector + tx_router classify priority/category
  -> simulator uses TxSimulator + token context + tx_processor viability
  -> signal_detector emits semantic signal
  -> Postgres rows + signal logs + ZMQ tcp://127.0.0.1:5556
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Public module map | `src/lib.rs` |
| Binary wiring | `src/bin/README.md`, `src/bin/mempool_signal_detector.rs` |
| Fetch and arrival tracking | `src/mempool_fetcher/`, `src/arrival_recorder.rs` |
| Function selectors | `src/function_detector.rs` |
| Routing/categories | `src/tx_router/` |
| Pending simulation | `src/simulator/README.md`, `src/simulator/` |
| Signal decisions | `src/signal_detector/README.md`, `src/signal_detector/` |
| Persistence/publishing | `src/db_writers/`, `src/signal_publisher.rs` |
| Replay/diagnostic examples | `examples/README.md` |

## Tests And Commands

```bash
cargo run -p mempool_processor --bin mempool_signal_detector
cargo run -p mempool_processor --example function_detector_example
cargo run -p mempool_processor --example tx_router_example
cargo run -p mempool_processor --example full_pipeline_signal_detection
cargo test -p mempool_processor
```

## Current Hazards

- Contract-creation flow is still limited. Same-block deployment, approval, and
  liquidity helper sequences can be truncated unless the pending sequence buffer
  captures all required helpers.
- `simulate_mempool_tx_with_state_changes` is not the canonical rich diff path;
  prefer processed tx/buy-sell facts from `tx_processor`.
- Live simulation depends on freshest confirmed token context from
  `eth_token_server`; stale context can suppress or misclassify signals.
- Do not duplicate tax or decoding logic here. Route to `tx_processor` and make
  detectors consume its output.
