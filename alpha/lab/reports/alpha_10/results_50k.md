# Alpha 10 50K Backtest Results

This is the completed 50K `alpha_10` replay generated on 2026-05-18.

## Range Note

The originally planned anchored 50K range was:

```text
25,090,165 to 25,140,164
```

That source range failed during block processing because the local source could
not load block `25,117,918`:

```text
failed to process block 25117918: No header for block 25117918
```

The completed 50K run below uses the nearest fully available 50K cached window:

```text
25,067,915 to 25,117,914
```

## Generated Data

Source token range:

| Metric | Value |
| --- | ---: |
| Source run | `run-2` |
| Start block | 25,067,915 |
| End block | 25,117,914 |
| Blocks processed | 50,000 |
| Processed-block cache hits | 50,000 |
| Processed-block cache misses | 0 |
| Transactions scanned | 13,860,875 |
| Transactions processed | 13,475,274 |
| Token update reports | 858,887 |
| Tracked tokens | 5,158 |
| Tracked pools | 5,679 |

Risk Atlas replay:

| Metric | Value |
| --- | ---: |
| Replay run ID | `risk-atlas-alpha10-50k-25067915-25117914-20260518` |
| Source ref | `run-2` |
| Token count | 5,158 |
| Pool count | 5,679 |
| Scam labels | 3,436 |
| Direct LP feature rows | 1,744 |
| Active observations | 254,950 |
| Status | completed |

The previous 20K `risk-atlas-run-2` rows were preserved as
`risk-atlas-run-2-20k-20260516` before the server export reused
`risk-atlas-run-2`.

## Backtest Run

```text
run id: alpha10-risk-atlas-50k-25067915-25117914-20260518
result set: historical-25067915-25117914
replay run id: risk-atlas-alpha10-50k-25067915-25117914-20260518
strategy suite: alpha-10-risk-atlas
execution delay: 1 block
status: completed
```

Command:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id alpha10-risk-atlas-50k-25067915-25117914-20260518 \
  --replay-run-id risk-atlas-alpha10-50k-25067915-25117914-20260518 \
  --strategy-suite alpha-10-risk-atlas \
  --from-block 25067915 \
  --to-block 25117914 \
  --execution-delay-blocks 1
```

Backtest UI:

```text
http://127.0.0.1:40019/eth/trade/backtests/historical/25067915-25117914/
http://127.0.0.1:40019/eth/trade/backtests/historical/25067915-25117914/strategies/alpha10-10-v2-hold15-retry3/
```

Aggregate result set:

| Metric | Value |
| --- | ---: |
| Trades | 14,616 |
| Closed | 14,283 |
| Open | 297 |
| Failed | 127 |
| Realized PnL ETH | 47.035761 |
| Unrealized PnL ETH | -1.559825 |
| Total PnL ETH | 45.475936 |
| ROI percent | 31.190628 |

## Strategy Results

| Strategy | Trades | Closed | Open | Failed | Realized ETH | Unrealized ETH | Total ETH | Losers | Loss ETH |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `alpha10-01-baseline-hold15-buy-confirm` | 1,566 | 1,431 | 99 | 48 | 10.666065 | -0.854258 | 9.811806 | 517 | -3.152063 |
| `alpha10-02-v2-hold15-buy-confirm` | 1,450 | 1,429 | 21 | 8 | 10.650772 | -0.074258 | 10.576514 | 403 | -2.368873 |
| `alpha10-03-v2-hold20` | 1,450 | 1,427 | 23 | 9 | 1.960668 | -0.101690 | 1.858978 | 460 | -0.746504 |
| `alpha10-04-v2-hold30` | 1,450 | 1,426 | 24 | 9 | 2.599903 | -0.046485 | 2.553418 | 464 | -0.791862 |
| `alpha10-05-v2-hold40` | 1,450 | 1,425 | 25 | 9 | 2.660984 | -0.047350 | 2.613633 | 468 | -0.838904 |
| `alpha10-06-v2-hold30-tp3x` | 1,450 | 1,427 | 23 | 10 | 1.935849 | -0.101690 | 1.834159 | 454 | -0.717065 |
| `alpha10-07-v2-hold30-tp5x` | 1,450 | 1,427 | 23 | 9 | 2.209888 | -0.101690 | 2.108198 | 459 | -0.756360 |
| `alpha10-08-v2-hold30-sl70-tp3x` | 1,450 | 1,429 | 21 | 9 | 1.893997 | -0.087103 | 1.806894 | 458 | -0.714128 |
| `alpha10-09-v2-hold30-sl85-tp3x` | 1,450 | 1,433 | 17 | 8 | 1.805090 | -0.071042 | 1.734048 | 466 | -0.688067 |
| `alpha10-10-v2-hold15-retry3` | 1,450 | 1,429 | 21 | 8 | 10.652545 | -0.074258 | 10.578287 | 403 | -2.367100 |

## Read

The best 50K candidate is `alpha10-10-v2-hold15-retry3` at
`+10.578287 ETH`. It beats `alpha10-02-v2-hold15-buy-confirm` by only
`0.001773 ETH`, so the retry change is not the main source of edge.

The material improvement versus the all-protocol baseline is still V2-only
selection:

| Comparison | Total ETH | Open | Failed | Loss ETH |
| --- | ---: | ---: | ---: | ---: |
| All protocol baseline | 9.811806 | 99 | 48 | -3.152063 |
| V2 hold15 buy-confirm | 10.576514 | 21 | 8 | -2.368873 |
| V2 hold15 retry3 | 10.578287 | 21 | 8 | -2.367100 |

Longer hold windows and price exits reduce loss size, but they give up too much
upside in this range. They remain diagnostic policies, not leaders.

## Loss Scan

Best candidate command:

```bash
cargo run -p eth_alpha_lab --bin eth_alpha_lab -- \
  losing-trades \
  --result-set historical-25067915-25117914 \
  --strategy alpha10-10-v2-hold15-retry3 \
  --limit 20
```

Summary:

| Metric | Value |
| --- | ---: |
| Losing trades | 403 |
| Total loss ETH | -2.367100 |
| Sample traces | 20 |

Main loss buckets:

| Signal | Trades | Loss ETH |
| --- | ---: | ---: |
| mark_to_market_below_entry_before_exit | 388 | -2.251728 |
| lp_approval_visible_by_sell_submit | 375 | -2.233292 |
| lp_approval_after_entry | 219 | -0.593935 |
| sell_submitted_after_liquidity_removal | 175 | -1.747806 |
| lp_approval_in_buy_confirm_block | 173 | -1.728115 |
| gas_material_to_loss | 103 | -0.030848 |
| lp_approval_lead_lte_1_block | 43 | -0.405891 |
| sell_confirmation_raced_liquidity_removal | 32 | -0.322309 |
| max_hold_exit_loser | 24 | -0.130704 |
| no_pool_risk_event_before_loss | 11 | -0.045050 |

The remaining loss surface is still mostly the direct-removal race: LP approval
is visible by sell submission, but the sell either submits after removal or
confirms in the same race window. Non-LP unexplained losses are small in this
V2-only policy.

## Concentration And Validation

For `alpha10-10-v2-hold15-retry3`:

| Metric | Value |
| --- | ---: |
| Winner PnL ETH | 12.945387 |
| Total PnL ETH | 10.578287 |
| Winning trades | 1,047 |
| Losing trades | 403 |
| Top 1 winner share of total PnL | 2.8323% |
| Top 3 winners share of total PnL | 4.8289% |
| Top 5 winners share of total PnL | 5.8959% |
| Largest winner ETH | 0.299614 |

Quick validation passed:

| Metric | Value |
| --- | ---: |
| Checks | 20 |
| Passed | 20 |
| Warnings | 0 |
| Failures | 0 |
| Blocked | 0 |

## Current Recommendation

Use `alpha10-10-v2-hold15-retry3` as the current alpha_10 leader, but treat the
retry benefit as negligible. The robust policy choice from both 20K and this
50K result is:

```text
V2-only + hold15 + buy-confirm LP approval deferral + LP approval/removal exits
```

The next strategy-design question is not another hold-window or take-profit
sweep. It is whether the direct-removal race can be reduced before entry or by
a stricter same-confirmation-block quarantine without giving up the launch
winners that produce the edge.
