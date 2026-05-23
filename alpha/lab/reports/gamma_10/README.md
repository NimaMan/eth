# Gamma 10 Risk Atlas Strategy Design

This package records the next ten policy iterations after `alpha_10`. The goal
is not another broad sweep. The goal is to isolate the direct LP race that still
explains most losses in the 50K alpha_10 leader, while preserving the launch
winners that make the policy profitable.

## Source Evidence

Primary page inspected:

```text
http://127.0.0.1:40019/eth/tokens/analytics/risk-atlas/direct-lp-warning/?run_id=risk-atlas-run-2
```

At gamma_10 inspection time, `risk-atlas-run-2` points to the current 50K
Risk Atlas run:

| Field | Value |
| --- | ---: |
| Source ref | `run-2` |
| Start block | 25,067,915 |
| End block | 25,117,914 |
| Tokens | 5,161 |
| Pools | 5,716 |
| Eligible pools | 3,770 |
| Scam labels | 3,441 |
| Direct LP feature rows | 1,749 |
| Active observations | 255,244 |

The strongest prior strategy is:

```text
alpha10-10-v2-hold15-retry3
```

on result set:

```text
historical-25067915-25117914
```

## Baseline Read

Alpha_10 proved the main robust improvement:

```text
UNISWAP-V2 only + LP gate + buy-confirm LP approval deferral + hold15
```

Retrying exits improved the 50K result by only `0.001773 ETH` versus the
non-retry V2 hold15 variant, so retry is not the core edge.

The remaining loss surface is direct LP race exposure:

| Loss bucket for `alpha10-10-v2-hold15-retry3` | Trades | Loss ETH |
| --- | ---: | ---: |
| `mark_to_market_below_entry_before_exit` | 388 | -2.251728 |
| `lp_approval_visible_by_sell_submit` | 375 | -2.233292 |
| `sell_submitted_after_liquidity_removal` | 175 | -1.747806 |
| `lp_approval_in_buy_confirm_block` | 173 | -1.728115 |
| `lp_approval_lead_lte_1_block` | 43 | -0.405891 |

The profit side is not dominated by only one outlier. In the same 50K leader,
top-five winners are under 6% of total PnL, so the policy can tolerate targeted
loss reduction if it does not remove the same-confirm launch winners wholesale.

## Implemented Suite

The Rust backtester suite is:

```text
gamma-10-risk-atlas
```

It deploys these ten strategies:

| # | Strategy | Main question |
| ---: | --- | --- |
| 1 | `gamma10-01-v2-hold15-immediate-lp-exit` | What is the cost of treating buy-confirm LP approval as an immediate exit? |
| 2 | `gamma10-02-v2-hold5-buy-confirm` | Does a very short survival window avoid early removals? |
| 3 | `gamma10-03-v2-hold8-buy-confirm` | Does hold8 keep enough launch upside? |
| 4 | `gamma10-04-v2-hold10-buy-confirm` | Does hold10 improve loss timing without capping too much upside? |
| 5 | `gamma10-05-v2-hold12-buy-confirm` | Is hold12 a better race/return balance than hold15? |
| 6 | `gamma10-06-v2-hold15-buy-confirm` | Reproduce the alpha_10 structural baseline under gamma names. |
| 7 | `gamma10-07-v2-hold20-buy-confirm` | Test whether alpha_10 longer holds failed mainly because deferral was disabled. |
| 8 | `gamma10-08-v2-hold12-retry3` | Test retry value on the shorter likely leader. |
| 9 | `gamma10-09-v2-hold15-retry3` | Reproduce the alpha_10 leader under gamma names. |
| 10 | `gamma10-10-v2-hold15-sl85-tp5x` | Test whether drawdown/profit caps help once buy-confirm deferral remains enabled. |

## Backtest Command

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id gamma10-risk-atlas-50k-25067915-25117914-20260518 \
  --replay-run-id risk-atlas-alpha10-50k-25067915-25117914-20260518 \
  --strategy-suite gamma-10-risk-atlas \
  --from-block 25067915 \
  --to-block 25117914 \
  --execution-delay-blocks 1
```

Results will be recorded in `results_50k.md` after the persisted replay.

## Completed Result

The 50K run completed on 2026-05-19 local time:

| Metric | Value |
| --- | ---: |
| Run ID | `gamma10-risk-atlas-50k-25067915-25117914-20260518` |
| Result set | `historical-25067915-25117914` |
| Strategy suite | `gamma-10-risk-atlas` |
| Strategies | 10 |
| Trades | 14,500 |
| Best total PnL | `gamma10-07-v2-hold20-buy-confirm` |
| Best total PnL ETH | 13.239751 |
| Best realized PnL ETH | 15.842230 |
| Best open positions | 266 |
| Best failed positions | 10 |

Read: hold20 with buy-confirm LP approval deferral is the strongest gamma_10
candidate by total and realized PnL, but it carries much more open exposure
than the hold15 retry baseline. It needs open-position and final-marking review
before promotion.
