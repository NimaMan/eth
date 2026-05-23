# Gamma 10 Evidence

## Risk Atlas Direct LP Page

Current `risk-atlas-run-2` page facts:

| Metric | Value |
| --- | ---: |
| Pools | 5,716 |
| Eligible pools | 3,770 |
| Ineligible pools | 1,946 |
| Scam-labeled pools | 3,441 |
| Direct LP feature rows | 1,749 |
| Active observations | 255,244 |
| Active target horizons | 1, 2, 3, 5, 10 |

Direct LP is the dominant scam label mechanism in this run. The page read is
that Direct LP Warning is a risk gate and timing signal, not a standalone trade
selector.

## Direct LP Duration

For labeled direct LP removals with timing:

| Duration from trading enabled to removal | Share |
| --- | ---: |
| <= 10 blocks | 10.05% |
| <= 50 blocks | 28.96% |
| <= 100 blocks | 80.24% |
| <= 500 blocks | 90.74% |
| <= 1000 blocks | 94.88% |

Median duration is 72 blocks, and the mechanism is overwhelmingly V2:

```text
UNISWAP-V2 direct LP removals: 1,509 / 1,526
```

## Alpha 10 Result To Beat

50K leader:

| Metric | Value |
| --- | ---: |
| Strategy | `alpha10-10-v2-hold15-retry3` |
| Result set | `historical-25067915-25117914` |
| Total PnL ETH | 10.578287 |
| Realized PnL ETH | 10.652545 |
| Open positions | 21 |
| Failed positions | 8 |
| Winners | 1,047 |
| Losers | 403 |
| Winner PnL ETH | 12.945387 |
| Loser PnL ETH | -2.367100 |

The V2 hold15 no-retry version produced `10.576514 ETH`, so retry is not the
main edge.

## Entry-Window LP Approval Cohort

The `lp_approval_entry_age` scan found that, in the inspected entry-window
cohort for the alpha_10 leader:

| Metric | Value |
| --- | ---: |
| Entry-window trades | 297 |
| Share of all trades | 53.61% |
| PnL ETH | 3.436195 |
| Share of total PnL | 90.40% |
| Wins / losses | 213 / 84 |
| LP approval visible at buy confirmation | 297 |
| LP approval visible at buy submission | 0 |

All 84 entry-window losses are cases where direct removal happened before or at
confirmed exit. This is why gamma_10 focuses on survival windows and
same-confirm handling instead of generic submit-visible feature gates.

## Submit-Visible Feature Contrast

The inspected winners and losers had flat submit-visible features:

| Feature | Winner median | Loser median |
| --- | ---: | ---: |
| Submit liquidity | 1.0 | 1.0 |
| Submit tx count | 1 | 1 |
| Submit buy volume | 0 | 0 |
| Submit sell volume | 0 | 0 |
| Submit bribe | 0 | 0 |
| Submit price / initial | 1.0 | 1.0 |
| Submit buy tax | 0 | 0 |
| Submit sell tax | 0 | 0 |

This does not prove there is no entry signal, but it means the first gamma_10
policies should not pretend there is a clean pre-buy filter already visible in
the current feature set.
