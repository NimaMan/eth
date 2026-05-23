# Alpha11 Hold15 Deploy Readiness

This file tracks the readiness criteria for promoting the main Alpha11 strategy
to public real capital.

## Strategy

`alpha11-live-univ2-lp30-pool-update-block-hold15`

## Required Before Public Hold15

- Gate 1 strategy scope and defaults accepted in the Asena Alpha11 readiness
  page.
- Gate 2 live-backtest spine parity accepted.
- Gate 3 chain-facing assumptions reviewed.
- `alpha11-live-univ2-lp30-pool-update-block-hold3-validation` passes the
  pre-live mined-validation gate.
- Production gas-rank readiness passes:
  `../../gates/production_gas_rank/README.md`.
- Receipt worker alerting and operator review are in place.
- Initial bankroll remains capped at `0.555 ETH`.
- Risk Atlas has calibrated any shared Gate 2 init-policy threshold for pool
  age, price / initial, and liquidity. Hold15 must not rely on a hidden
  Alpha11-only price / initial cap.

## Promotion Rule

Hold15 cannot use public real capital until the hold3 mined-validation file has
a passed run with buy and sell receipts, or the operator explicitly records a
different approved validation path.
