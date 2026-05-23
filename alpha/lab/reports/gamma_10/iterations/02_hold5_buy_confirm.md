# 02 Hold5 Buy-Confirm Deferral

Strategy:

```text
gamma10-02-v2-hold5-buy-confirm
```

Question: can a very short active-observation hold avoid the earliest direct LP
removals while preserving enough launch upside?

Expected read: this should reduce some race exposure, but it may exit before
the large launch moves that funded alpha_10 profits.
