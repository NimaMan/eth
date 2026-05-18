# Iteration 09 - Launch Momentum Gate

Question:

```text
Can pre-entry launch-quality features select pools with enough upside to pay for
LP and non-LP scam losses?
```

Policy change:

- Add candidate entry filters based on as-of observations:
  - liquidity growth from initial state;
  - price-to-initial greater than 1 after early observations;
  - active observation density;
  - economic buy and sell state.

Reason:

- Profit review shows large winners are launch-momentum trades.
- Loss review shows direct LP warnings alone do not explain all losses.

Implementation status:

- This needs a coded strategy variant or a model-derived gate.
- The 50K review should first report these features for winners and losers
  before hard-coding thresholds.

Review gate:

- Thresholds must be chosen from train/review split, not from the final 50K PnL
  table alone.
- The gate must reduce loser count without deleting the top winner class.
