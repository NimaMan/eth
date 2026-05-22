# Store

Crate: `eth_alpha_store`

PostgreSQL persistence for the alpha runtime. This crate is infrastructure: it implements the pure `eth_alpha_core::TradingStore` trait without putting SQL, migrations, connection pools, or dashboard queries into `core` or `engine`.

## Responsibilities

- Create and migrate the `alpha_trading` schema.
- Persist trader runs, heartbeats, order intents, execution reports, positions, position snapshots, risk events, and strategy observations.
- Keep indexed columns for common dashboard filters while retaining the full typed payload as JSONB.
- Provide query helpers for API/frontend layers later.

## Non-Responsibilities

- No strategy implementation.
- No token tracking.
- No mempool simulation.
- No transaction execution.
- No frontend routing.

## Runtime Contract

`eth_alpha_trader`, `eth_alpha_backtest`, `eth_alpha_lab`, and `eth_chain_server`
read `databases.alpha.url` from the shared `blockchains/eth/config.toml` file.
There is no `databases.mempool.url` fallback for alpha state.

`eth_alpha_store` owns the `alpha_trading` migrations embedded in
`src/lib.rs`. `alpha/lab` may add lab-only validation tables in the same schema,
but it must document those additions here.

Rows are tagged by `run_id`, so multiple chain-sim/live/replay runs can coexist:

| Table | Purpose |
| --- | --- |
| `alpha_trading.trader_runs` | One durable row per live, chain-sim, replay, or backtest run; stores mode/status/config/metadata and heartbeat timestamps. |
| `alpha_trading.order_intents` | Strategy order intents before execution, including portfolio/wallet, side, token/pool/protocol, amount/slippage/deadline, reason fields, and full payload. |
| `alpha_trading.execution_reports` | Execution lifecycle reports keyed by order/run, including status, tx hash, block, fill, gas, error, and payload. |
| `alpha_trading.positions` | Current position state per run/position, including trade id, token/pool/protocol, entry/exit orders, entry/exit blocks, and full payload. |
| `alpha_trading.position_snapshots` | Mark-to-market position snapshots with block coordinates, current value, realized/unrealized profit, ROI, and payload. |
| `alpha_trading.backtest_result_sets` | Named strategy/backtest result suites with mode/status/range/config/metadata. |
| `alpha_trading.backtest_result_set_runs` | Join table linking result sets to the run ids that produced them. |
| `alpha_trading.trades` | Trade-level rollup rows for result sets, including entry/exit/current value, realized/unrealized/total PnL, gas, ROI, latest snapshot blocks, and payload. |
| `alpha_trading.trade_events` | Trade execution event stream tied to `trades`, including order side/status, tx hash, block, fill, gas, error, and payload. |
| `alpha_trading.trade_snapshots` | Trade mark-to-market snapshots with pool price/liquidity context and PnL fields. |
| `alpha_trading.risk_events` | Strategy/run risk events with kind/severity, token/pool, optional pending tx, observed block, message, and payload. |
| `alpha_trading.strategy_decisions` | Auditable strategy decisions with event source/key, block, token/pool, action, reason fields, order side, and payload. |
| `alpha_trading.strategy_observations` | Restart-safe strategy input/watermark log for live pool updates, mempool signals, priming/submission decisions, and observed payloads. |
| `alpha_trading.strategy_validation_reports` | Lab-owned validation reports for persisted strategy result sets. Created by `alpha/lab` when validation runs are saved. |

The dashboard should read these tables or API endpoints backed by these tables. It should not reconstruct positions from journal logs.

`strategy_observations` stores the decision inputs and watermarks used by the trader: live pool updates, mempool signal ids, whether the event was primed/held/submitted, report count, and the full observed payload. This is where restart-safe signal and pool watermarks live.

The current SQL column is still named `pool_address` for compatibility, but alpha writes the canonical token-scoped `TokenPoolId` into it. Do not assume that value is always an EVM address; V4 rows use `token_address:pool_manager#pool_id`.
