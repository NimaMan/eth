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
- All strategy performance stats should be computed only over eligible pools.
  Non-eligible pools are an exclusion summary, not part of winner, scam-after-
  entry, or strategy outcome denominators.

## Eligibility Filter

The first layer is a strict split between eligible and non-eligible pools.
Eligibility is the set of pools we would have considered trading at the time of
analysis.

Initial eligible pool rules:

- pool has at least `0.5 ETH` liquidity in an ETH/WETH-denominated pool;
- simulator says the pool can be bought and sold;
- pool currency is currently supported by the strategy analysis. Stable,
  unknown, or unsupported currencies go into non-eligible until we add explicit
  ETH-equivalent thresholds.

Initial non-eligible reasons:

- dust/test liquidity below the threshold;
- buyable but not sellable;
- not buyable;
- unsupported or unknown currency;
- missing protocol-specific data needed for strategy stats.

Later scams or failures among eligible pools are not removed from the eligible
cohort. They are outcomes of the eligible cohort and should be measured as
strategy risks. Examples include liquidity removal, hidden mint/rebase behavior,
tax changes, ownership/control actions, or later sell failure.

Eligibility rules can expand later as we learn from the non-eligible pool
summary and add support for more currencies, protocols, or simulator paths.

The first implementation computes time windows from block distance using a
`12s` Ethereum block-time estimate because the current price-ratio history is
block-number based. It also uses the current run snapshot for buy/sell
eligibility until we add per-block pool eligibility snapshots.

The first backend contract should expose enough pool-level data to answer:

- how many pools launched;
- how many pools were eligible versus non-eligible;
- why pools were non-eligible;
- how many crossed each winner threshold within each time window;
- how many eligible pools later failed through scam/risk behavior;
- how eligible outcomes break down by protocol, currency, launch hour, initial
  liquidity bucket, tax bucket, LP control, and ownership/control evidence.

## Open Design Questions

- Should initial price mean pool-creation price, first meaningful liquidity over
  `0.5 ETH`, or first successful buy/sell simulation?
- Should stable and non-WETH denominated pools use ETH-equivalent liquidity
  thresholds, or denom-specific thresholds?
- Which cohort breakdowns should ship first: protocol, currency, launch hour,
  initial liquidity bucket, tax bucket, LP control, or ownership/control?
- Should the frontend show a threshold-by-time-window matrix or headline stats
  such as `5x in 24h: 12 pools / 4.3%`?
