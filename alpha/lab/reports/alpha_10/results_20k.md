# Alpha 10 20K Backtest Results

This is the first actual `alpha_10` backtest run persisted into the Asena
backtest store.

## Run

```text
run id: alpha10-risk-atlas-run-2-20k-20260517
result set: historical-25090165-25110164
replay run id: risk-atlas-run-2
block range: 25,090,165 to 25,110,164
strategy suite: alpha-10-risk-atlas
execution delay: 1 block
status: completed
```

Command:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id alpha10-risk-atlas-run-2-20k-20260517 \
  --replay-run-id risk-atlas-run-2 \
  --strategy-suite alpha-10-risk-atlas \
  --from-block 25090165 \
  --to-block 25110164 \
  --execution-delay-blocks 1
```

Backtest UI:

```text
http://127.0.0.1:40019/eth/trade/backtests/historical/25090165-25110164/
http://127.0.0.1:40019/eth/trade/backtests/historical/25090165-25110164/strategies/alpha10-10-v2-hold15-retry3/
```

Run metadata:

| Metric | Value |
| --- | ---: |
| Events processed | 92,052 |
| Positions | 5,590 |
| Open positions | 96 |
| Strategies | 10 |
| Reports generated | 22,216 |
| Confirmed reports | 11,066 |
| Failed reports | 42 |

## Results

| Strategy | Trades | Closed | Open | Failed | Realized ETH | Unrealized ETH | Total ETH | Losers | Loss ETH |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `alpha10-01-baseline-hold15-buy-confirm` | 604 | 548 | 44 | 12 | 3.780894 | -0.383726 | 3.397168 | 229 | -1.491478 |
| `alpha10-02-v2-hold15-buy-confirm` | 554 | 547 | 6 | 1 | 3.782454 | 0.016274 | 3.798728 | 179 | -1.089918 |
| `alpha10-03-v2-hold20` | 554 | 548 | 5 | 1 | 0.516400 | 0.016417 | 0.532818 | 197 | -0.293542 |
| `alpha10-04-v2-hold30` | 554 | 548 | 5 | 1 | 0.606276 | 0.016417 | 0.622694 | 199 | -0.322208 |
| `alpha10-05-v2-hold40` | 554 | 548 | 5 | 1 | 0.531946 | 0.016417 | 0.548364 | 201 | -0.337232 |
| `alpha10-06-v2-hold30-tp3x` | 554 | 549 | 4 | 1 | 0.633006 | -0.017149 | 0.615857 | 195 | -0.287833 |
| `alpha10-07-v2-hold30-tp5x` | 554 | 548 | 5 | 1 | 0.760557 | 0.016417 | 0.776974 | 196 | -0.297160 |
| `alpha10-08-v2-hold30-sl70-tp3x` | 554 | 550 | 3 | 1 | 0.670635 | -0.012562 | 0.658073 | 195 | -0.263703 |
| `alpha10-09-v2-hold30-sl85-tp3x` | 554 | 552 | 1 | 1 | 0.629964 | -0.010064 | 0.619900 | 199 | -0.263324 |
| `alpha10-10-v2-hold15-retry3` | 554 | 547 | 6 | 1 | 3.784747 | 0.016274 | 3.801021 | 179 | -1.087625 |

## Read

The best 20K candidate is `alpha10-10-v2-hold15-retry3` at `+3.801021 ETH`.
It only improves the V2 hold15 baseline by about `0.002293 ETH`, so the retry
change is not the real source of edge in this window. The larger effect is the
V2-only universe: it drops open exposure from 44 to 6 positions and failed
positions from 12 to 1 while improving total PnL versus the all-protocol
baseline.

The hold-window and price-exit variants underperform the hold15 V2 baseline in
this 20K range. They reduce loser loss ETH materially, but they also surrender
too much upside. That makes them useful diagnostic policies, not current
leaders.

## Best Candidate Loss Scan

Command:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -p eth_alpha_lab --bin eth_alpha_lab -- \
  losing-trades \
  --result-set historical-25090165-25110164 \
  --strategy alpha10-10-v2-hold15-retry3 \
  --limit 20
```

Summary:

| Metric | Value |
| --- | ---: |
| Losing trades | 179 |
| Total loss ETH | -1.087625 |
| Sample traces | 20 |

Main remaining loss buckets:

| Signal | Trades | Loss ETH |
| --- | ---: | ---: |
| mark_to_market_below_entry_before_exit | 174 | -1.068915 |
| lp_approval_visible_by_sell_submit | 171 | -1.064035 |
| sell_submitted_after_liquidity_removal | 83 | -0.829987 |
| lp_approval_in_buy_confirm_block | 84 | -0.830154 |
| lp_approval_after_entry | 91 | -0.246968 |
| lp_approval_lead_lte_1_block | 21 | -0.204035 |
| sell_confirmation_raced_liquidity_removal | 18 | -0.181133 |
| no_pool_risk_event_before_loss | 4 | -0.010503 |

This confirms the best 20K policy still loses mainly when the warning arrives
too late for a next-block exit or when the LP approval was already in the buy
confirmation block. V2-only selection removes most non-LP unexplained loss in
this range, but it does not solve the direct-removal race.

## 50K Status

No 50K Risk Atlas row-observation replay run is available in the current store.
The only runnable Risk Atlas replay source confirmed here is `risk-atlas-run-2`
over 20,000 blocks. The same `alpha-10-risk-atlas` suite can be run on 50K as
soon as a completed Risk Atlas run exists for that exact block range.
