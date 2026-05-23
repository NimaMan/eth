# mempool_processor

Agent operating map for pending-transaction ingestion, classification,
simulation, and semantic signal emission.

## Purpose

- Read full pending transactions from local Reth IPC.
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

## Component Responsibilities

| Component | Responsibility |
| --- | --- |
| `MempoolFetcherIPCClient` | Maintains the local Reth IPC subscription and delivers full pending tx data without per-tx RPC fetches. |
| `function_detector` | Adds selector/function facts used by routing and diagnostics. |
| `tx_router` | Classifies txs against `TokenTrackingCache` as known critical, unresolved critical, or ordinary. |
| `SimulationManager` | Replays txs with nonce/funding dependencies, dispatches per-pool simulations, and emits `SimulationResult` values. |
| `SignalManager` | Converts mapped decoder/simulation facts into semantic per-pool signals. |
| `SignalPublisher` | Writes the public Postgres signal rows and diagnostic logs/ZMQ topics. |
| `MempoolArrivalRecorder` | Records first-seen pending tx timing and writes resolved mined tx timing to RethIndex. |

Key signal rules:

- Per-pool signals are independent; one token can produce separate signals for
  multiple tracked pools.
- `sell_blocked` requires a buy and approval path to work while the sell path
  fails for the same pool.
- `tax_change` is for configured threshold/bucket risk, not every routine tax
  calculation.
- Successful simulations are tracked through interval metrics. They are not
  persisted as one row per simulation.
- Tax and transfer math should come from `tx_processor` and the existing
  token-tracking calculation helpers, not duplicated in detectors.

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
  immediately reloads `/live/tokens` and `/live/pools`.
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
`databases.mempool.url` from `blockchains/eth/config.toml` and refuses to start without it unless
`--allow-database-disabled` is passed for a diagnostic run.

Persistent outputs:

| Store | Tables / keys | Purpose |
| --- | --- | --- |
| PostgreSQL `live_trading` schema | `signal_events`, `trading_enabled_details`, `sell_blocked_details`, `tax_change_details`, `liquidity_removal_details`, `lp_position_approval_details`, `token_supply_risk_details` | Canonical public semantic signal store for token-server, ASENA, and alpha. |
| RethIndex MDBX | `mempool_tx_arrival_times` | First-seen mempool arrival time for transactions after they are mined and resolved to Reth txumber. |
| Legacy `eth_db` timestamp updater | `eth_db.transactions.mempool_first_seen` | Older optional batch updater in `mempool_timestamp_tracker`; not the live public signal source. |

The public signal store is the PostgreSQL `live_trading` schema. Arrival timing
is analytics metadata and should not be used as a signal/event substitute.
`mempool_signal_detector` writes arrival timing through `MempoolArrivalRecorder`
and RethIndex; the legacy timestamp updater is not wired into the current live
detector path.

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

## Runtime Configuration

The binary reads CLI flags, `MEMPOOL_*` environment variables, and shared
`ETH_CONFIG_PATH` values where supported.

Required or primary live settings:

| Setting | Purpose |
| --- | --- |
| `MEMPOOL_IPC_PATH` / `RETH_IPC_PATH` | Local Reth IPC socket used for pending tx ingestion. |
| `MEMPOOL_RETH_DATADIR` / `RETH_DATADIR` | Reth data directory used for simulations and the `<datadir>/reth_index` arrival sidecar. |
| `databases.mempool.url` | Required Postgres URL for live semantic signal persistence. |
| `MEMPOOL_LIVE_TOKEN_SERVER_URL` | Token-server base URL for `/live/tokens`, `/live/pools`, and `/live/updates`. |
| `MEMPOOL_LOG_DIR` / `ETH_LOG_DIR` | Run log directory. |
| `MEMPOOL_SIM_WORKERS` | Simulation worker count; default is 4. |
| `MEMPOOL_ZMQ_SIGNAL_ENDPOINT` | Diagnostic ZMQ publisher endpoint; default is `tcp://127.0.0.1:5556`. |
| `MEMPOOL_TOKEN_CACHE_ETH_THRESHOLD` | Minimum ETH threshold used by token-cache pool heuristics. |

Operational defaults in the current binary:

- `--batch-size 100`
- `--sim-workers 4`
- `--simulation-timeout-ms 5000`
- `--report-interval 60`
- unresolved intents live for two seconds and retry every 250ms
- function-detection channel buffer is 50,000

## Live Signal Timing Evaluation

Before changing queueing, priority, or detector code, first verify the running
processes are on the latest timing-aware code and measure the current live
pipeline.

Latest-code checks:

- token-server/chain-server signal API must expose `signal_source`,
  `signal_created_at`, `mempool_first_seen_at`, and `mempool_first_seen_ms` on
  `/eth/tokens/api/mempool/signals`. If those fields are missing, the running
  server has not picked up the timing-field commit.
- `eth_alpha_trader` must create a `strategy_observations` row with decision
  `received` before `engine.handle_event` updates the same row to `submitted`,
  `hold`, `ignored`, or `invalid`. If no `received` phase appears for new
  signals after restart, the trader is not running the latest timing code.
- Use the persisted DB rows as the source of truth. ZMQ and logs are diagnostic.

What to measure after the latest code has run for a while:

| Stage | Source | Healthy expectation |
| --- | --- | --- |
| Mempool detection to signal store | `live_trading.signal_events.detection_timestamp` -> `created_at` | Usually sub-second for recent examples. |
| Signal store to API visibility | `signal_events.created_at` -> API row visible with same `signal_id` | Should be near the next API poll; missing timing fields means old server code. |
| API/trader receive time | `strategy_observations.first_seen_at` for event source `mempool_signal` and decision `received` | Should be bounded by trader poll cadence unless the trader is blocked. |
| Receive to risk/decision finish | `first_seen_at` -> `risk_events.created_at`, `strategy_decisions.created_at`, final observation `last_seen_at` | This is where chain-sim/report work can block later signals. |
| Decision to execution report | `strategy_decisions.created_at` -> `execution_reports.created_at` | Needed to separate decision latency from execution adapter latency. |

Current assessment from 2026-05-22 before restarting onto the latest timing
code:

- Recent `live_trading.signal_events` rows were written quickly: sampled
  detect-to-store latency ranged from about `6 ms` to `962 ms`.
- The running API at `40019` returned the old signal shape, so it was not yet
  exposing the committed timing fields.
- The running trader showed store-to-risk delays far above the `2000 ms` poll
  interval for some signals, including roughly `7.9s`, `9.2s`, `33.9s`,
  `56.4s`, and `68.6s`.
- The large delays looked like sequential trader-side blocking: one expensive
  `engine.handle_event`/chain-sim/report path can hold later mempool signals
  behind it.

Evaluation rule:

Do not treat mempool signal generation as the bottleneck until the timing table
shows `detection_timestamp -> signal_events.created_at` is slow. If that stage
is fast and `created_at -> received` or `received -> decision` is slow, fix the
trader/API critical path first.

Known item to revisit after the timing table is current:

- `src/simulator/simulation_queue.rs` should be audited for full-queue priority
  drop behavior. `SimulationPriority` uses lower numeric values for higher
  priority (`Critical=0`, `High=1`, `Normal=2`, `Low=3`), so any comparison that
  treats `<= Normal` as low priority is suspicious. Patch it only with a focused
  test and only after confirming whether queue pressure is actually present in
  the latest live run.

## Queue-Fill Failure Assessment

The repeated `IPC ingress queue full` failure should not be treated as normal
busy-period pressure. The old run logs show a stop-and-fill pattern:

- the last healthy interval has `ipc_queue=0`, no critical backlog, and
  `simulation_queue current=0`
- `simulation_queue enqueued` and `processed` are equal at the last interval
- interval reporting then stops, while the live token cache task keeps logging
  snapshots and IPC ingress later fills the 50,000 normal queue plus the 10,000
  critical queue

That means ingress is still alive, token-cache sync is still alive, and
simulation workers are not saturated. The detector consumer loop stopped
draining. Bigger queues or more simulation workers do not solve that class of
failure.

Changes now in the detector:

- arrival recording is a sidecar channel and no longer takes the arrival
  recorder mutex on the urgent signal path
- the IPC reader filters unrelated transactions before the signal queue, while
  still sending every tx to the arrival observer
- the critical lane now only admits tracked LP approvals, tracked position
  approvals, and tracked liquidity removals
- critical transactions are drained by an independent urgent consumer instead
  of sharing the normal detector loop
- normal and critical detector loops record their current stage and log a
  watchdog warning if either remains in one stage for more than five seconds
- signal-path awaits for routing, dependency recording, LP approval publishing,
  unresolved-intent writes, and simulation submission are bounded by short
  stage timeouts
- interval reporting is best-effort and time bounded; slow stats reads cannot
  stop the normal detector consumer from draining the IPC queue
- synchronous function detection no longer performs blocking async cache reads

The next failure should identify the stuck stage directly in
`signal_detector.log`. Proper fixes should then target that stage with a hard
timeout or isolation. The remaining architectural fixes are:

- keep ZMQ/log publishing non-blocking or time bounded
- fail the process or mark it unhealthy when the watchdog sees a stuck stage,
  so the supervisor restarts instead of letting the queue silently fill

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
