# Direct LP Liquidity Removal Feature Summary

Source run: `run-2`

Rows are one pool each for `direct_lp_liquidity_removal`. The label
block is the liquidity-removal/reserve-drain block. `as_of_block` is
`label_block - 1`; activity features are filtered to that block.
Current LP and network fields are marked with scope columns because
the token-server summary is not fully time-sliced yet.

## Source Progress

- Range: `24994815..25094814`
- Status at export: `completed`
- Blocks processed at export: `100000` / `100000`
- Rows exported: `3565`
- Window rows exported: `10736`
- Control window rows exported: `2970`

## Time To Removal

| Bucket | Count |
| --- | ---: |
| `early_11_to_100` | 2339 |
| `delayed_101_to_1000` | 646 |
| `late_gt_1000` | 297 |
| `fast_1_to_10` | 250 |
| `same_block` | 33 |

## LP Approval Timing

| Any Pre-Removal Approval | Count |
| --- | ---: |
| `true` | 3390 |
| `false` | 175 |

| Pre-Removal Approval | Count |
| --- | ---: |
| `true` | 3161 |
| `false` | 404 |

| Same-Block Approval | Count |
| --- | ---: |
| `false` | 3288 |
| `true` | 277 |

| First Approval Lead To Removal | Count |
| --- | ---: |
| `early_11_to_100` | 2131 |
| `fast_1_to_10` | 634 |
| `delayed_101_to_1000` | 443 |
| `late_gt_1000` | 182 |
| `unknown` | 114 |
| `same_block` | 61 |

| Last Pre-Removal Approval Lead | Count |
| --- | ---: |
| `early_11_to_100` | 2001 |
| `fast_1_to_10` | 960 |
| `delayed_101_to_1000` | 367 |
| `unknown` | 175 |
| `late_gt_1000` | 62 |

| Metric | Count | Min | P25 | Median | P75 | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Trading Enabled To Removal | 3565 | -21 | 42 | 71 | 106 | 57057 |
| First Pre-Removal LP Approval To Removal | 3390 | 1 | 21 | 62 | 86 | 50249 |
| Last Pre-Removal LP Approval To Removal | 3390 | 1 | 6 | 56 | 79 | 49580 |
| Trading Enabled To First Pre-Removal LP Approval | 3390 | -1865 | 1 | 1 | 4 | 57047 |
| Trading Enabled To Last Pre-Removal LP Approval | 3390 | -114 | 1 | 2 | 7 | 57047 |

Negative trading-to-approval values mean the LP approval was already
visible before the selected `trading_enabled_block`.

## LP Feature Scope

| Scope | Count |
| --- | ---: |
| `current_lp_state_has_pre_removal_approval` | 3161 |
| `current_lp_state_approval_same_block_as_removal` | 277 |
| `current_lp_state_no_approval_observed` | 111 |
| `current_lp_state_approval_after_removal` | 16 |

## Confidence

| Confidence | Count |
| --- | ---: |
| `verified` | 3565 |

## Protocols

| Protocol | Count |
| --- | ---: |
| `UNISWAP-V2` | 3451 |
| `UNISWAP-V3` | 107 |
| `PANCAKESWAP-V2` | 3 |
| `SUSHISWAP-V3` | 3 |
| `SUSHISWAP-V2` | 1 |

## Network Feature Scope

| Scope | Count |
| --- | ---: |
| `current_token_detail_not_time_sliced` | 3565 |

## Feature Coverage

| Field | Rows | Non-Empty | Non-Zero | Non-Empty % |
| --- | ---: | ---: | ---: | ---: |
| Price Ratio At Label | 3565 | 3456 | 3456 | 96.9% |
| Liquidity At Label | 3565 | 3565 | 3456 | 100.0% |
| First Pre-Removal LP Approval | 3565 | 3390 | 3390 | 95.1% |
| Last Approval Owner Is Creator | 3565 | 3390 | 3390 | 95.1% |
| Last Approval Owner Is Current Owner | 3565 | 773 | 773 | 21.7% |
| Pre-Label Activity Tx Count | 3565 | 3565 | 3541 | 100.0% |
| Pre-Label Activity Density | 3565 | 3565 | 3541 | 100.0% |
| Pre-Label Buy/Sell Volume Ratio | 3565 | 2892 | 2887 | 81.1% |
| Current Network Address Count | 3565 | 3565 | 0 | 100.0% |

## Window Rows

| Blocks Before Removal | Count |
| --- | ---: |
| `1` | 3531 |
| `10` | 3312 |
| `50` | 2527 |
| `100` | 963 |
| `500` | 403 |

| LP Approval Seen As Of | Count |
| --- | ---: |
| `true` | 9097 |
| `false` | 1639 |

## Training Windows

| Row Kind | Count |
| --- | ---: |
| `positive` | 10736 |
| `control` | 2970 |

| Target Removal Within Horizon | Count |
| --- | ---: |
| `true` | 10736 |
| `false` | 2970 |

| Control Horizon Blocks | Count |
| --- | ---: |
| `1` | 673 |
| `10` | 642 |
| `50` | 578 |
| `100` | 559 |
| `500` | 518 |

| Control Reason | Count |
| --- | ---: |
| `no_scam_mechanism_trading_liquid_observed_through_completed_range` | 2970 |

## Horizon Signal Availability

| Row Kind | Horizon Blocks | Rows | LP Approval Seen | Approval Seen % | Median Activity Density | Median Tx Last 10 | Median Tx Last 100 | Median Net Buy ETH |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `control` | 1 | 673 | 217 | 32.2% | 0.00127002229 | 0 | 0 | 0.0001061159136 |
| `control` | 10 | 642 | 215 | 33.5% | 0.001141885172 | 0 | 0 | 0.0001144307385 |
| `control` | 50 | 578 | 198 | 34.3% | 0.0009644085703 | 0 | 0 | 0.0001189794981 |
| `control` | 100 | 559 | 195 | 34.9% | 0.0009001478814 | 0 | 0 | 0.00012 |
| `control` | 500 | 518 | 185 | 35.7% | 0.0006959058921 | 0 | 0 | 0.000119530569 |
| `positive` | 1 | 3531 | 3358 | 95.1% | 0.3176470588 | 0 | 23 | 0.7817829827 |
| `positive` | 10 | 3312 | 2767 | 83.5% | 0.3466666667 | 0 | 23 | 0.8064329959 |
| `positive` | 50 | 2527 | 2096 | 82.9% | 0.4098360656 | 4 | 13 | 0.50333 |
| `positive` | 100 | 963 | 630 | 65.4% | 0.06681514477 | 0 | 1 | 0.3630343804 |
| `positive` | 500 | 403 | 246 | 61.0% | 0.007246376812 | 0 | 0 | 0.2136372283 |

## Training Window Feature Coverage

| Field | Rows | Non-Empty | Non-Zero | Non-Empty % |
| --- | ---: | ---: | ---: | ---: |
| Liquidity As Of | 13706 | 13542 | 13472 | 98.8% |
| Price Ratio As Of | 13706 | 13489 | 13489 | 98.4% |
| LP Last Approval As Of | 13706 | 10107 | 10107 | 73.7% |
| LP Approval Owner Is Creator As Of | 13706 | 10107 | 10107 | 73.7% |
| LP Approval Owner Is Current Owner As Of | 13706 | 2827 | 2827 | 20.6% |
| Activity Density As Of | 13706 | 13706 | 13520 | 100.0% |
| Tx Share Last 10 | 13706 | 13520 | 5343 | 98.6% |
| Current Network Address Count | 13706 | 13706 | 0 | 100.0% |

## Next Feature Work

- Add time-sliced LP holder and approval state at `as_of_block`.
- Add time-sliced token-network graph features instead of current detail
  graph counts.
- Add matched-control sampling and class-balance policy after the full
  range build completes.
- Add fully time-sliced liquidity, tax, and authority histories instead of
  current-state fields where noted by scope columns.
