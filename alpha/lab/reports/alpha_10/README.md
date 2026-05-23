# Alpha 10 Risk Atlas Strategy Design

This package records ten strategy-design iterations for the Risk Atlas Direct
LP Warning work. It is a research and review package, not production strategy
code.

## Scope

Primary evidence comes from:

- Risk Atlas run `risk-atlas-run-2`
- source token range run `run-2`
- block range `25,090,165` to `25,110,164`
- Direct LP Warning page:
  `http://127.0.0.1:40019/eth/tokens/analytics/risk-atlas/direct-lp-warning/?run_id=risk-atlas-run-2`
- related backtest strategy:
  `snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold`
- latest historical result set inspected:
  `historical-25090165-25110164`
- alpha_10 run id:
  `alpha10-risk-atlas-50k-25067915-25117914-20260518`

This folder records the actual 20K and completed 50K backtests and the design
questions behind the ten strategy iterations.

## Current Baseline

The baseline policy is:

```text
snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold
```

Rules:

- buy eligible pools once;
- block new entries after mined LP approval is visible;
- sell on mined LP approval unless the approval first appears in the buy
  confirmation block;
- sell on direct LP liquidity removal;
- force exit after 15 active pool-update observations.

Latest alpha_10 result:

| Metric | Value |
| --- | ---: |
| Run ID | `alpha10-risk-atlas-50k-25067915-25117914-20260518` |
| Result set | `historical-25067915-25117914` |
| Strategies | 10 |
| Best strategy | `alpha10-10-v2-hold15-retry3` |
| Best total PnL ETH | 10.578287 |
| Best realized PnL ETH | 10.652545 |
| Best open positions | 21 |
| Best failed positions | 8 |

The best 20K candidate is only slightly ahead of
`alpha10-02-v2-hold15-buy-confirm`, and the same is true in the completed 50K
run. The material current finding is V2-only universe selection, not exit
retries.

## Package Contents

| File | Purpose |
| --- | --- |
| `evidence.md` | Source facts from Risk Atlas, Asena, and lab scans. |
| `results_20k.md` | Actual persisted alpha_10 backtest command, URLs, and result table. |
| `results_50k.md` | Completed 50K data generation, backtest results, validation, and loss scan. |
| `questions.md` | Strategy-neutral and policy-specific questions to answer before trusting a policy. |
| `loss_review.md` | What the losing pools show, and which signals might prevent losses. |
| `profit_review.md` | What the profitable pools show, and which signals might improve upside capture. |
| `pnl_validation_50_pool_sample.md` | 50-pool event chronology, no-leakage, fill, gas, and PnL audit for the leading 50K strategy. |
| `iterations/` | Ten concrete policy iterations with questions, changes, and review gates. |
| `backtest_50k_checklist.md` | Exact commands and validation checklist for the later 50K review. |

## Promotion Rule

A policy is not promoted from this package unless the 50K result proves all of
these:

1. It beats the baseline on total PnL and realized PnL.
2. It does not improve PnL only by increasing open exposure.
3. It reduces or explains the largest loss buckets.
4. It preserves no-leakage timing under the current next-block execution model.
5. It survives top-winner concentration, worst-loser, and random-trade review.

## 50K Range Note

The exact anchored range `25,090,165` to `25,140,164` could not be processed
because block `25,117,918` was unavailable from the local source. The completed
50K result uses the nearest fully available cached 50K range:

```text
25,067,915 to 25,117,914
```
