# Alpha

Agent operating map for the Ethereum decision layer. `alpha/` consumes confirmed
live token state plus speculative mempool risk, turns those into strategy
events, and persists decisions before execution.

## Purpose

- Run trading strategy state machines over `MarketEvent`, `RiskEvent`, and
  `ExecutionReport`.
- Keep strategy decisions auditable in Postgres before trusting PnL.
- Keep paper/live/backtest logic behind the same core domain contracts.

## Owns

| Folder | Crate | Owns |
| --- | --- | --- |
| `core/` | `eth_alpha_core` | Domain types, traits, IDs, portfolio/order/position/risk models. |
| `engine/` | `eth_alpha_engine` | Runtime state machine, order/position transitions, risk gating, execution adapter boundary. |
| `strategies/` | `eth_strategies` | Concrete strategy rules such as `SnipeAllStrategy`. |
| `store/` | `eth_alpha_store` | Durable run, observation, order, execution, position, and risk records. |
| `live/state/` | `eth_live_state` | Redis live-state schemas and protocol types. |
| `live/feed/` | `eth_live_feed` | Confirmed processed-block/token feed used by live services. |
| `backtest/` | planned | Historical replay over the same core strategy contracts. |
| `mempool_risk/` | planned | Future crate boundary for pending-risk events; current service is `mempool_processor`. |

## Does Not Own

- Raw simulation, traces, or calldata building; use `tx_simulator`,
  `tx_processor`, and `reth_chain_query`.
- Canonical token/pool state mutation; use `eth_token` and `eth_token_server`.
- Signing, nonce management, gas policy, or broadcast; use `tx_executor`.
- Mempool ingestion or signal persistence; use `mempool_processor`.

## Data Flow

```text
tx_processor live_block_processor
  -> Redis eth/live/blocks + processed-block disk cache
  -> eth_token_server live token/pool views
  -> eth_alpha_trader polls /live/status, /live/pools, /mempool/signals
  -> strategy_observations + orders + reports + positions + risk events
```

`eth_alpha_trader` is paper-only right now. It must not become decision-active
until `/live/status` is `live`; while warming, it records heartbeats and primes
watermarks only.

## Where To Look First

| Need | Start here |
| --- | --- |
| Event and domain type ownership | `core/src/` and `core/src/README.md` |
| Strategy runtime and execution adapter behavior | `engine/README.md`, `engine/src/lib.rs` |
| Durable decision ledger | `store/README.md`, Postgres `alpha_trading.*` tables |
| Snipe All entry/exit rules | `strategies/README.md`, `strategies/src/snipe_all/` |
| Live confirmed-chain feed | `live/feed/README.md`, `live/feed/src/` |
| Redis live-state contract | `live/state/README.md`, `live/state/src/` |
| Service wiring | `engine/src/bin/eth_alpha_trader.rs` |

## Current Bottlenecks And Focus Order

The goal of `alpha/` is to make the current bottleneck measurable, then move it. A run is not useful unless it tells us whether the limit is live-state freshness, signal recall, decision quality, fill modeling, or execution.

| Order | Bottleneck | Owner | What To Watch | Next Focus |
| --- | --- | --- | --- | --- |
| 1 | Live feed readiness and failure isolation | `live/feed`, `eth_token_server`, `eth_token`, `tx_simulator` | live status, warmup progress, failed block, block apply time, simulation validation errors, V2/V3/V4 tracked-pool counters | Make live token apply resilient: optional pool metadata and buy/sell simulation failures must be recorded on the affected pool and must not fail the whole live tracker. |
| 2 | Mempool signal recall and timing | `mempool_processor`, future `mempool_risk` | IPC drops, queue depth, arrival writes, first-seen timestamps, LP approvals before liquidity removals, V2/V3/V4 pool identity coverage | Improve early liquidity-removal detection across pool types. LP approval, removal intent, token, canonical `TokenPoolId`, and first-seen time must be persisted before the trader consumes them. |
| 3 | Trader decision ledger completeness | `engine`, `store`, `eth_alpha_trader` | every decision input has `TokenPoolId`, market payload, signal payload, rule id, decision, order, execution report, exit reason, and PnL snapshot | Token-scoped pool identity is now the key path; next make every skip, entry, and exit auditable in Postgres so the frontend can explain strategy behavior. |
| 4 | Paper fill realism | `engine` | synthetic execution reports versus worst achievable block price | Replace placeholder paper fills with worst-case block fill modeling before trusting PnL. This belongs in `PaperExecutionAdapter`, not mempool risk. |
| 5 | Strategy policy quality | `strategies` | Snipe All v1 entries, exits, risk reactions, skipped candidates, V3/V4 behavior | Keep `Snipe All v1` as the baseline and extend it with LP approval response, creator public/private labels, tax/honeypot response, position sizing, and pool filters. |
| 6 | Backtest and replay alignment | `backtest`, `engine` | same strategy state machine in historical and live paper runs, same `TokenPoolId` matching | Historical replay should use confirmed blocks only unless recorded mempool arrivals/signals exist. Compare historical lower-bound PnL to live paper behavior. |
| 7 | Real execution handoff | `engine`, `tx_executor` | adapter boundary, execution reports, nonce/gas failures, real order id to `TokenPoolId` mapping | Only replace the paper adapter with a `tx_executor` adapter after live state, signal recall, decision persistence, and fill modeling are measurable. |

The next major bottleneck is live feed readiness and failure isolation. The current live tracker can fail warmup from a simulation validation path, such as insufficient simulated funds. That should become a pool-level trading-status failure, not a runtime failure. Until the tracker reliably reaches `live`, paper trading cannot produce dependable strategy measurements.

## Tests And Commands

```bash
cargo test -p eth_alpha_core
cargo test -p eth_alpha_engine
cargo test -p eth_alpha_store
cargo test -p eth_strategies
cargo run -p eth_alpha_engine --bin eth_alpha_trader
```

## Current Hazards

- `eth_alpha_trader` uses paper execution only; do not route it to
  `tx_executor` without an explicit adapter and persistence plan.
- `strategy_observations` is the durable input log. In-memory watermarks are
  polling mechanics and must be recoverable from Postgres.
- Fix order for live issues: live feed readiness and failure isolation, mempool
  signal recall, decision ledger completeness, paper fill realism, then
  strategy policy.
- Keep confirmed state and speculative mempool risk separate. Strategies consume
  both but do not mutate either.
