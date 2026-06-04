# Scam Exit Rule

Triggers an exit when a critical `Scam` risk event is received for a pool with
an open position.

## Logic

1. Event kind must be `RiskKind::Scam` **or** severity is `Critical`.
2. Pool address is taken from `event.pool_address` or falls back to
   `ctx.market.pool_address`.
3. Strategy must have an open position for that pool.

## Why Shared

A critical scam flag is a universal stop-loss. No strategy should hold through
it.
