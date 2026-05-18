# Profit Review

## Profit Surface

Original all-protocol baseline:

| Metric | Value |
| --- | ---: |
| Winning trades | 375 |
| Winning PnL ETH | 4.888646 |
| Losing trades | 229 |
| Losing PnL ETH | -1.491478 |
| Net PnL ETH | 3.397168 |

The winners more than pay for many near-total losers. This makes top-winner
fragility and take-profit design central to the 50K review.

Best actual alpha_10 20K candidate:

| Metric | Value |
| --- | ---: |
| Strategy | `alpha10-10-v2-hold15-retry3` |
| Winning trades | 375 |
| Winning PnL ETH | 4.888646 |
| Losing trades | 179 |
| Losing PnL ETH | -1.087625 |
| Net PnL ETH | 3.801021 |

The winner side did not materially change versus the all-protocol baseline;
the improvement comes from removing weaker non-V2 exposure and reducing failed
exits. The 50K question is therefore whether V2-only preserves the same winner
pool while continuing to reduce loss.

Completed 50K best candidate:

| Metric | Value |
| --- | ---: |
| Strategy | `alpha10-10-v2-hold15-retry3` |
| Winning trades | 1,047 |
| Winning PnL ETH | 12.945387 |
| Losing trades | 403 |
| Losing PnL ETH | -2.367100 |
| Net PnL ETH | 10.578287 |
| Top 1 winner share of total PnL | 2.8323% |
| Top 5 winner share of total PnL | 5.8959% |

The 50K winner side is not dangerously concentrated in the top few trades. The
top-five winners explain under 6% of total PnL for the best candidate, so the
policy is not only a single outlier winner.

## Top Winner Example

Trade `trd_mpa7s7y6_1slr7_w0`:

| Field | Value |
| --- | ---: |
| Entry block | 25,109,506 |
| Exit block | 25,109,543 |
| Entry cost ETH | 0.010000 |
| Exit value ETH | 0.119436 |
| Gas ETH | 0.000039 |
| PnL ETH | 0.109397 |
| ROI percent | 1,093.97 |

As-of snapshots show the visible profit path:

| Block | Current value ETH | Price-to-initial |
| ---: | ---: | ---: |
| 25,109,506 | 0.009559 | 1.0001 |
| 25,109,509 | 0.016404 | 1.7265 |
| 25,109,521 | 0.017403 | 1.8330 |
| 25,109,522 | 0.022058 | 2.3309 |
| 25,109,527 | 0.113156 | 12.4083 |
| 25,109,532 | 0.119436 | 13.1206 |

This winner supports two design questions:

1. Can a take-profit policy capture most of the upside earlier?
2. Does a fixed hold window let rare launch momentum pay for losses better
   than a low take-profit cap?

## Profit-Improving Factors To Test

| Factor | Why it may help | Risk |
| --- | --- | --- |
| Longer hold windows | Earlier sweeps improved from hold5 to hold15. | More open exposure and more scam exposure. |
| Take-profit at 3x or 5x | Converts unrealized spikes into realized PnL. | May cap rare 10x-plus winners. |
| Price acceleration | Captures launch momentum without waiting for LP approval. | Can overfit noisy early pool updates. |
| V2-only universe | Direct LP signal is overwhelmingly V2. | May miss profitable non-V2 pools once route metadata improves. |
| Same-buy-confirm deferral | Lets launch winners run even when approval appears in confirmation block. | Live replication depends on ordering control. |
| Fee caps | Prevents small winners from becoming gas losers. | May skip urgent exits in high-fee blocks. |

## Profit Review Questions For 50K

1. Are top 10 winners responsible for most of net PnL?
2. Does PnL remain positive if the top 1, 3, and 5 winners are removed?
3. Do winners cluster in same-confirmation-block LP approval cases?
4. Do winners exit by LP approval, max hold, take profit, or open marking?
5. What is the best take-profit threshold that preserves most top-winner PnL?
6. Does V2-only improve risk-adjusted return or only reduce trade count?
