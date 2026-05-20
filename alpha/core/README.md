# Core

Planned crate: `eth_alpha_core`

This crate owns the pure trading domain. It should compile without external
stores, ZMQ, RPC clients, `tx_executor`, or service orchestration.

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
- No external store key ownership.
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

## Pool Identity

Alpha decisions use `TokenPoolId`, not a raw pool contract address, as the unique pool key. The id is token-scoped:

```text
v2/v3: token_address:pool_contract_address
v4:    token_address:pool_manager#pool_id
```

This is the identity used by market events, risk events, order intents, positions, strategy memory, and watermarks. A pool contract, pool manager, V4 pool id, hook, fee tier, or protocol name is source metadata; it should not replace the token-scoped id when matching a position or deciding whether a pool was already bought.

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
