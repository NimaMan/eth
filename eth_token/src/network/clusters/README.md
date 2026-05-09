# clusters

Cluster and inferred-relationship analysis.

This folder turns raw/simplified graph state into explainable groups of related
addresses. It should distinguish direct evidence from inferred relationships and
attach confidence to every inferred cluster.

## Files

- `components.rs`: connected-component detection over selected graph views.
- `intermediaries.rs`: Magic-Node-style shared funder, deposit, and pass-through
  connector discovery.
- `time_windows.rs`: Time-Node-style coactivity grouping by block/timestamp
  windows around noisy hubs.
- `scoring.rs`: confidence and risk-feature scoring for clusters.

## Boundaries

- Does not mutate raw graph state.
- Does not label a heuristic cluster as a confirmed real-world entity.
- Keeps direct-transfer clusters separate from inferred intermediary or timing
  clusters.
- Produces explainable cluster evidence for snapshots.

## First Implementation Target

Start with connected components over a simplified holder graph, then add
intermediary and timing clusters as separate evidence layers.

## Subgraph Analyzer Lessons From Python

The Python `NetworkSubgraphAnalyzer` gave us the right direction but mixed a few
different ideas into one graph:

- It used only `tx owner` edges, meaning fee-source co-presence was the clustering
  relation.
- It removed default noisy nodes: token contract, pools, zero address, dead
  address, and high-degree nodes.
- It removed incoming edges to high-frequency non-fee-source addresses before
  removing high-degree nodes.
- It used weakly connected components, then aggregated activity features for the
  addresses in each component.
- It had TODOs around fake transfer volume, wallet-vs-contract thresholds, and
  separate degree rules.

Rust should keep those ideas, but split them into explicit layers. A direct
transfer component, a same-funder component, a fee-source co-presence component,
and a time-window component are different claims. They should not be merged
without confidence and evidence.

## Cluster Layers

### Layer 1: Direct Observed Components

Purpose: find addresses connected by direct value movement.

Edges:

- `TokenTransfer`,
- selected direct `DenomTransfer`.

Confidence:

- high for direct tracked-token transfer,
- medium for denom transfer unless timing and amount imply funding.

Output:

- connected components,
- member addresses,
- aggregate token/denom balances,
- aggregate fees/bribes/PnL proxy,
- first/last seen block,
- direct evidence examples.

### Layer 2: Funding Components

Purpose: find launch or trading groups funded by the same address.

Edges:

- derived `Funding` from denom flows.

Identification rules:

- funder sends ETH/WETH/stable/known denom to a holder before or near that
  holder's first token buy/receive;
- funding happens within a configurable block/time window;
- amount is non-dust relative to the later buy or fee spend;
- known CEX/bridge/router hubs are suppressed or represented as intermediaries,
  not direct same-entity evidence.

Confidence:

- high when funding is direct, near in time, and reused by multiple fresh
  addresses;
- medium when timing is weaker or amount match is unclear;
- low when the funder is a noisy hub.

### Layer 3: Fee-Source Co-Presence Components

Purpose: preserve the old Python `tx owner` concept.

Edges:

- `FeeSourceTouches`.

Identification rules:

- fee payer touched multiple addresses in the same processed transaction;
- do not include default noisy nodes;
- treat routers/pools/contracts as touched infrastructure, not as holder members;
- count repeated co-presence across blocks as stronger evidence.

Confidence:

- medium when the same fee source repeatedly touches the same holder set;
- low for one-off large transactions involving many unrelated addresses.

### Layer 4: Shared Intermediary Components

Purpose: Magic-Node-style discovery of non-holder connectors.

Edges:

- `SharedIntermediary`.

Identification rules:

- a non-holder address connects multiple holders through funding, deposits,
  receipts, or pass-through transfers;
- the intermediary is not a known global hub, or it is a hub with a meaningful
  token-local pattern;
- the intermediary should remain visible as a connector, even if suppressed from
  direct holder components.

Confidence:

- based on number of holders connected, timing, amount similarity, and hub score.

### Layer 5: Temporal Coactivity Components

Purpose: Time-Node-style grouping around noisy hubs.

Edges:

- `TemporalCoactivity`.

Identification rules:

- addresses interact with the same pool/CEX/bridge/router within a configurable
  block or timestamp window;
- activity is token-relevant, for example first buys, coordinated sells, or LP
  changes;
- the hub itself remains synthetic or suppressed, not a normal holder member.

Confidence:

- low by default;
- increases with repeated coordinated windows, similar sides, and matching pool
  or amount behavior.

## What To Identify

The cluster layer should identify:

- directly connected holder groups,
- shared-funder launch groups,
- repeated fee-source or bundler groups,
- holders connected through non-holder intermediaries,
- coordinated first-buy or exit windows,
- control-actor-linked trading groups,
- LP/control/liquidity clusters,
- noisy hubs and why they were suppressed.

## What Not To Claim

- Do not call a heuristic component a confirmed same-entity wallet set.
- Do not merge everyone who traded through the same pool.
- Do not merge everyone who touched the same CEX/bridge/router.
- Do not let zero/burn/token/pool nodes create holder clusters.
- Do not hide the evidence path behind a cluster score.

Each cluster must carry the layer that produced it, the active edge kinds, the
suppressed nodes, and representative evidence.
