# tests

Integration tests for `eth_alpha_core` contracts.

## Required Coverage

- Buy cannot confirm before submit.
- Sell cannot submit before buy confirmed.
- Failed buy moves to a safe terminal state.
- Scam risk can disable a position.
- Portfolio limits reject oversized/open-capacity orders.
- Backtest and live execution both update positions through `ExecutionReport`.
