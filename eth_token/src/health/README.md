# health

Token health, deceptive-volume analysis, and scam-risk signals.

This folder is the Rust home for the useful concepts from the old Python
`erc20_token/token_health` modules. For now this is documentation only. We do
not yet have the curated address data needed for known scammer or legitimate
actor detection, so the address-list-based checks should be treated as future
work.

## Scope

The health layer should answer whether a token's observed behavior looks
healthy, suspicious, or already unsafe. It should read token, pool, transfer,
and network summaries produced elsewhere; it should not mutate token state
directly during block processing.

Health outputs should be deterministic structs that can be exposed through the
token server and Asena. The same outputs can later label nodes and transactions
inside the token network graph.

## Python Concepts To Preserve

### Deceptive Volume Analyzer

Python `VolumeAnalyzer` was not a market-volume calculator. It was a
deceptive-volume heuristic layer over processed transactions:

- `green_addresses`: known legitimate or important actors, such as whales,
  oracles, exchanges, or other addresses that can make a transfer look more
  legitimate than it is.
- `mimic_octopus_addresses`: known grey or malicious actors.
- malicious actor involvement: if more than one known malicious address appears
  in `unique_addresses`, mark the transaction as highly suspicious.
- green actor involvement: if more than one known green address appears in
  `unique_addresses`, record the interaction as a possible fake-buy or
  legitimacy-mimic signal.
- fanout heuristic: count ERC20 transfers and unique addresses in one
  transaction; unusually high fanout is a weak deceptive-volume signal.

The original thresholds were simple defaults:

- more than `15` ERC20 transfers in one transaction, or
- more than `20` unique addresses in one transaction,
- known malicious actor involvement had much higher confidence than fanout.

These thresholds should become explicit config in Rust rather than hidden
constants.

### Health Predictor

Python `TokenHealthPredictor` converted the volume analyzer outputs plus token
state into a compact assessment:

- `has_malicious_activity`
- `scam_probability`
- `scam_reason`
- detection block and transaction hash
- involved grey addresses and count
- involved green addresses and count

This should become a Rust health assessment view. It should not replace
existing hard scam labels such as hidden mint or cannot-sell states; it should
layer softer behavioral evidence on top.

### Scam Thresholds

Python also kept threshold config for:

- hidden mint tolerance, currently represented in Rust by token status manager
  logic;
- liquidity dust/drain thresholds by denomination category;
- stablecoin, wrapped-native, BTC, gold, and fiat-denominated threshold
  families.

Rust already has pieces of this split across token status and pool liquidity
views. The health layer should eventually centralize threshold configuration
without moving pool state mutation into `health`.

### Token Snapshot Shape

Python exposed a compact token snapshot instead of serializing the entire token
object. The useful shape was:

- metadata and creation context;
- latest processed block context;
- lifecycle/scam/trading status;
- owner/control addresses and renouncement state;
- pool addresses, prices, reserves, liquidity, and trading flags;
- transfer summaries such as bribes, approvals, and per-address tx counts;
- optional bounded transfer history.

Rust token server views already provide much of this as detail/list responses.
If we need a stable cache/export payload later, this snapshot concept is the
right model.

### Multi-Pool Health Signals

Python `MultiPoolLiquidityAnalyzer` and arbitrage helpers were adjacent to
health. Useful concepts:

- best executable pool for buy and sell;
- aggregate liquidity by denomination;
- total token liquidity across pools;
- simple V2 constant-product impact estimate;
- cross-pool price spread and arbitrage opportunity ranking.

Rust already tracks V2 pool state, liquidity, prices, buy/sell status, taxes,
and LP control. We do not yet have a first-class health view that turns this
into "price spread looks manipulated", "liquidity is fragmented", or "best
buy/sell route changed".

### Network And Cluster Health

The new Rust token network already computes per-address activity and PnL from
processed transaction movements. Health should later consume that state for:

- suspicious high-fanout transfer transactions;
- addresses with deceptive volume patterns;
- related-address clusters with aggregate PnL, balances, and activity;
- green/grey address labels on graph nodes once address lists exist;
- weak evidence labels for fee-source co-presence or hub-mediated interaction.

Cluster and subgraph implementation belongs under `network/clusters`; health
should consume cluster summaries and produce risk labels.

## Current Rust Coverage

Already present elsewhere:

- `state/transfer`: token, ETH/WETH, denomination transfers, approvals, bribes,
  per-address transaction counters, and minted supply from zero address.
- `state/authority`: owner, pending owner, AccessControl roles, proxy admins,
  and renouncement state.
- `state/status`: trading enabled/disabled and hidden-mint scam detection.
- `pools`: V2 reserves, prices, liquidity, buy/sell status, taxes, LP holders,
  LP approvals, and pool risk/liquidity views.
- `network`: address movement, PnL proxy, raw graph state, and token detail
  views for address PnL and graph visualization.

Missing or intentionally deferred:

- known green/grey/scammer address datasets;
- deceptive-volume scoring structs;
- health assessment API/view;
- configurable fanout and known-actor thresholds;
- cross-pool spread/arbitrage health output;
- cluster-level health labels.

## Future Implementation Path

1. Add config structs for fanout thresholds and optional address lists.
2. Add pure analysis functions that accept `ProcessedTransaction` plus config
   and return transaction-level health signals.
3. Add `TokenHealthAssessment` as an aggregate read model over token status,
   pool views, network activity, and transaction health signals.
4. Expose the assessment in token detail API responses.
5. Render a compact "Health Signals" panel in Asena.
6. Feed health labels back into token network node/edge views once cluster
   summaries are implemented.
