# mempool_processor

Agent operating map for pending-transaction ingestion, classification,
simulation, and semantic signal emission.

## Purpose

- Read pending transactions from local Reth IPC/RPC.
- Detect relevant function selectors, classify creator/pool/token actions, and
  simulate effects against live token context.
- Persist and publish semantic signals such as trading enabled, tax bucket
  risk, honeypot/sell-blocked risk, LP approval, and liquidity removal.
- Treat protocol support explicitly: V2/Sushi LP approvals are early-warning
  signals, V2/Sushi and V3 pools can be buy/sell probed when metadata is
  present, and V4 removal intent is surfaced as unknown-severity risk until V4
  simulation is validated.

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

## Live Pipeline Boundary

Keep `mempool_signal_detector` as a separate runtime. Do not merge it into
token-server or the live token tracker. token-server owns confirmed-chain
token/pool state and HTTP read models; the mempool detector owns speculative
pending-transaction routing, simulation, and semantic signal persistence.

Live runtime contracts:

- `live_block_processor` publishes confirmed processed blocks and live state to
  Redis.
- `eth_token_server` consumes disk-cache/Redis blocks, applies `eth_token`, and
  exposes live token/pool context over HTTP.
- `mempool_signal_detector` consumes Reth pending tx, token-server context, and
  Redis/Reth simulation state, then writes semantic signals to Postgres.
- `eth_alpha_trader` consumes token-server APIs and persisted mempool signals.
- ASENA reads token-server/trade APIs only; it should not consume ZMQ/logs
  directly.

Failure isolation rules:

- Pending-tx bursts or simulation failures must not stop token-server live
  block tracking.
- Live simulations can use only local Reth historical state or Redis live state.
  If neither source has the needed block/state, fail with a source-specific
  error.
- ZMQ/log output is diagnostic; persisted Postgres rows are the source of truth
  for token-server, ASENA, and alpha.

## Live Persistence Rule

For live runs, Postgres signal persistence is required. token-server and ASENA
read mempool signals from `live_trading.*`; ZMQ and signal logs are diagnostic
outputs. `mempool_signal_detector` loads `MEMPOOL_DATABASE_URL` from the process
environment or the shared `ETH_CONFIG_PATH` config file and refuses to start
without it unless `--allow-database-disabled` is passed for a diagnostic run.

## Logging Contract

One run directory is created under `MEMPOOL_LOG_DIR`. Successful simulations are
reported through interval metrics only; they are not written one-by-one. The
high-signal diagnostic artifact is `simulation_errors.log`, which contains
simulation execution failures and buy/sell branch errors such as failed tax
calculation.

The `signals/` subdirectory is semantic only:

- `trading_enabled.log`
- `honeypot_signals.log`
- `tax_signals.log` for actual tax risk signals, not every tax calculation
- `liquidity_removals.log`
- `lp_approval_signals.log`
- `signal_manager.log` for emitted signals and publication summaries

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
