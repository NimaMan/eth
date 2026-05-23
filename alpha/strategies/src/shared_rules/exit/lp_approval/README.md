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
`<=2` active chain blocks from trading enabled. In other words, LP approval at
active age 0, 1, or 2 is deferred to max-hold; active age greater than 2 exits.

## Evidence Requirements

Every deferred LP approval must be auditable from persisted rows:

- `risk_events.payload.evidence.signal_id`
- `risk_events.payload.evidence.mempool_first_seen_at` for mempool signals when
  available
- `risk_events.payload.evidence.approved_share_pct`
- `risk_events.payload.evidence.trading_enabled_block`
- `risk_events.payload.evidence.trading_enabled_age_blocks_at_signal`
- `risk_events.payload.evidence.lp_approval_age_basis`
- matching `strategy_decisions.reason_details.risk_event_evidence`

If `trading_enabled_block` is missing, the live trader falls back to
`pool_creation_block` and records `lp_approval_age_basis=pool_creation_block`.
That fallback is acceptable for auditability but should be treated as weaker
evidence than a real trading-enabled block.

## Future Enhancement

Once creator public/private labels are wired, this rule can be restricted or
weighted by creator class, leaving known public deployers alone.
