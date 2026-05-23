# 01 Same-Confirm Immediate LP Exit

Strategy:

```text
gamma10-01-v2-hold15-immediate-lp-exit
```

Question: what does performance look like if LP approval in the buy-confirm
block is treated as an immediate exit instead of a max-hold branch?

Expected read: if this collapses PnL, the same-confirm cohort is not simply a
bad branch. It contains the launch winners and needs better timing or ordering
support rather than a blunt quarantine.
