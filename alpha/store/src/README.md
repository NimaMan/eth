# Alpha Store

Postgres is the source of truth for live trading lifecycle state.

The live trader writes these records during a normal order lifecycle:

1. `order_intents`: the strategy decision and exact `OrderIntent`, including
   strategy, side, token, pool, amount, protocol, and reason code.
2. `positions`: the current position state, including submitted order ids,
   entry/exit blocks, fills, gas cost, and terminal state.
3. `execution_reports`: submitted and final execution reports. Reports tied to
   positions are written with `position_id`, `trade_id`, and `order_side`.
4. `strategy_decisions` and observation tables: why a strategy acted or held on
   a market, risk, or position-monitor event.

For real live trading, `load_submitted_executions()` loads submitted reports
with transaction hashes for receipt reconciliation. It is intentionally
cross-run: it finds any position currently in `buy_submitted` or
`sell_submitted` state with a pending tx hash, regardless of which run
submitted it. This ensures that transactions submitted by a previous process
that died before receiving the receipt are reconciled by the next process
instead of being orphaned permanently.

For chain-sim live backtesting, `load_chain_sim_submitted_executions()` loads
submitted reports with `mined_evidence.receipt_status =
live_backtest_chain_sim_submitted`. It joins back to the recorded
`OrderIntent`, carries the submitted block hash when it was recorded, requires
the position to still be in a submitted state, and excludes orders that already
have a final confirmed, failed, deferred, or cancelled report.

That means chain-sim settlement is restart-safe: the database submitted report,
not an in-memory queue, determines what still needs final simulation.
