# Backtest

Planned crate: `eth_alpha_backtest`

This crate replays historical market data through the same trading core used by live trading.

## Responsibilities

- Load historical blocks or precomputed market events.
- Feed events into `eth_alpha_engine`.
- Provide a simulated execution adapter.
- Define fill models, slippage models, gas models, and latency assumptions.
- Produce reports, metrics, and snapshots.

## Non-Responsibilities

- No separate strategy API.
- No separate position state machine.
- No live signing or broadcasting.
- No mutation of canonical live Redis state.

## Core Principle

Backtest should not have special position transition logic.

Use the same shape as live:

```text
StrategyDecision
  -> OrderIntent
  -> SimulatedExecutionAdapter
  -> ExecutionReport
  -> Engine position update
```

The Python version had separate `BacktestStrategyEngine` and `LiveStrategyEngine` paths with duplicated state transitions. This crate should remove that duplication.

## Simulation Assumptions

Make assumptions explicit and configurable:

- fill price source
- slippage model
- gas cost model
- confirmation latency
- failed transaction behavior
- pool liquidity threshold
- scam/rug handling

Backtest reports should include the assumptions used for a run.
