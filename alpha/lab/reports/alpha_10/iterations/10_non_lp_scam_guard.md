# Iteration 10 - Non-LP Scam Guard

Question:

```text
What losses remain after Direct LP controls, and do they need pair-balance or
reserve-drain models instead of more LP approval rules?
```

Policy change:

- Keep the best Direct LP policy.
- Add separate review buckets and future targets for:
  - pair-balance backdoor drain;
  - unknown reserve drain;
  - reserve dump drain.

Reason:

- Direct LP removal is 45.00% of scam labels.
- Pair-balance backdoor drain is 36.02%.
- Loss scan found 52 losing trades with no pool risk event before loss.

Implementation status:

- This is a next-model requirement, not a Direct LP rule tweak.
- It should become a separate Risk Atlas target and strategy gate.

Review gate:

- Quantify no-risk-event losses in the 50K result.
- Promote Direct LP policy only with an explicit residual-risk note if this
  bucket remains material.
