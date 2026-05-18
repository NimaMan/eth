# Evidence

## Risk Atlas Run

Risk Atlas run `risk-atlas-run-2` covers:

| Field | Value |
| --- | ---: |
| Chain | ethereum |
| Source run | `run-2` |
| Start block | 25,090,165 |
| End block | 25,110,164 |
| Block count | 20,000 |
| Tokens | 1,884 |
| Pools | 1,899 |
| Eligible pools | 1,443 |
| Scam-labeled pools | 1,280 |
| Active observations | 73,286 |

Eligible pool split:

| Protocol | Eligible pools | Scam pools |
| --- | ---: | ---: |
| UNISWAP-V2 | 1,105 | 1,039 |
| UNISWAP-V4 | 287 | 238 |
| UNISWAP-V3 | 51 | 3 |

Direct LP removals are almost entirely V2 in this window:

```text
573 / 576 direct LP scam pools are UNISWAP-V2.
```

## Direct LP Warning Facts

Direct LP approval is a real warning signal:

| Metric | Value |
| --- | ---: |
| Direct LP approval-timing rows | 638 |
| Pre-removal LP approval rows | 620 |
| No visible pre-removal approval | 18 |
| Approval coverage | 97.18% |
| Latest approval lead P25 | 4 blocks |
| Latest approval lead median | 63 blocks |
| Latest approval lead P90 | 93 blocks |
| Latest approval lead P95 | 378 blocks |

The separate actionability report narrows the eligible direct-LP cohort:

| Window | Count | Interpretation |
| --- | ---: | --- |
| No pre-removal approval | 5 | No mined-chain approval warning |
| One-block window | 58 | Ordering race |
| Two-plus block window | 513 | Clean next-block exit window |

LP approval timing after trading enabled:

| Window | Share |
| --- | ---: |
| Within 1 block | 51.50% |
| Within 2 blocks | 68.32% |
| Within 5 blocks | 82.12% |
| Within 10 blocks | 86.02% |

Design implication: LP approval is both a no-entry feature and an exit cap. It
is not the standalone profit source.

## Active Observation Targets

The model target is:

```text
P(direct LP removal within next N active observations | as-of pool state)
```

| Horizon | Rows | Positives | Positive share |
| --- | ---: | ---: | ---: |
| 1 active observation | 68,588 | 576 | 0.8398% |
| 2 active observations | 68,588 | 1,152 | 1.6796% |
| 3 active observations | 68,588 | 1,727 | 2.5179% |
| 5 active observations | 68,588 | 2,833 | 4.1305% |
| 10 active observations | 68,588 | 5,199 | 7.5800% |

## Current Backtest Evidence

Completed 50K alpha_10 suite result:

```text
source range: run-2
risk atlas replay: risk-atlas-alpha10-50k-25067915-25117914-20260518
result set: historical-25067915-25117914
run id: alpha10-risk-atlas-50k-25067915-25117914-20260518
strategy suite: alpha-10-risk-atlas
best strategy: alpha10-10-v2-hold15-retry3
```

| Metric | Value |
| --- | ---: |
| Source blocks | 50,000 |
| Start block | 25,067,915 |
| End block | 25,117,914 |
| Risk Atlas active observations | 254,950 |
| Strategies | 10 |
| Trades | 14,616 |
| Result-set total PnL ETH | 45.475936 |
| Best total PnL ETH | 10.578287 |
| Best realized PnL ETH | 10.652545 |
| Best unrealized PnL ETH | -0.074258 |
| Best losers | 403 |
| Best loser PnL ETH | -2.367100 |

The planned anchored 50K range `25,090,165` to `25,140,164` failed at block
`25,117,918` because the local source returned `No header for block 25117918`.
The completed 50K run therefore uses the nearest fully available cached range.

The full 50K table is recorded in `results_50k.md`.

Actual alpha_10 suite result:

```text
result set: historical-25090165-25110164
run id: alpha10-risk-atlas-run-2-20k-20260517
strategy suite: alpha-10-risk-atlas
best strategy: alpha10-10-v2-hold15-retry3
```

| Metric | Value |
| --- | ---: |
| Strategies | 10 |
| Events processed | 92,052 |
| Positions | 5,590 |
| Open positions | 96 |
| Best total PnL ETH | 3.801021 |
| Best realized PnL ETH | 3.784747 |
| Best unrealized PnL ETH | 0.016274 |
| Best losers | 179 |
| Best loser PnL ETH | -1.087625 |

The full table is recorded in `results_20k.md`.

Latest inspected related result:

```text
result set: historical-25090165-25110164
run id: risk-atlas-lp-buy-confirm-nofilter-20k-20260517
strategy: snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold
```

| Metric | Value |
| --- | ---: |
| Trades | 604 |
| Closed | 548 |
| Open | 47 |
| Failed | 12 |
| Total PnL ETH | 3.397168 |
| Realized PnL ETH | 3.780894 |
| Unrealized PnL ETH | -0.383726 |
| Winners | 375 |
| Winner PnL ETH | 4.888646 |
| Losers | 229 |
| Loser PnL ETH | -1.491478 |

Earlier report note: the prior V2-only hold15 run showed `+1.596745 ETH`.
The newer all-protocol buy-confirm comparison result is stronger but carries
more open exposure and needs the same validation scrutiny.

## Loss Scan Evidence

Command used:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -p eth_alpha_lab --bin eth_alpha_lab -- \
  losing-trades \
  --result-set historical-25090165-25110164 \
  --strategy snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold \
  --limit 20
```

Summary:

| Metric | Value |
| --- | ---: |
| Losing trades | 229 |
| Total loss ETH | -1.491478 |
| Sample traces | 20 |

Largest repeated loss signals:

| Signal | Trades | Loss ETH |
| --- | ---: | ---: |
| mark_to_market_below_entry_before_exit | 175 | -1.071477 |
| lp_approval_visible_by_sell_submit | 171 | -1.066327 |
| lp_approval_after_entry | 91 | -0.249260 |
| sell_submitted_after_liquidity_removal | 85 | -0.850245 |
| lp_approval_in_buy_confirm_block | 84 | -0.830154 |
| no_pool_risk_event_before_loss | 52 | -0.391805 |
| gas_material_to_loss | 51 | -0.021943 |
| lp_approval_lead_lte_1_block | 21 | -0.204035 |
| sell_confirmation_raced_liquidity_removal | 19 | -0.191011 |
| max_hold_exit_loser | 6 | -0.025261 |
