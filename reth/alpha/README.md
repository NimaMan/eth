# Alpha

`alpha/` is the Ethereum decision layer above the confirmed live feed, mempool risk, simulation, and transaction execution.

This area should not become another copy of the Python `eth_portfolio_manager`. The Python module proved the product shape, but it also mixed token projection, strategy logic, position lifecycle, persistence, ZMQ publishing, backtesting, and live execution into one package. The Rust design keeps those responsibilities explicit.

## Target Crates

| Folder | Crate name | Role |
| --- | --- | --- |
| `core/` | `eth_alpha_core` | Pure trading domain types and traits. |
| `engine/` | `eth_alpha_engine` | Live trading runtime, portfolio/order state, strategy scheduling, risk gating. |
| `strategies/` | `eth_alpha_strategies` | Built-in strategy implementations. |
| `backtest/` | `eth_alpha_backtest` | Historical replay and simulated execution using the same core traits. |
| `live/state/` | `eth_live_state` | Shared Redis live-state protocol and schemas. |
| `live/feed/` | `eth_live_feed` | Live confirmed-chain feed over processed blocks and token updates. |
| `mempool_risk/` | `eth_mempool_risk` | Pending-transaction simulation and speculative risk signals. |

These folders are documentation-first scaffolding for now. They should become Cargo workspace members only when the crate boundary is ready to compile.

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
