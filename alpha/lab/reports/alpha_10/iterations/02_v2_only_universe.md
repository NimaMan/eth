# Iteration 02 - V2-Only Universe

Question:

```text
Does restricting to UNISWAP-V2 improve risk-adjusted return because direct LP
removal is overwhelmingly V2 in this window?
```

Policy change:

- Keep the baseline LP gate, LP exit, removal exit, and hold15.
- Restrict `allowed_protocols` to `UNISWAP-V2`.

Reason:

- `573 / 576` direct LP scam pools were V2.
- Older V2-only hold15 result was profitable, but weaker than the latest
  all-protocol result.

Backtest mapping:

```bash
--strategy-suite risk-atlas-lp-buy-confirm-block-comparison-uniswap-v2-only
```

Review gate:

- Compare total PnL, open exposure, loser count, and top-winner concentration.
- If all-protocol wins only through unsupported or fragile route metadata, V2
  remains the safer policy.
