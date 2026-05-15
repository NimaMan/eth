# token_analytics

Source-level feature contract for token-pool analytics.

The module is called `token_analytics` because it belongs to the token crate,
but the primary row is one active observation for one `(token, pool)` pair.
This is the shared vocabulary for live inspection, historical exports, and
training-data generation.

## Scope

This module should define observations and features known as of an observation:

- token and pool identity;
- active observation index, block, timestamp, and reasons the block is active;
- token creation and supply context;
- owner, creator, renouncement, and control-address context;
- pool lifecycle, trading, tax, and sell-failure context;
- reserve, liquidity, price, price-to-initial, and drawdown context;
- LP holder and router-approval context;
- token/pool block activity;
- token-network and fund-flow summaries;
- latest evidence block per feature family for leakage checks.

Future labels do not belong here. A target such as
`rug within the next 10 active observations` requires future observations, so it
belongs in `token_lab/scam_analytics` or a historical export layer that joins
labels onto these source features.

## Active Observation

An active observation is a block where the token or pool materially changes or
emits useful signal: swap, mint, burn, sync/reserve update, token approval,
LP approval, LP transfer, token transfer touching the pool, buy/sell volume,
bribe, trading status change, scam status change, control-address activity, or
token-network activity. Idle chain blocks do not advance the active observation
index.

## Observation Folder

`observation/` is the current as-of-block contract:

- `mod.rs`: token/pool identity, active observation index, block, timestamp, and
  reasons the block is active.
- `current.rs`: current block attributes that already exist today:
  transaction count, token transfers, denomination transfers, buy volume,
  sell volume, bribe, plus end-of-block pool trading state (`can_buy`,
  `can_sell`, effective buy/sell, tax rates, and liquidity-removal state).
  It also carries event flags for token creation, pool creation, trading
  enablement, pool swaps/mints/burns/syncs, LP transfers, LP approvals,
  liquidity updates, price updates, tax checks, trading status changes, scam
  status changes, and direct liquidity removal.
- `tx_classification.rs`: stable transaction labels, descriptions, and
  token/pool-affect flags shared by summaries and downstream renderers.
- `transaction.rs`: typed transaction summaries for the observation block. A
  transaction can be a swap, token approval, token transfer, denominator
  transfer, LP transfer, LP approval, mint, burn, sync, trading simulation, tax
  simulation, bribe, control-address activity, network activity, or other.

The existing `token_activity` tracker is still the raw per-token accumulator.
`token_analytics::observation` is the pool-scoped contract we should build from
when creating analytics rows. Until the pool-scoped builder is wired, current
token activity can be converted into observation activity as a compatibility
source.

Observation activity distinguishes source and completeness. A block that comes
from an address-participation index, liquidity history, price history, LP
history, or lifecycle marker may be active even when decoded token-activity
metrics are not present. Those rows must be marked as incomplete-source rows,
not as known zero-transaction rows.

The current target horizon constants are kept here as shared metadata:

```text
1, 2, 3, 5, 10, 15, 20, 30, 50, 100, 250, 500
```

## Data Ownership

```text
tx_processor facts
  -> eth_token tracking/pools/state/token_activity/network
  -> eth_token token_analytics features as-of observation O
  -> health consumes latest features for current risk assessment
  -> token_lab joins future labels for historical training rows
  -> Asena renders observations and labeled overlays
```

`health` should stay a verdict/scoring layer. It can consume these features,
but it should not own historical feature construction.
