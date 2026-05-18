# Iteration 08 - Retry And Fee Guard

Question:

```text
Do failed exits and high gas explain enough loss to justify retry and fee caps?
```

Policy change:

- Add exit retry interval of 1 block.
- Allow up to 3 failed exit retries.
- In live design, add max priority fee and max total ETH fee per trade.

Reason:

- Baseline had 12 failed trades.
- Loss scan found gas was material in 51 losing trades, though total gas-driven
  loss was small relative to scam losses.

Backtest mapping:

```bash
--exit-retry-interval-blocks 1 --max-exit-retries 3
```

Review gate:

- Retries must reduce failed-loss ETH without increasing removal-race losses.
- Fee caps are live guardrails and should not be backfilled as if they were
  historical signals unless fee data is part of the execution model.
