# Alpha

`alpha/` is the Ethereum decision layer above the confirmed live feed, mempool risk, simulation, and transaction execution.

This area should not become another copy of the Python `eth_portfolio_manager`. The Python module proved the product shape, but it also mixed token projection, strategy logic, position lifecycle, persistence, ZMQ publishing, backtesting, and live execution into one package. The Rust design keeps those responsibilities explicit.

## Target Crates

| Folder | Crate name | Role |
| --- | --- | --- |
| `core/` | `eth_alpha_core` | Pure trading domain types and traits. |
| `engine/` | `eth_alpha_engine` | Live trading runtime, portfolio/order state, strategy scheduling, risk gating. |
| `strategies/` | `eth_alpha_strategies` | Built-in strategy implementations. |
| `store/` | `eth_alpha_store` | Durable run, decision, position, order, execution, and risk event records. |
| `backtest/` | `eth_alpha_backtest` | Historical replay and simulated execution using the same core traits. |
| `live/state/` | `eth_live_state` | Shared Redis live-state protocol and schemas. |
| `live/feed/` | `eth_live_feed` | Live confirmed-chain feed over processed blocks and token updates. |
| `mempool_risk/` | `eth_mempool_risk` | Pending-transaction simulation and speculative risk signals. |

Crates that are already compileable should stay small and explicit. New Cargo members should be added only when the boundary is stable enough to compile independently.

## Runtime Shape

```text
eth_live_feed
  -> writes canonical confirmed state to eth_live_state
  -> emits LiveFeedEvent

eth_mempool_risk
  -> reads eth_live_state
  -> simulates pending txs
  -> emits RiskEvent

eth_alpha_engine
  -> consumes LiveFeedEvent, RiskEvent, ExecutionReport
  -> runs eth_alpha_strategies
  -> submits approved orders to tx_executor

tx_executor
  -> signs, manages nonce/gas, broadcasts
  -> emits ExecutionReport
```

## Current Bottlenecks And Focus Order

The purpose of `alpha/` is to make the bottleneck visible, then move it. A strategy run should tell us whether the limiting factor is state freshness, mempool signal recall, decision quality, fill modeling, or execution. Decisions that affect positions must be written to the alpha store, not only logged.

| Order | Bottleneck | Owner | What To Watch | Next Focus |
| --- | --- | --- | --- | --- |
| 1 | Live readiness and restart recovery | `live/feed`, `live/state`, `eth_token_server` | live status, warmup progress, block source, block apply time, cache misses | After restart, verify warmup completes and live tail switches from `processed_block_disk_cache` to `live_redis_processed_block`; later add persisted live snapshots or faster warm resume. |
| 2 | Mempool signal recall and timing | `mempool_risk`, `mempool_signal_detector` | IPC drops, queue depth, arrival writes, first-seen timestamps, LP approvals before liquidity removals | Improve early liquidity-removal risk detection across pool types; LP approval, removal intent, token, pool, and first-seen time must be persisted before the trader consumes them. |
| 3 | Decision ledger quality | `engine`, `store` | every position has the market input, signal input, rule id, decision, order, execution report, exit reason, and PnL snapshot | Make the Postgres store the audit trail for all strategy decisions. In-memory watermarks are acceptable only for polling mechanics, not for decision facts. |
| 4 | Paper fill realism | `engine` | synthetic execution reports versus worst achievable block price | Replace placeholder paper fills with worst-case block fill modeling before trusting PnL. This belongs in `PaperExecutionAdapter`, not `mempool_risk`. |
| 5 | Strategy policy quality | `strategies` | Snipe All v1 entries, exits, risk reactions, skipped candidates | Keep `Snipe All v1` as the baseline: buy eligible pools above the liquidity floor, then exit on liquidity-removal risk. Extend with LP approval response, creator public/private labels, tax/honeypot response, position sizing, and pool filters. |
| 6 | Backtest and replay alignment | `backtest`, `engine` | same strategy state machine in historical and live paper runs | Historical replay should use confirmed blocks only unless recorded mempool arrivals/signals exist. Compare historical lower-bound PnL to live paper behavior. |
| 7 | Real execution handoff | `engine`, `tx_executor` | adapter boundary, execution reports, nonce/gas failures | Only replace the paper adapter with a `tx_executor` adapter after live state, signal recall, decision persistence, and fill modeling are measurable. |

Focus order is strict when diagnosing the live trader:

1. If the live feed is not `live`, fix live readiness first.
2. If the feed is live but exits are late or missing, fix signal recall and timing.
3. If signals are present but trades are confusing, fix the decision ledger.
4. If the ledger is complete but PnL is not credible, fix paper fills.
5. If fills are credible, iterate the strategy policy.

## Design Rules

1. Confirmed state and speculative state stay separate.
2. Strategies emit intent; they do not submit transactions.
3. Position transitions happen from execution reports, not from "next update" assumptions.
4. Redis live state is a read model, not the owner of trading decisions.
5. Signing authority stays behind `tx_executor`.
6. Backtest and live trading share the same strategy and portfolio state machine.

## Lessons From Python

- `LiveTokenTracker` became too broad. In Rust, live feed, strategy runtime, risk, and execution are separate services/crates.
- `TokenPosition` mixed token market state with our portfolio state. In Rust, `MarketState`, `PortfolioState`, `OrderState`, and `ExecutionState` are separate.
- Live and backtest engines duplicated state transitions. In Rust, live and backtest both feed `ExecutionReport` into the same engine logic.
- ZMQ address notifications plus Redis snapshots worked well as an invalidation/state-hydration pattern. Keep that, but move shared schemas into `eth_live_state`.
- Mempool simulation should read confirmed Redis overlays but should not mutate canonical token or chain state.
