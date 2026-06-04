# Tax / Honeypot Exit Rule

Triggers an exit when a `TaxChange` or `Honeypot` risk event is received for a
pool with an open position.

## Logic

1. Event kind must be `RiskKind::TaxChange` or `RiskKind::Honeypot`.
2. Pool address is taken from `event.pool_address` or falls back to
   `ctx.market.pool_address`.
3. Strategy must have an open position for that pool.

## Why Shared

Tax changes and honeypot flags are universal danger signals. Any strategy
holding the token should exit.
