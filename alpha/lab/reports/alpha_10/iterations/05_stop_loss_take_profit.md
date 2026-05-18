# Iteration 05 - Stop-Loss Plus Take-Profit

Question:

```text
Can a stop-loss reduce the 175 mark-to-market loser bucket without cutting
launch winners before they accelerate?
```

Policy change:

- Keep LP gate and LP exits.
- Add stop-loss ratios 0.70 and 0.85.
- Pair with take-profit ratios 3x and 5x over hold20, hold30, and hold40.

Reason:

- Loss scan found 175 losing trades with mark-to-market below entry before
  exit.
- Stop-loss must be tested against the profit review because some winners start
  slightly negative.

Backtest mapping:

```bash
--strategy-suite risk-atlas-edge-v5
```

Review gate:

- Stop-loss must reduce loss ETH more than it reduces winner ETH.
- Check whether top winners had early drawdown that would have stopped them out.
