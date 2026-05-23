# Gamma 10 50K Backtest Results

This is the completed 50K `gamma_10` replay generated on 2026-05-18 and
completed after midnight local time on 2026-05-19.

## Backtest Run

```text
run id: gamma10-risk-atlas-50k-25067915-25117914-20260518
result set: historical-25067915-25117914
replay run id: risk-atlas-alpha10-50k-25067915-25117914-20260518
strategy suite: gamma-10-risk-atlas
execution delay: 1 block
status: completed
```

Command:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id gamma10-risk-atlas-50k-25067915-25117914-20260518 \
  --replay-run-id risk-atlas-alpha10-50k-25067915-25117914-20260518 \
  --strategy-suite gamma-10-risk-atlas \
  --from-block 25067915 \
  --to-block 25117914 \
  --execution-delay-blocks 1
```

Backtest UI after run:

```text
http://127.0.0.1:40019/eth/trade/backtests/historical/25067915-25117914/
```

## Aggregate Result

| Metric | Value |
| --- | ---: |
| Strategies | 10 |
| Trades | 14,500 |
| Closed | 12,897 |
| Open | 1,408 |
| Failed | 195 |
| Realized PnL ETH | 91.752807 |
| Unrealized PnL ETH | -14.978137 |
| Total PnL ETH | 76.774670 |

| Strategy | Trades | Closed | Open | Failed | Realized ETH | Unrealized ETH | Total ETH | Losers | Loss ETH |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `gamma10-07-v2-hold20-buy-confirm` | 1,450 | 1,174 | 266 | 10 | 15.842230 | -2.602479 | 13.239751 | 471 | -3.072545 |
| `gamma10-09-v2-hold15-retry3` | 1,450 | 1,246 | 196 | 8 | 12.444551 | -1.904258 | 10.540293 | 404 | -2.375566 |
| `gamma10-06-v2-hold15-buy-confirm` | 1,450 | 1,245 | 196 | 9 | 12.452598 | -1.914258 | 10.538340 | 404 | -2.377518 |
| `gamma10-10-v2-hold15-sl85-tp5x` | 1,450 | 1,256 | 188 | 6 | 12.109705 | -1.833611 | 10.276094 | 413 | -2.356778 |
| `gamma10-08-v2-hold12-retry3` | 1,450 | 1,280 | 162 | 8 | 9.689808 | -1.585764 | 8.104044 | 376 | -2.107329 |
| `gamma10-05-v2-hold12-buy-confirm` | 1,450 | 1,279 | 162 | 9 | 9.697740 | -1.595764 | 8.101976 | 376 | -2.109397 |
| `gamma10-04-v2-hold10-buy-confirm` | 1,450 | 1,302 | 138 | 10 | 7.903706 | -1.411602 | 6.492103 | 353 | -1.867652 |
| `gamma10-03-v2-hold8-buy-confirm` | 1,450 | 1,326 | 114 | 10 | 6.288151 | -1.183466 | 5.104685 | 320 | -1.555888 |
| `gamma10-02-v2-hold5-buy-confirm` | 1,450 | 1,373 | 70 | 7 | 3.459144 | -0.713466 | 2.745679 | 275 | -1.060891 |
| `gamma10-01-v2-hold15-immediate-lp-exit` | 1,450 | 1,416 | 25 | 9 | 1.865175 | -0.233469 | 1.631705 | 459 | -0.705493 |

## Read

`gamma10-07-v2-hold20-buy-confirm` is the total-PnL and realized-PnL leader:

| Metric | Value |
| --- | ---: |
| Total PnL ETH | 13.239751 |
| Realized PnL ETH | 15.842230 |
| Unrealized PnL ETH | -2.602479 |
| Open positions | 266 |
| Failed positions | 10 |
| Losers | 471 |
| Loss ETH | -3.072545 |

This beats the hold15 retry family on PnL, but not cleanly on risk. The
improvement comes with much larger open exposure than the gamma hold15 retry
baseline:

| Strategy | Total ETH | Realized ETH | Open | Failed | Loss ETH |
| --- | ---: | ---: | ---: | ---: | ---: |
| `gamma10-07-v2-hold20-buy-confirm` | 13.239751 | 15.842230 | 266 | 10 | -3.072545 |
| `gamma10-09-v2-hold15-retry3` | 10.540293 | 12.444551 | 196 | 8 | -2.375566 |

The key design reads are:

1. Same-confirm immediate LP exit is too blunt. It cuts loss ETH to
   `-0.705493`, but total PnL collapses to `1.631705 ETH`.
2. Shorter holds reduce losses and open exposure, but give up too much launch
   upside. Total PnL rises from hold5 to hold20.
3. Hold20 with buy-confirm deferral answers a gamma question: alpha_10's longer
   hold sweep was confounded by disabled buy-confirm deferral.
4. Retry remains a small effect. Hold15 retry beats hold15 no-retry by only
   `0.001953 ETH`.
5. The `sl85/tp5x` diagnostic reduces loss ETH slightly versus hold15, but it
   does not beat hold15 on total PnL.

## Loss Scan For Gamma Leader

Command:

```bash
cargo run -p eth_alpha_lab --bin eth_alpha_lab -- \
  losing-trades \
  --result-set historical-25067915-25117914 \
  --strategy gamma10-07-v2-hold20-buy-confirm \
  --limit 20
```

Summary:

| Metric | Value |
| --- | ---: |
| Losing trades | 471 |
| Total loss ETH | -3.072545 |
| Sample traces | 20 |

Signal buckets:

| Signal | Trades | PnL ETH |
| --- | ---: | ---: |
| `lp_approval_in_buy_confirm_block` | 240 | -2.405922 |
| `lp_approval_after_entry` | 219 | -0.618640 |
| `lp_approval_visible_by_sell_submit` | 210 | -0.564485 |
| `mark_to_market_below_entry_before_exit` | 202 | -0.418430 |
| `gas_material_to_loss` | 100 | -0.020821 |
| `lp_approval_lead_lte_1_block` | 43 | -0.421470 |
| `sell_confirmation_raced_liquidity_removal` | 23 | -0.222867 |
| `max_hold_exit_loser` | 16 | -0.098582 |
| `no_pool_risk_event_before_loss` | 12 | -0.047984 |
| `sell_submitted_after_liquidity_removal` | 4 | -0.039325 |

Exit-reason loss split:

| Exit reason | Trades | PnL ETH |
| --- | ---: | ---: |
| open/unmapped | 247 | -2.429637 |
| `exit.lp_approval` | 208 | -0.544326 |
| `exit.max_hold_active_blocks` | 16 | -0.098582 |

The leader's remaining loss is now mostly open/unmapped same-confirm LP
approval exposure, not confirmed sell-after-removal. That makes open-position
marking and live-exit feasibility the next audit target before promotion.

## Concentration

| Strategy | Winners | Winner PnL ETH | Total PnL ETH | Top 1 ETH | Top 1 Share | Top 5 ETH | Top 5 Share |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `gamma10-07-v2-hold20-buy-confirm` | 979 | 16.312296 | 13.239751 | 0.291050 | 2.1983% | 0.695107 | 5.2501% |
| `gamma10-09-v2-hold15-retry3` | 1,046 | 12.915858 | 10.540293 | 0.299614 | 2.8426% | 0.623681 | 5.9171% |

The gamma leader is not overly dependent on the largest winners.

## Validation

Quick validation for `gamma10-07-v2-hold20-buy-confirm`:

| Metric | Value |
| --- | ---: |
| Checks | 30 |
| Passed | 30 |
| Warnings | 0 |
| Failures | 0 |
| Blocked | 0 |

Validation command:

```bash
cargo run -p eth_alpha_lab --bin eth_alpha_lab -- \
  strategy-validation \
  --result-set historical-25067915-25117914 \
  --strategy gamma10-07-v2-hold20-buy-confirm \
  --profile quick
```

## Current Recommendation

Do not blindly promote `gamma10-07-v2-hold20-buy-confirm` yet. It is the best
gamma_10 candidate by total and realized PnL, and validation passed, but it
materially increases open exposure. The next audit should focus on those 266
open positions and especially the 247 losing open/unmapped traces in the loss
scan.

If that open exposure is acceptable under live marking and execution, hold20
with buy-confirm deferral becomes the new candidate. If not, keep the hold15
retry family as the safer baseline and use gamma_10's result to design a
specific open-exposure exit rule rather than a broader hold sweep.
