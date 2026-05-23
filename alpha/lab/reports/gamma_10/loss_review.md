# Gamma 10 Loss Review

## Main Loss Buckets

For the 50K alpha_10 leader:

| Signal | Trades | Loss ETH | Gamma response |
| --- | ---: | ---: | --- |
| `mark_to_market_below_entry_before_exit` | 388 | -2.251728 | Test `sl85` with buy-confirm deferral still enabled. |
| `lp_approval_visible_by_sell_submit` | 375 | -2.233292 | Separate clean lead from race windows; keep reporting this bucket. |
| `sell_submitted_after_liquidity_removal` | 175 | -1.747806 | Treat confirmed removal mostly as a label, not a recoverable edge. |
| `lp_approval_in_buy_confirm_block` | 173 | -1.728115 | Isolate immediate exit versus deferred hold policies. |
| `lp_approval_lead_lte_1_block` | 43 | -0.405891 | Do not over-credit one-block exits in promotion review. |
| `no_pool_risk_event_before_loss` | 11 | -0.045050 | Small enough to leave outside gamma_10's first scope. |

## What Looks Preventable

The preventable-looking losses are not mostly caused by non-V2 protocol
selection anymore. Alpha_10 already removed that larger issue. The remaining
candidate prevention paths are:

1. exit earlier from the same-confirm LP approval cohort;
2. shorten the max hold so sell submission starts before common removal windows;
3. add a drawdown exit that reacts before the LP removal/approval race ends;
4. avoid claiming liquidity-removal exits are actionable when removal is
   already mined before sell submission.

## What Does Not Look Solved Yet

The current chain-only features did not show a strong pre-buy separator inside
the buy-confirm LP approval branch. The policy may ultimately need mempool or
transaction-ordering information to improve this branch without losing too many
winners.

## Gamma 10 Result Update

The 50K gamma leader is:

```text
gamma10-07-v2-hold20-buy-confirm
```

It improves total PnL but increases the loss surface:

| Metric | Value |
| --- | ---: |
| Losing trades | 471 |
| Total loss ETH | -3.072545 |
| Open/unmapped losing trades | 247 |
| Open/unmapped loss ETH | -2.429637 |
| LP approval exit losing trades | 208 |
| LP approval exit loss ETH | -0.544326 |

The important change versus alpha_10 is that the leader's risk is now dominated
by open/unmapped same-confirm LP approval exposure. The next loss audit should
inspect whether those opens are acceptable live marks, missed exits, or a
strategy rule problem.
