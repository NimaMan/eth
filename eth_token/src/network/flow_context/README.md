# flow_context

Second-order fund-flow context for a token network.

The raw token network is the first-order view: tracked-token transfers, pool
trades, liquidity events, and token control relations. This module builds the
second-order view around those same actors using non-token fund flows such as
ETH, WETH, stables, and other known denominations.

## Goal

Reveal relationships that are invisible if we only look at the token contract:

- wallets funded by the same upstream address before buying;
- profitable wallets cashing out to the same sink;
- short multi-hop funding paths between token actors;
- timing patterns such as funding shortly before first token activity;
- the filtered backbone after noisy hubs are suppressed.

## Boundaries

- Does not mutate `RawTokenNetworkGraph`.
- Does not decode receipts, traces, or logs directly.
- Uses address-block participation and processed-block loading through traits.
- Delegates actual fund-flow extraction to adapters around existing
  `tx_fund_flow` and processed-block APIs.
- Keeps observed background flows separate from inferred cluster/promoted edges.

## Pipeline

```text
RawTokenNetworkGraph
  -> seeds
  -> windows
  -> address-index query
  -> processed-block loader
  -> fund-flow extractor
  -> direct/shared-funder/shared-sink candidates
  -> hub filter
  -> scoring
  -> FlowContextLayer
  -> optional promotion into inferred token-network edges
```

## Files

- `config.rs`: limits, windows, hub thresholds, scoring thresholds.
- `model.rs`: stable second-order layer DTOs.
- `seeds.rs`: address selection from the token graph.
- `windows.rs`: block windows around seed activity.
- `index_query.rs`: address participation query trait.
- `block_loader.rs`: processed block loading trait.
- `extractor.rs`: fund-flow observation extraction trait and aggregation.
- `builder.rs`: orchestration.
- `hub_filter.rs`: noisy-node suppression.
- `scoring.rs`: confidence scoring.
- `promotion.rs`: conversion to existing inferred network edge kinds.
- `snapshot.rs`: compact API/debug output.
