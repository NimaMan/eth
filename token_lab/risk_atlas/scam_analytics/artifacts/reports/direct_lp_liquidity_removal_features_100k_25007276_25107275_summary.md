# Direct LP Liquidity Removal Feature Summary

Source run: `run-1`

Rows are one pool each for `direct_lp_liquidity_removal`. The label
block is the liquidity-removal/reserve-drain block. `as_of_block` is
`label_block - 1`; activity features are filtered to that block.
Current LP and network fields are marked with scope columns because
the token-server summary is not fully time-sliced yet.

## Source Progress

- Range: `25007276..25107275`
- Status at export: `completed`
- Blocks processed at export: `100000` / `100000`
- Rows exported: `3473`
- Window rows exported: `10314`
- Control window rows exported: `2907`

## Time To Removal

| Bucket | Count |
| --- | ---: |
| `early_11_to_100` | 2280 |
| `delayed_101_to_1000` | 575 |
| `fast_1_to_10` | 300 |
| `late_gt_1000` | 280 |
| `same_block` | 38 |

## LP Approval Timing

| Any Pre-Removal Approval | Count |
| --- | ---: |
| `true` | 3328 |
| `false` | 145 |

| Pre-Removal Approval | Count |
| --- | ---: |
| `true` | 3139 |
| `false` | 334 |

| Same-Block Approval | Count |
| --- | ---: |
| `false` | 3258 |
| `true` | 215 |

| First Approval Lead To Removal | Count |
| --- | ---: |
| `early_11_to_100` | 2083 |
| `fast_1_to_10` | 697 |
| `delayed_101_to_1000` | 384 |
| `late_gt_1000` | 164 |
| `unknown` | 109 |
| `same_block` | 36 |

| Last Pre-Removal Approval Lead | Count |
| --- | ---: |
| `early_11_to_100` | 1947 |
| `fast_1_to_10` | 1005 |
| `delayed_101_to_1000` | 305 |
| `unknown` | 145 |
| `late_gt_1000` | 71 |

| Metric | Count | Min | P25 | Median | P75 | Max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Trading Enabled To Removal | 3473 | -21 | 40 | 71 | 99 | 80282 |
| First Pre-Removal LP Approval To Removal | 3328 | 1 | 18 | 62 | 83 | 50249 |
| Last Pre-Removal LP Approval To Removal | 3328 | 1 | 5 | 56 | 78 | 49580 |
| Trading Enabled To First Pre-Removal LP Approval | 3328 | -1865 | 1 | 1 | 3.25 | 67981 |
| Trading Enabled To Last Pre-Removal LP Approval | 3328 | -114 | 1 | 2 | 7 | 67981 |

Negative trading-to-approval values mean the LP approval was already
visible before the selected `trading_enabled_block`.

## LP Feature Scope

| Scope | Count |
| --- | ---: |
| `current_lp_state_has_pre_removal_approval` | 3139 |
| `current_lp_state_approval_same_block_as_removal` | 215 |
| `current_lp_state_no_approval_observed` | 106 |
| `current_lp_state_approval_after_removal` | 13 |

## Confidence

| Confidence | Count |
| --- | ---: |
| `verified` | 3473 |

## Protocols

| Protocol | Count |
| --- | ---: |
| `UNISWAP-V2` | 3365 |
| `UNISWAP-V3` | 99 |
| `PANCAKESWAP-V2` | 3 |
| `PANCAKESWAP-V3` | 3 |
| `SUSHISWAP-V3` | 2 |
| `SUSHISWAP-V2` | 1 |

## Network Feature Scope

| Scope | Count |
| --- | ---: |
| `current_token_detail_not_time_sliced` | 3473 |

## Feature Coverage

| Field | Rows | Non-Empty | Non-Zero | Non-Empty % |
| --- | ---: | ---: | ---: | ---: |
| Price Ratio At Label | 3473 | 3370 | 3370 | 97.0% |
| Liquidity At Label | 3473 | 3473 | 3371 | 100.0% |
| First Pre-Removal LP Approval | 3473 | 3328 | 3328 | 95.8% |
| Last Approval Owner Is Creator | 3473 | 3328 | 3328 | 95.8% |
| Last Approval Owner Is Current Owner | 3473 | 719 | 719 | 20.7% |
| Pre-Label Activity Tx Count | 3473 | 3473 | 3448 | 100.0% |
| Pre-Label Activity Density | 3473 | 3473 | 3448 | 100.0% |
| Pre-Label Buy/Sell Volume Ratio | 3473 | 2770 | 2768 | 79.8% |
| Current Network Address Count | 3473 | 3473 | 0 | 100.0% |

## Window Rows

| Blocks Before Removal | Count |
| --- | ---: |
| `1` | 3434 |
| `10` | 3171 |
| `50` | 2449 |
| `100` | 868 |
| `500` | 392 |

| LP Approval Seen As Of | Count |
| --- | ---: |
| `true` | 8747 |
| `false` | 1567 |

## Training Windows

| Row Kind | Count |
| --- | ---: |
| `positive` | 10314 |
| `control` | 2907 |

| Target Removal Within Horizon | Count |
| --- | ---: |
| `true` | 10314 |
| `false` | 2907 |

| Control Horizon Blocks | Count |
| --- | ---: |
| `1` | 663 |
| `10` | 621 |
| `50` | 566 |
| `100` | 548 |
| `500` | 509 |

| Control Reason | Count |
| --- | ---: |
| `no_scam_mechanism_trading_liquid_observed_through_completed_range` | 2907 |

## Horizon Signal Availability

| Row Kind | Horizon Blocks | Rows | LP Approval Seen | Approval Seen % | Median Activity Density | Median Tx Last 10 | Median Tx Last 100 | Median Net Buy ETH |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `control` | 1 | 663 | 209 | 31.5% | 0.001325004732 | 0 | 0 | 0.0001189052108 |
| `control` | 10 | 621 | 208 | 33.5% | 0.001110431255 | 0 | 0 | 0.0001265616509 |
| `control` | 50 | 566 | 189 | 33.4% | 0.0008974440923 | 0 | 0 | 0.000127468629 |
| `control` | 100 | 548 | 187 | 34.1% | 0.0008309430167 | 0 | 0 | 0.0001275835245 |
| `control` | 500 | 509 | 180 | 35.4% | 0.0007524454477 | 0 | 0 | 0.0001279638553 |
| `positive` | 1 | 3434 | 3291 | 95.8% | 0.3181818182 | 0 | 23 | 0.8023508383 |
| `positive` | 10 | 3171 | 2639 | 83.2% | 0.3493975904 | 0 | 23 | 0.8416134665 |
| `positive` | 50 | 2449 | 2033 | 83.0% | 0.4285714286 | 4 | 13 | 0.5325135363 |
| `positive` | 100 | 868 | 550 | 63.4% | 0.05608537145 | 0 | 0 | 0.4718718 |
| `positive` | 500 | 392 | 234 | 59.7% | 0.01203990938 | 0 | 0 | 0.3631360052 |

## Training Window Feature Coverage

| Field | Rows | Non-Empty | Non-Zero | Non-Empty % |
| --- | ---: | ---: | ---: | ---: |
| Liquidity As Of | 13221 | 13210 | 13127 | 99.9% |
| Price Ratio As Of | 13221 | 13143 | 13143 | 99.4% |
| LP Last Approval As Of | 13221 | 9720 | 9720 | 73.5% |
| LP Approval Owner Is Creator As Of | 13221 | 9720 | 9720 | 73.5% |
| LP Approval Owner Is Current Owner As Of | 13221 | 2625 | 2625 | 19.9% |
| Activity Density As Of | 13221 | 13221 | 13068 | 100.0% |
| Tx Share Last 10 | 13221 | 13068 | 4997 | 98.8% |
| Current Network Address Count | 13221 | 13221 | 0 | 100.0% |

## Next Feature Work

- Add time-sliced LP holder and approval state at `as_of_block`.
- Add time-sliced token-network graph features instead of current detail
  graph counts.
- Add matched-control sampling and class-balance policy after the full
  range build completes.
- Add fully time-sliced liquidity, tax, and authority histories instead of
  current-state fields where noted by scope columns.
