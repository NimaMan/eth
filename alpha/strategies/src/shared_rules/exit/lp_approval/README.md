# LP Approval Exit Rule

Triggers an exit when an `LpApproval` risk event is received for a pool with
an open position.

## Logic

1. Event kind must be `RiskKind::LpApproval`.
2. Pool address is taken from `event.pool_address` or falls back to
   `ctx.market.pool_address`.
3. Strategy must have an open position for that pool.

## Future Enhancement

Once creator public/private labels are wired, this rule can be restricted to
`private` creators only, leaving `public` creators alone.
