# graph

Raw and simplified token-network graph state.

This folder owns the in-memory network representation for a tracked token. The
raw graph should preserve multiedge evidence. Simplified views can collapse or
suppress noisy nodes for holder maps and downstream snapshots.

## Files

- `raw.rs`: canonical token-network state and update application.
- `index.rs`: lookup indexes for addresses, pools, tokens, and synthetic nodes.
- `simplified.rs`: view builder for holder graphs and pool graphs.

## Boundaries

- Does not parse `ProcessedTransaction` directly; use `ingest`.
- Does not decide UI layout.
- Keeps raw evidence even when simplified views hide hubs or pools.
- Should be deterministic for the same ordered block stream.

## First Implementation Target

Build a small append/update API that accepts typed network events and maintains
node/edge state with first/last seen block and evidence counts.

## Current Contract

The graph layer now exposes `RawTokenNetworkGraph`, which owns persistent state
for one token network:

- nodes keyed by stable `NetworkNodeId`;
- collapsed edges keyed by stable `NetworkEdgeId`;
- per-address `AddressActivity`;
- secondary indexes for token, address, pool, time-window, and synthetic nodes;
- an `apply_batch` API for `NetworkIngestBatch`.

Applying a batch:

- ensures address/pool/token nodes exist;
- updates address activity from movement and cost updates;
- merges node labels by kind/source/value and combines observation ranges;
- merges duplicate edges and combines evidence counts/examples;
- bounds stored address examples and edge evidence examples while keeping
  running totals intact.

This layer still does not simplify the graph, compute clusters, or produce
Asena-facing snapshots.

## Python Builder And Analyzer Assessment

The old Python implementation had useful features, but its graph semantics were
very specific:

- `LiveTokenNetworkBuilder` created nodes from `address_balance_changes`.
- `AddressTokenActivityTracker` stored the important per-address state:
  token in/out, denom in/out, fees, bribes, balances, buy/sell counts, and PnL
  proxies.
- The graph edge was not a direct transfer edge. `_add_fee_source_edges` linked
  the fee source to every address touched by the transaction with edge type
  `tx owner`.
- ERC20 and ETH movements influenced node features, but were not used as graph
  edges.
- `NetworkSubgraphAnalyzer` simplified the graph by removing default noisy
  nodes: token contract, pool contracts, zero address, dead address, and
  high-degree/high-frequency addresses.
- It protected fee sources from some removal rules because fee sources were the
  core of the old edge model.
- Connected components were weak components over the simplified directed graph.

The main assumption was: if the same transaction fee payer touches several
addresses, those addresses are likely related. That is useful for launch bots,
bundlers, deployer activity, and tax/owner interactions, but it is weaker than an
actual value transfer. It can also connect unrelated addresses through routers,
pools, exchanges, bridges, or fake-transfer patterns.

The Rust graph keeps the useful activity counters but makes the relationship
types explicit. `FeeSourceTouches` remains available, but it is no longer the
only edge type and should not be the first clustering signal.

## Raw Graph Vs Simplified Views

The raw graph should preserve all observed and inferred evidence. Simplified
views choose which edges are active for a particular question.

### Holder Direct View

Question: which token holders are directly related by value movement?

Use by default:

- `TokenTransfer` between address nodes.
- `DenomTransfer` between address nodes only when shown as a direct value-flow
  layer, not as a same-entity claim.

Suppress by default:

- token contract node,
- pool nodes,
- zero and burn addresses,
- known CEX, bridge, router, and other high-degree hubs,
- `PoolTrade`, `PoolCreation`, `LiquidityEvent`, `LpApproval`,
  `ControlRelation`.

Optional overlays:

- `FeeSourceTouches` as co-presence evidence,
- `Funding` when derived with timing/order constraints,
- control actor markers on holder nodes.

### Pool View

Question: who traded with, created, or changed liquidity in pools?

Use by default:

- `PoolCreation`,
- `PoolTrade`,
- `LiquidityEvent`,
- later `LpTransfer` and `LpApproval`.

Do not collapse traders into one holder cluster just because they trade through
the same pool. Pools are normal hubs.

### Control/Risk View

Question: how do owner/admin/tax/creator actors relate to holders and pools?

Use by default:

- `ControlRelation`,
- `TokenTransfer`,
- `DenomTransfer`,
- `PoolTrade`,
- `LiquidityEvent`.

This view is for explanation and risk features. It should not claim same-entity
ownership unless another stronger edge layer supports it.

### Inferred Cluster View

Question: which addresses appear related even without direct token transfers?

Use only after direct views exist:

- `Funding`,
- `SharedIntermediary`,
- `TemporalCoactivity`,
- selected `FeeSourceTouches`.

These edges require confidence and explanations. They should be rendered and
scored separately from direct observed transfers.

## Simplification Rules To Implement

1. Start from a selected edge policy, not from all raw edges.
2. Build an address-only working graph for holder components.
3. Suppress known default nodes: token contract, zero address, dead address,
   tracked pools, routers, CEX hot wallets, bridges, and known infrastructure.
4. Detect token-local hubs:
   - high in-degree addresses,
   - high out-degree addresses,
   - addresses appearing in too many transactions,
   - addresses connected to too many unrelated components.
5. Suppress hubs but keep a `suppressed_hubs` record with reason, degree, and
   affected edge count.
6. Never silently discard evidence from raw state.
7. Treat direct observed components and inferred components as separate layers.

## What We Should Identify First

The first simplified graph should identify:

- active holder addresses,
- direct token-transfer links between holders,
- direct denom value-flow links between relevant addresses,
- suppressed noisy nodes and why they were suppressed,
- weak connected components over the selected direct edges,
- per-component aggregate address activity.

It should not yet identify Magic Nodes, Time Nodes, or same-entity clusters.
Those belong in `clusters/` after the simplified holder graph is deterministic.
