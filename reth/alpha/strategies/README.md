# Strategies

Planned crate: `eth_alpha_strategies`

This crate contains built-in strategies. Strategies are decision logic only.

## Responsibilities

- Implement `eth_alpha_core::Strategy`.
- Read `StrategyContext`, market snapshots, risk state, and current portfolio state.
- Return `StrategyDecision`.
- Keep strategy-local parameters and lightweight memory.

## Non-Responsibilities

- No transaction submission.
- No signing.
- No DB writes.
- No Redis reads.
- No ZMQ publishing.
- No direct mempool simulation.

## Initial Strategy Ports

The Python module had these useful starting points:

- `MarketTracker`: buys/tracks broad market performance; good for analytics and benchmarking.
- `BuyAll`: simple entry/exit state machine; good for validating engine behavior.
- `WalletTrackerStrategy`: wallet-specific limits and active-position tracking.
- `BuyScamStrategy`: useful as a risk/behavior experiment, but should be treated carefully.

In Rust, each should return intent instead of mutating position state:

```text
Hold
SubmitBuy(OrderIntent)
SubmitSell(OrderIntent)
Cancel(OrderId)
```

## Strategy Contract

A strategy can say what it wants. The engine decides whether it is allowed.

```text
StrategyDecision
  -> RiskCheck
  -> PortfolioLimits
  -> ExecutionAdapter
```

This avoids the Python problem where strategy, position manager, and signal publisher were tightly coupled.
