# Liquidity Removal Exit Rule

Triggers an exit when a `LiquidityRemoval` risk event is received for a pool
that has an open, non-terminal position.

## Logic

1. Event kind must be `RiskKind::LiquidityRemoval`.
2. Pool address is taken from `event.pool_address` or falls back to
   `ctx.market.pool_address`.
3. Strategy must have an open position for that pool.

## Why Shared

Any strategy that holds pool positions should exit on liquidity removal.
This is not specific to snipe-all.
