# Iteration 01 - Baseline Hold15 Buy-Confirm

Policy:

```text
snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold
```

Question:

```text
Does deferring buy-confirmation-block LP approval to max hold preserve launch
momentum without obvious information leakage?
```

Known evidence:

- 604 trades, 548 closed, 47 open, 12 failed.
- Total PnL was `+3.397168 ETH`.
- Loss scan found 84 losing trades with LP approval in the buy-confirm block.

Backtest mapping:

```bash
--strategy-suite risk-atlas-lp-buy-confirm-block-comparison
```

Review gate:

- Compare against immediate LP exit in the same suite.
- Report same-confirmation-block LP approval PnL separately.
- Require validation of buy submission before LP approval visibility.
