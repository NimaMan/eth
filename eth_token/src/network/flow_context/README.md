# flow_context

Second-order fund-flow context for a token network.

The raw token network is the first-order view: tracked-token transfers, pool
trades, liquidity events, and token control relations. This module builds the
second-order view around those same actors using non-token fund flows such as
ETH, WETH, stables, and other known denominations.

This layer is the bridge between a token-specific graph and generic fund-flow
analytics. It should make relationships visible when the token graph alone only
shows many apparently unrelated traders touching a pool.

## Goal

Reveal relationships that are invisible if we only look at the token contract:

- wallets funded by the same upstream address before buying;
- profitable wallets cashing out to the same sink;
- short multi-hop funding paths between token actors;
- timing patterns such as funding shortly before first token activity;
- the filtered backbone after noisy hubs are suppressed.

Typical scam hypothesis:

```text
operator/control address
  -> funds several wallets with ETH/WETH/stables
  -> wallets buy/swap through the token pool
  -> wallets transfer or churn the token to inflate apparent activity
  -> wallets sell or remove liquidity
  -> profits converge to a shared sink
```

The first-order token graph may only show pool interactions and token transfers.
The second-order flow context should reveal the funding and cash-out structure
around those interactions.

## Boundaries

- Does not mutate `RawTokenNetworkGraph`.
- Does not decode receipts, traces, or logs directly.
- Uses address-block participation and processed-block loading through traits.
- Delegates actual fund-flow extraction to adapters around existing
  `tx_fund_flow` and processed-block APIs.
- Keeps observed background flows separate from inferred cluster/promoted edges.
- Does not treat "same funder" as proof of same entity. It records evidence,
  confidence, timing, and path length so downstream risk scoring can decide.

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

## Concrete `tx_fund_flow` Adapter

`tx_fund_flow.rs` is the production adapter for the current pipeline. The token
network still owns seed and window selection; this adapter only consumes the
processed blocks loaded for those selected windows.

Default behavior:

- runs `tx_fund_flow::extract_fund_flows_from_processed_tx` per processed tx;
- emits `FlowContextObservation` for direct ETH and internal ETH flows;
- emits ERC-20 observations only for known denomination assets such as WETH and
  stables from `DENOM_ADDRESSES`;
- requires at least one endpoint to be a token-selected seed address;
- skips failed transactions, gas payments, zero-address flows, and unknown
  non-denomination token transfers unless explicitly configured otherwise.

That keeps the first concrete pass focused on the signals we care about most:
shared funders, shared sinks, and direct denomination movement around token
actors.

## Output Semantics

`FlowContextLayer` is not a replacement for `RawTokenNetworkGraph`.

- `FlowContextEdgeKind::DirectDenomFlow` is observed non-token value movement.
- `SharedFunder`, `SharedSink`, `TemporalFunding`, and `MultiHopFundingPath`
  are second-order context edges.
- `promotion.rs` can translate high-confidence context into existing inferred
  token-network edge kinds such as `Funding`, `SharedIntermediary`, and
  `TemporalCoactivity`, but promotion must stay explicit.
- `hub_filter.rs` creates the backbone by suppressing noisy non-seed hubs while
  preserving `SuppressedHub` records.

## Risk/Graph-ML Use

This layer should eventually feed deterministic risk rules and graph-learning
experiments. Keep the output typed and evidence-rich:

- node role: seed, funder, sink, intermediary, hub;
- edge kind: direct flow, shared funder, shared sink, temporal funding, path;
- confidence and explanation;
- block/tx/log evidence;
- amounts and assets;
- suppressed-hub metadata.

Do not collapse these into untyped "related address" edges too early. The
model/risk layer needs to know why two addresses are connected.

## Files

- `config.rs`: limits, windows, hub thresholds, scoring thresholds.
- `model.rs`: stable second-order layer DTOs.
- `seeds.rs`: address selection from the token graph.
- `windows.rs`: block windows around seed activity.
- `index_query.rs`: address participation query trait.
- `block_loader.rs`: processed block loading trait.
- `extractor.rs`: fund-flow observation extraction trait and aggregation.
- `tx_fund_flow.rs`: concrete adapter from `tx_fund_flow` movements into
  `FlowContextObservation`.
- `builder.rs`: orchestration.
- `hub_filter.rs`: noisy-node suppression.
- `scoring.rs`: confidence scoring.
- `promotion.rs`: conversion to existing inferred network edge kinds.
- `snapshot.rs`: compact API/debug output.
