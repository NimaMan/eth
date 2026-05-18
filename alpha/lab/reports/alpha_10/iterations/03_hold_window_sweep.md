# Iteration 03 - Hold Window Sweep

Question:

```text
Is hold15 still the best active-observation exit on a 50K range, or was it a
20K local optimum?
```

Policy change:

- Sweep hold windows above the previous best: 15, 20, 25, 30, and 40 active
  pool-update observations.

Reason:

- Earlier v2 sweep improved from hold5 to hold15.
- Profit review suggests launch momentum may need more than 15 observations.

Backtest mapping:

```bash
--strategy-suite risk-atlas-edge-v3
```

Review gate:

- Winner PnL must increase faster than open exposure and loser PnL.
- The chosen hold window must not depend on a single top winner.
