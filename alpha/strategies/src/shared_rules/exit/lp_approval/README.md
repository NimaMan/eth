# LP Approval Exit Rule

Triggers an exit when an `LpApproval` risk event is received for a pool with
an open position.

## Logic

1. Event kind must be `RiskKind::LpApproval`.
2. Pool address is taken from `event.pool_address` or falls back to
   `ctx.market.pool_address`.
3. Strategy must have an open position for that pool.
4. If a minimum approval percentage is configured, the event approval percent
   must be strictly greater than that threshold.
5. If `lp_approval_exit_defer_max_trading_enabled_age_blocks` is configured,
   LP approvals inside that launch window are deferred to max-hold instead of
   forcing an immediate sell. Later approvals exit immediately.

Alpha11 sets the threshold to `>30%` and the launch-window defer threshold to
`<=2` chain blocks from trading enabled.

## Future Enhancement

Once creator public/private labels are wired, this rule can be restricted or
weighted by creator class, leaving known public deployers alone.
