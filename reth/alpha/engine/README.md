# Engine

Planned crate: `eth_alpha_engine`

This is the high-level live trading runtime. It consumes typed events, runs strategies, applies risk checks, manages portfolio/order state, and routes approved orders to execution.

## Responsibilities

- Own the main event loop.
- Maintain in-memory portfolio and order state.
- Run one or more strategies against market updates.
- Apply risk gates before order submission.
- Submit approved `OrderIntent`s through an `ExecutionAdapter`.
- Apply `ExecutionReport`s to position/order state.
- Persist state through a `TradingStore`.
- Expose operational snapshots for dashboards and recovery.

## Non-Responsibilities

- It does not parse blocks.
- It does not decode transaction logs.
- It does not simulate pending mempool txs directly.
- It does not sign transactions.
- It does not own Redis live-state schemas.

## Event Loop

```text
MarketEvent received
  -> update market view
  -> mark open positions to market
  -> run strategies
  -> convert StrategyDecision to OrderIntent
  -> apply portfolio/risk checks
  -> submit to ExecutionAdapter
  -> persist order state

ExecutionReport received
  -> update order state
  -> update position state
  -> persist trade/snapshot

RiskEvent received
  -> update risk view
  -> pause/cancel/sell if policy requires it
```

## Lessons From Python

The Python `LiveStrategyEngine` sometimes updated positions when a signal was submitted and later confirmed on the next token update. In this engine, confirmations should come from `ExecutionReport`.

Backtest mode can synthesize reports immediately, but it should still use the same report path:

```text
OrderIntent -> SimulatedExecutionAdapter -> ExecutionReport
```

Live mode uses:

```text
OrderIntent -> TxExecutorAdapter -> ExecutionReport
```

This keeps live and backtest behavior aligned.
