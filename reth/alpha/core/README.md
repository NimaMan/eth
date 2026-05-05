# Core

Planned crate: `eth_alpha_core`

This crate owns the pure trading domain. It should compile without Redis, PostgreSQL, ZMQ, RPC clients, `tx_executor`, or service orchestration.

## Responsibilities

- Define shared event types:
  - `MarketEvent`
  - `RiskEvent`
  - `ExecutionReport`
  - `ControlEvent`
- Define trading state:
  - `Position`
  - `Portfolio`
  - `Order`
  - `StrategyRun`
  - `PortfolioSnapshot`
- Define decision types:
  - `StrategyDecision`
  - `OrderIntent`
  - `RiskDecision`
- Define traits:
  - `Strategy`
  - `ExecutionAdapter`
  - `TradingStore`
  - `RiskCheck`
  - `MarketDataView`

## Non-Responsibilities

- No block processing.
- No mempool polling.
- No direct transaction signing or broadcasting.
- No database-specific SQL.
- No Redis key ownership.
- No strategy implementations except tiny test fixtures.

## Important Boundary

Python's `TokenPosition` acted as both market snapshot and portfolio position. This crate should split that clearly:

```text
MarketState
  token/pool facts from chain

PortfolioState
  what this strategy/wallet owns or simulates owning

OrderState
  what we asked an executor to do

ExecutionState
  what actually happened on chain
```

Strategies receive read-only context and return decisions. They do not mutate engine state directly.

## Expected Public Surface

```rust
pub trait Strategy {
    fn name(&self) -> StrategyName;
    fn on_event(&mut self, ctx: &StrategyContext, event: &MarketEvent) -> StrategyDecision;
}

pub trait ExecutionAdapter {
    async fn submit(&self, intent: OrderIntent) -> Result<OrderSubmission>;
}
```

The exact API can change, but the dependency direction should not.
