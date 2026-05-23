# Alpha11 Hold15 Deploy Review Summary

- result set: `live-alpha11-live-univ2-lp30-pool-update-block-hold-sweep-chain-sim-20260521-105807`
- strategy: `alpha11-live-univ2-lp30-pool-update-block-hold15`
- status: `running`
- run updated at: `2026-05-21 18:33:04.58996+00`
- submitted orders reviewed: 181

## Current Performance

| metric | value |
| --- | ---: |
| positions | 105 |
| closed | 75 |
| open | 30 |
| winners | 58 |
| losers | 47 |
| total PnL ETH | 0.582543738 |
| realized PnL ETH | 0.749988344 |
| unrealized PnL ETH | -0.167444607 |
| ROI | 55.480% |

## Existing Lab Validation

| metric | value |
| --- | ---: |
| status | validated |
| overall verdict | pass |
| passed checks | 49 |
| warnings | 0 |
| failures | 0 |
| blocked | 0 |

## Existing Lab Assessment

| metric | value |
| --- | ---: |
| profit factor | 3.7135842179010488 |
| top5/net PnL | 62.31% |
| exposure/capital | 28.57% |

| question | assessment | answer |
| --- | --- | --- |
| top_winner_concentration | review | winner concentration is material: top five are 62.31% of net PnL |
| profit_factor | stable | profit factor is 3.7136 |
| loss_tail_concentration | stable | top five losers explain 23.40% of gross loss |
| open_exposure_materiality | review | open exposure is material at 28.57% of deployed capital |
| failed_exit_exposure | review | 1 failed-exit exposure trades need inspection |



## Priority Fee Impact

The current chain-sim PnL includes simulated gas cost, but not the public
priority spend required to rank in the next mined block. The table below
subtracts only additional priority spend from current PnL.

| policy | added priority ETH | PnL after priority ETH |
| --- | ---: | ---: |
| exact next-block top 25 | 0.051346499 | 0.531197239 |
| exact next-block top 10 | 0.068966793 | 0.513576944 |
| fixed 2 gwei tip | 0.053340048 | 0.529203690 |
| fixed 5 gwei tip | 0.133350120 | 0.449193618 |

## Next-Block Rank Calibration

| metric | value |
| --- | ---: |
| median top-25 required tip | 2.000000 gwei |
| p90 top-25 required tip | 2.002826 gwei |
| median top-10 required tip | 2.106444 gwei |
| p90 top-10 required tip | 3.277349 gwei |
| median rank at 2 gwei | 13 |
| p90 rank at 2 gwei | 26 |

## Initial Deployment Reading

- Use `gas_rank_review.csv` as the per-submit source of truth. It records the
  exact next mined block for each simulated submit and the priority fee needed
  for tail/top-50/top-25/top-10/top-5 placement.
- A fixed 2 gwei priority fee is a reasonable first baseline in this sample:
  it ranks near the front of most sampled confirmation blocks while adding less
  priority spend than 5 gwei.
- Top-10 placement should be reserved for exits where slippage or rug timing is
  more expensive than the extra priority. It is not free enough to use blindly
  on every buy.
- Current open exposure makes the headline PnL unstable. Treat open positions
  in `position_reviews.md` as incomplete until they close.
