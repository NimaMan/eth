# Iteration 07 - Buy-Confirm LP Approval Quarantine

Question:

```text
How much edge remains if pools with LP approval in the buy-confirmation block
are treated as non-replicable or quarantined?
```

Policy change:

- Keep baseline rules.
- Add a review-only cut that excludes or separately reports trades where LP
  approval first appears in the buy-confirmation block.

Reason:

- The strategy submitted the buy before confirmed LP approval was visible.
- Live replication depends on next-block ordering and private or protected
  routing.

Implementation status:

- Immediate LP exit and defer-to-max-hold already exist as comparison branches.
- A strict "quarantine after confirmation" coded variant would need an explicit
  post-buy de-risk or cancel-like policy.

Review gate:

- Report baseline PnL with same-confirmation-block trades removed.
- Promote only if the 50K edge survives this exclusion or live-ordering support
  is accepted as part of the policy.
