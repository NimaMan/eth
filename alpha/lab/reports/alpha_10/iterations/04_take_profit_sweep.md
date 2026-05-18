# Iteration 04 - Take-Profit Sweep

Question:

```text
Can take-profit exits realize launch momentum earlier without cutting the rare
winners that pay for the loser surface?
```

Policy change:

- Keep LP gate and LP exits.
- Sweep take-profit ratios at 2x, 3x, 5x, and 8x over hold20, hold30, and
  hold40.

Reason:

- Top winner reached more than 13x price-to-initial before exit.
- Low take-profit may improve realized PnL but cap too much upside.

Backtest mapping:

```bash
--strategy-suite risk-atlas-edge-v4
```

Review gate:

- Compare total PnL after removing top 1, 3, and 5 winners.
- Prefer the lowest take-profit threshold that does not destroy net PnL.
