# Store

Crate: `eth_alpha_store`

PostgreSQL persistence for the alpha runtime. This crate is infrastructure: it implements the pure `eth_alpha_core::TradingStore` trait without putting SQL, migrations, connection pools, or dashboard queries into `core` or `engine`.

## Responsibilities

- Create and migrate the `alpha_trading` schema.
- Persist trader runs, heartbeats, order intents, execution reports, positions, position snapshots, and risk events.
- Keep indexed columns for common dashboard filters while retaining the full typed payload as JSONB.
- Provide query helpers for API/frontend layers later.

## Non-Responsibilities

- No strategy decisions.
- No token tracking.
- No mempool simulation.
- No transaction execution.
- No frontend routing.

## Runtime Contract

`eth_alpha_trader` should use `ALPHA_DATABASE_URL` when set. For the current deployment it falls back to `MEMPOOL_DATABASE_URL`, which already points at the local Ethereum Postgres database.

Rows are tagged by `run_id`, so multiple paper/live/replay runs can coexist:

```text
alpha_trading.trader_runs
alpha_trading.order_intents
alpha_trading.execution_reports
alpha_trading.positions
alpha_trading.position_snapshots
alpha_trading.risk_events
```

The dashboard should read these tables or API endpoints backed by these tables. It should not reconstruct positions from journal logs.
