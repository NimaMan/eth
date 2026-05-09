# Strategy Analysis

This folder captures analysis contracts for designing trading strategies from
historical token range runs and live token tracker state. Safety is one input,
but the broader goal is to understand launch behavior, winner distribution,
scam/risk rates, liquidity quality, and cohorts that are useful for alpha.

## Launch Stats Card

The launch stats card should use the same component for historical ranges and
live data. Live coverage can be expanded later by increasing the live warmup.

Initial contract:

- Unit of analysis: pool, not token. Tokens may have multiple pools, and trades
  happen against pools.
- Launch definition: pool creation.
- Real-liquidity lower bound: `0.5 ETH` or equivalent denom value. Pools below
  this threshold should be tracked as dust, and the dust ratio should be shown.
- Winner thresholds: `2x`, `5x`, `10x`, `20x`, `50x`, `100x`.
- Time windows from launch: `15m`, `1h`, `6h`, `24h`, `36h`, `48h`, `5d`,
  `7d`.
- Winner stats should be based on pools that are sellable by the simulator.
  Unsellable pools can inflate price ratios and should be reported separately
  as risk/scam behavior, not counted as strategy winners.
- Price-ratio stats should ignore dust winners by default, while still showing
  the ratio of launches that were filtered out as dust.

The first backend contract should expose enough pool-level data to answer:

- how many pools launched;
- how many crossed each winner threshold within each time window;
- how many were sellable at the time they crossed;
- how many were dust, scam/risk, or unsellable;
- how outcomes break down by protocol, currency, launch hour, initial liquidity
  bucket, tax bucket, LP control, and ownership/control evidence.

## Open Design Questions

- Should a pool count as a winner only if it was sellable before or at the same
  window where it hit the ratio, or is sellable by end of range enough?
- Should initial price mean pool-creation price, first meaningful liquidity over
  `0.5 ETH`, or first successful buy/sell simulation?
- Should stable and non-WETH denominated pools use ETH-equivalent liquidity
  thresholds, or denom-specific thresholds?
- Should scam/risk rate use all launched pools as the denominator, non-dust
  pools, or both?
- Which cohort breakdowns should ship first: protocol, currency, launch hour,
  initial liquidity bucket, tax bucket, LP control, or ownership/control?
- Should the frontend show a threshold-by-time-window matrix or headline stats
  such as `5x in 24h: 12 pools / 4.3%`?
