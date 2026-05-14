# mempool_processor

Agent operating map for pending-transaction ingestion, classification,
simulation, and semantic signal emission.

## Purpose

- Read pending transactions from local Reth IPC/RPC.
- Detect relevant function selectors, classify creator/pool/token actions, and
  simulate effects against live token context.
- Queue critical pending intents whose token/pool mapping has not reached the
  confirmed live-token context yet, then retry them as soon as the mapping is
  available.
- Persist and publish explicit semantic events: `trading_enabled`,
  `sell_blocked`, `tax_change`, `liquidity_removal`, `lp_position_approval`,
  and `token_supply_risk`.
- Treat protocol support explicitly: V2-style Uniswap/Sushi/Pancake/Shiba/Frax
  LP approvals use ERC20 LP share, V3-style Uniswap/Sushi/Pancake approvals use
  position-liquidity share when mapped, Balancer uses BPT share, Curve uses LP
  share, and V4/Balancer composite identifiers stay strings.

## Owns

- Mempool fetchers and arrival timestamp recording.
- Function detection, transaction routing, simulation queueing, and per-pool
  pending simulation orchestration.
- Token-context hydration from `eth_chain_server`, including monotonic context
  acceptance rules.
- The short unresolved-intent lane for cache-waiting critical txs.
- Signal detectors, signal publishing, DB writers, and ZMQ notification output.

## Does Not Own

- Canonical token/pool state mutation; use `eth_token` via `eth_chain_server`.
- Core transaction decoding or tax math; consume `tx_processor` facts and
  buy/sell viability results.
- Raw EVM implementation; use `tx_simulator`.
- Strategy state or order placement; use `alpha` and `tx_executor`.

## Data Flow

```text
Reth IPC pending tx
  -> mempool_fetcher records arrival
  -> function_detector identifies selectors
  -> tx_router classifies priority/category using TokenTrackingCache
       | known token/pool critical tx
       |   -> simulation queue or direct LP-approval publication
       |
       | short cache-wait position approval / removal / creator control / V4 modify-liquidity
       |   -> unresolved intent store
       |   -> retry against current token cache for at most a couple seconds
       |   -> simulation queue or direct position-approval publication once mapped
       |
       ` ordinary tx
           -> drop after arrival accounting
  -> simulator uses TxSimulator + token context + tx_processor viability
  -> signal_detector emits semantic signal only after required context/checks pass
  -> Postgres rows + semantic signal logs + ZMQ tcp://127.0.0.1:5556
```

Entry-signal rule: publish `trading_enabled` only when the specific pool has
successful buy, approve, and sell simulation. Risk-signal rule: publish
sell-blocked, liquidity-removal, tax, supply-risk, and LP-position-approval
events only after the tx is mapped to a tracked token/pool and the decoder or
simulator confirms the risk. Cache waits are internal telemetry, not public
signals.

## Live Pipeline Boundary

Keep `mempool_signal_detector` as a separate runtime. Do not merge it into
token-server or the live token tracker. token-server owns confirmed-chain
token/pool state and HTTP read models; the mempool detector owns speculative
pending-transaction routing, simulation, and semantic signal persistence.

Live runtime contracts:

- `eth_chain_server` processes confirmed blocks, applies `eth_token`, persists
  processed blocks, and exposes live token/pool context over HTTP.
- `mempool_signal_detector` consumes Reth pending tx, token-server context, and
  local Reth-backed simulation state, then writes semantic signals to Postgres.
- `eth_alpha_trader` consumes token-server APIs and persisted mempool signals.
- ASENA reads token-server/trade APIs only; it should not consume ZMQ/logs
  directly.

Failure isolation rules:

- Pending-tx bursts or simulation failures must not stop token-server live
  block tracking.
- Live simulations use local Reth historical state or direct live state sessions
  supplied by the live chain runtime. If the needed block/state is unavailable,
  fail with a source-specific error.
- Mempool token context comes only from token-server `/live/tokens` and
  `/live/pools`. `/live/updates` is a notification-only long-poll wakeup: when
  token-server broadcasts `BlockApplied`, the request returns and mempool
  immediately reloads `/live/tokens` and `/live/pools`. Redis token snapshots
  and token-update ZMQ are not context sources.
- Live token-server snapshots are accepted while `status=warming` until the
  first live context is accepted. After that, only `status=live` snapshots are
  accepted, and lower-block snapshots are rejected and counted.
- ZMQ/log output is diagnostic; persisted `live_trading.signal_events` rows are
  the source of truth for token-server, ASENA, and alpha. Typed detail tables
  hang off `signal_id` for analytics.

## Live Persistence Rule

For live runs, Postgres signal persistence is required. token-server and ASENA
read mempool signals from `live_trading.signal_events` plus typed detail tables;
ZMQ and signal logs are diagnostic outputs. `mempool_signal_detector` loads
`MEMPOOL_DATABASE_URL` from the process environment or the shared
`ETH_CONFIG_PATH` config file and refuses to start without it unless
`--allow-database-disabled` is passed for a diagnostic run.

## Logging Contract

One run directory is created under `MEMPOOL_LOG_DIR`. Successful simulations are
reported through interval metrics only; they are not written one-by-one. The
high-signal diagnostic artifact is `simulation_errors.log`, which contains
simulation execution failures and buy/sell branch errors such as failed tax
calculation. Cache misses that mean "token-server has not published this
mapping yet" are not simulation errors; they are tracked through
`unresolved_intents.log` and interval metrics.

The `signals/` subdirectory is semantic only:

- `trading_enabled.log`
- `sell_blocked_signals.log`
- `tax_signals.log` for actual tax risk signals, not every tax calculation
- `liquidity_removals.log`
- `lp_approval_signals.log`
- `token_supply_risk_signals.log`
- `signal_manager.log` for emitted signals and publication summaries

Run-root diagnostics:

- `simulation_errors.log` for actionable simulator/viability failures
- `unresolved_intents.log` for LP approval, liquidity removal, creator-control,
  and V4 modify-liquidity txs waiting on token/pool context
- `external_data_updates.log` for token-server/cache context updates

## Where To Look First

| Need | Start here |
| --- | --- |
| Public module map | `src/lib.rs` |
| Binary wiring | `src/bin/mempool_signal_detector.rs`, `src/mempool_signal_detector_runtime/README.md` |
| Fetch and arrival tracking | `src/mempool_fetcher/`, `src/arrival_recorder.rs` |
| Function selectors | `src/function_detector.rs` |
| Routing/categories | `src/tx_router/` |
| Cache-wait lane | `src/unresolved_intents.rs` |
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
- Live simulation depends on fresh confirmed token context from
  `eth_chain_server`; unknown mappings may wait only in the short unresolved
  lane. If they are still unmapped after that same-mempool-window check, let
  confirmed block processing provide the later context.
- Do not duplicate tax or decoding logic here. Route to `tx_processor` and make
  detectors consume its output.
