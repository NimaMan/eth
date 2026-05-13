# Mempool Signal Detector Service

`mempool_signal_detector` is the live trading signal layer for pending Ethereum
transactions. It remains separate from token-server: token-server owns confirmed
mined token/pool state, while this service owns speculative pending-tx routing,
simulation, cache-wait handling, and semantic signal persistence.

## Operating Goals

1. Protect open positions quickly when a pending transaction can rug, drain,
   block selling, or materially change tax/trading behavior.
2. Emit entry signals only when the specific pool is buy/approve/sell viable.

Risk publication requires token/pool mapping plus decoder or simulation
confirmation. Entry publication requires successful buy, approve, and sell
simulation for the pool. Internal cache waits are telemetry, not public signals.

## Runtime Data Flow

```text
Reth pending tx stream
  -> MempoolFetcherIPCClient
       records first-seen arrival time
  -> FunctionDetector
       adds selector/function facts
  -> TransactionRouter + TokenTrackingCache
       known critical tx
         -> simulation queue or LP-approval fast path
       unresolved critical tx
         -> UnresolvedIntentStore
         -> retry against newer token cache
       ordinary tx
         -> ignored after accounting
  -> SimulationManager
       replays tx against live state
       runs per-pool buy/approve/sell checks where supported
       builds SimulationResult
  -> SignalManager
       emits semantic TradingEnabled / Tax / Honeypot / LiquidityRemoval /
       LpApproval signals
  -> SignalPublisher
       writes Postgres rows, semantic logs, and ZMQ topics
```

## Token Context Contract

The service hydrates and refreshes `TokenTrackingCache` from token-server live
HTTP endpoints. The refresh loop is:

```text
fetch /live/tokens + /live/pools
wait on /live/updates?after_block=<accepted_block>
token-server BlockApplied wakes the long-poll
fetch /live/tokens + /live/pools again
```

`/live/updates` carries wakeup metadata only; it is not an authoritative token
payload.

Cache acceptance is monotonic:

- Accept `status=warming` until the first live context is accepted, then accept
  only `status=live`.
- Reject empty live-server hydrate responses as unsuccessful startup hydration.
- Reject snapshots whose context block is lower than the last accepted block.
- Log accepted/rejected context with block, status, source, and rejection
  counters.

This prevents a token-server restart or warmup response from rolling the mempool
processor back to stale token/pool mappings. Redis token snapshots and
token-update ZMQ are not used for token context.

## Unresolved Intent Lane

Some important mempool txs arrive before token-server has published the
confirmed token/pool mapping. These are queued instead of being dropped or
reported as simulation errors.

Queued intent kinds:

- LP approvals against pools/LP tokens that are not mapped yet.
- Liquidity removals whose token/pool cannot be mapped yet.
- Creator-control calls waiting for target-token context.
- V4 modify-liquidity calls.

The store is bounded and uses a 10 minute TTL. It periodically retries intents
against the current token cache. When a mapping appears, the tx is routed to the
same path it would have used if the mapping had been present originally:

- Known LP approval: publish immediately without simulation.
- Liquidity removal / creator-control / V4 removal: submit to simulation.
- Still unmapped: keep pending until the retry interval or TTL expiry.

Interval logs include unresolved `pending`, `in_flight`, `recorded`,
`resolved`, `expired`, `dropped`, and average cache-wait timing.

## Routing Rules

Fast paths:

- Known token creator/control transactions are simulated by priority.
- Known V2/Sushi LP approvals to routers or Permit2 are published immediately as
  early rug setup signals.
- Known liquidity removals go through the dedicated liquidity-removal flow.

Cache-wait paths:

- Critical txs without token/pool mapping go to `UnresolvedIntentStore`.
- Cache-wait errors use `unresolved_cache_context` semantics and do not belong
  in `simulation_errors.log`.

Unsupported paths:

- V4 pool identity is a string: `pool_manager#pool_id`. Never parse it as an EVM
  address.
- V4 liquidity-removal signals may publish the composite pool identifier once
  mapped.
- V4 trading-enabled entry signals are disabled until full buy/approve/sell
  simulation is implemented and succeeds.

## Simulation And Signals

`SimulationManager` owns the simulation queue and worker tasks. It consumes
`TxSimulationJob` values, replays transactions with `TxSimulator`, uses
`tx_processor` for rich decoded facts and viability, then sends
`SimulationResult` values to `SignalManager`.

Signal semantics:

- `TradingEnabled`: buy, approve, and sell succeed for the pool with acceptable
  tax.
- `Honeypot`: buy/approve succeed but sell fails for the same pool.
- `TaxSignal`: buy/sell tax enters high or extreme buckets, or crosses
  configured thresholds.
- `LiquidityRemoval`: tracked pool drain/removal risk.
- `LpApproval`: tracked LP/pool token approval to known router/Permit2.

Successful mined tx replay mismatches are classified as
`replay_context_mismatch`; they are diagnostic context mismatches, not
token/pool mapping failures.

## Outputs

Live runs require Postgres signal persistence unless
`--allow-database-disabled` is explicitly passed for diagnostics.

ZMQ topics are diagnostic/low-latency fanout:

- `trading_enabled`
- `honeypot_signal`
- `tax_signal`
- `liquidity_removal`
- `lp_approval`

The stable consumer path for token-server, ASENA, and alpha is the persisted
`live_trading.*` signal store.

## Log Structure

Each run creates:

```text
logs/mempool_processor/signal_detector_YYYY-MM-DD_HH-MM-SS/
├── signal_detector.log          # service lifecycle, interval metrics
├── external_data_updates.log    # token context refreshes
├── simulation_results.log       # compact simulation summaries where enabled
├── simulation_errors.log        # actionable simulator/viability errors
├── unresolved_intents.log       # internal cache-wait lane
└── signals/
    ├── trading_enabled.log      # semantic entry signals
    ├── honeypot_signals.log     # semantic cannot-sell signals
    ├── tax_signals.log          # semantic tax risk signals
    ├── liquidity_removals.log   # semantic removal/drain signals
    ├── lp_approval_signals.log  # semantic LP approval signals
    └── signal_manager.log       # publication summaries
```

`simulation_errors.log` is for actionable simulation failures. Repeated "no
tracked token found" cache waits and V4 composite-id address parsing failures
should not appear there after the unresolved-intent path is active.

## Interval Metrics To Watch

- Mempool ingress: received/dropped IPC counts and simulation queue depth.
- Simulation submitted/ok/error totals.
- Signal counts and publisher errors.
- LP approval path: router approvals, tracked approvals, cache misses,
  published count, DB errors.
- Token cache: token/pool/creator counts plus context block/status/source and
  stale/non-live rejection counters.
- Unresolved intents: pending, in-flight, resolved, expired, dropped,
  cache-wait timing.
- Arrival recorder: pending, resolved, written, unresolved, expired.

## Command

```bash
cargo run -p mempool_processor --bin mempool_signal_detector -- \
  --batch-size 100 \
  --sim-workers 4 \
  --report-interval 30
```

Useful options:

- `--config` / `MEMPOOL_CONFIG_PATH`: optional TOML config path.
- `--ipc-path` / `MEMPOOL_IPC_PATH`: Reth IPC socket path.
- `--reth-db-path` / `MEMPOOL_RETH_DATADIR`: Reth datadir for simulations.
- `--log-dir`: base log directory.
- `--batch-size`: pending tx batch size.
- `--sim-workers`: simulation worker count.
- `--simulation-timeout-ms`: max wall-clock time for one pending simulation.
- `--report-interval`: interval metric cadence in seconds.
- `--allow-database-disabled`: diagnostic ZMQ/log-only mode.

## Operational Checks

Healthy live behavior looks like:

- Token-server snapshots are monotonic by block; `warming` is accepted only
  before the first `live` context.
- Simulation queue depth returns to zero between bursts.
- LP approval cache misses do not grow without corresponding unresolved-intent
  telemetry.
- Liquidity-removal cache waits appear in `unresolved_intents.log`, not
  `simulation_errors.log`.
- V4 `pool_manager#pool_id` identifiers pass through as strings.
- Entry signals are absent for unsupported V4 pools until buy/approve/sell is
  fully supported.

When signals are missing, first check token context freshness and unresolved
intent counts. Then check whether the tx is known/mapped, whether it was
submitted to simulation, and whether the final signal was persisted.
