# network

Token-centric relationship graph construction.

This module is the Rust home for the "token network" concept from the old Python
`erc20_token/network` package. The goal is not to recreate a dashboard here. Asena
owns presentation. This crate should own the compact state and snapshots needed
to explain how addresses, pools, control actors, and fund flows relate to a
tracked token.

## Status

Concept and folder skeleton are in place. The `model` layer now defines the
stable serializable IDs, node/edge kinds, labels, confidence, observations, and
evidence summaries. The `activity` layer now defines per-address movements,
totals, fee/bribe costs, and lightweight PnL summaries. The `ingest` layer now
converts `ProcessedTransaction` data into typed activity, label, and edge update
batches. The `graph` layer now applies those batches into persistent raw
node/edge/activity state. The cluster and snapshot layers are still
placeholders.

## Folder Layout

```text
network/
  README.md
  mod.rs
  config.rs

  model/
    README.md
    mod.rs
    ids.rs
    nodes.rs
    edges.rs
    labels.rs
    evidence.rs

  activity/
    README.md
    mod.rs
    address.rs
    movement.rs
    pnl.rs

  ingest/
    README.md
    mod.rs
    transaction.rs
    transfers.rs
    pools.rs
    authority.rs

  graph/
    README.md
    mod.rs
    raw.rs
    index.rs
    simplified.rs

  clusters/
    README.md
    mod.rs
    components.rs
    intermediaries.rs
    time_windows.rs
    scoring.rs

  snapshots/
    README.md
    mod.rs
    token.rs
    address.rs
    edge.rs
    cluster.rs
    pool.rs
    risk.rs
```

## Why This Exists

A token is not just a contract plus a list of pools. For launch tokens and live
trading, we care about the relationships around the token:

- who funded or controlled early buyers,
- which holders are connected by token, ETH, stablecoin, or fee-source flows,
- which pools and LP positions mediate activity,
- whether a set of wallets behaves like one coordinated actor,
- whether liquidity, tax, bribe, and control-address behavior point to a higher
  risk token.

The network module should turn processed block data into graph-ready analytical
state. The UI can then render holder maps, cluster tables, pool relationship
views, or time-window views without recomputing the graph from raw transactions.

## Existing Python Concept

The old Python code had three useful ideas:

- `LiveTokenNetworkBuilder` consumed each processed transaction, read
  `address_balance_changes`, created address nodes, updated per-address movement
  counters, and linked the fee source to addresses touched by the transaction.
- `AddressTokenActivityTracker` kept per-address token in/out, denomination
  in/out, fee spend, bribe exposure, balance, buy/sell counts, and rough
  realized/unrealized PnL features.
- `NetworkSubgraphAnalyzer` simplified noisy graphs by removing the token
  contract, pools, burn/zero addresses, and high-degree nodes; it then produced
  connected components and aggregate component metrics.

That is a good starting point, but too narrow for the Rust side. It treated the
network mostly as an address-activity graph. The Rust version should be a
token-centric relationship graph that can also represent pools, LP ownership,
control addresses, launch-time coordination, and non-holder intermediaries.

## External Context

Public tools and research point to several design constraints:

- Bubblemaps starts from top holders, sizes nodes by current holdings, and draws
  links when holders have on-chain transfers. It also hides noisy contracts or
  exchange nodes by default and treats very high-volume "Supernodes" specially
  for performance.
- Bubblemaps "Magic Nodes" adds non-holder intermediaries that connect current
  holders, such as shared gas funders or shared deposit addresses. This matters
  because an address can connect holders even if it currently holds zero token.
- Bubblemaps "Time Nodes" splits noisy hubs like exchanges or DEXs into
  time-window nodes so wallets can be clustered by coordinated timing instead of
  direct transfers.
- ERC20 token-network research defines token networks as address graphs whose
  edges are token transfers, and finds many individual ERC20 networks dominated
  by hub-and-spoke structures rather than rich social-style communities.
- Ethereum address-clustering research is careful about account-model limits:
  Bitcoin-style multi-input heuristics do not directly apply, so Ethereum
  clustering must be heuristic, labeled, and confidence-aware.
- Ethereum phishing and fraud-detection papers repeatedly model the chain as a
  directed multigraph with temporal, frequency, amount, and interaction features.
  We should preserve multiedge evidence internally even if Asena receives a
  simplified view.

## Core Model

The network should maintain several related views over the same block-level
facts. These are views, not separate sources of truth.

### Nodes

- `Token`: the tracked ERC20 contract.
- `Address`: wallets and contracts observed around the token.
- `Pool`: liquidity pools associated with the token, across V2/V3/V4 when those
  pool implementations exist.
- `ControlActor`: owner, pending owner, admin role, proxy admin, creator, tax
  wallet, or another address inferred by the authority/state modules.
- `LiquidityActor`: LP holder, router approver, pool creator, mint/burn actor.
- `Intermediary`: non-holder address that links holders, such as funders,
  deposit addresses, bridges, exchanges, routers, or routers/contracts that pass
  through value.
- `TimeWindow`: synthetic node for coordinated activity through a noisy hub in a
  bounded block or timestamp window.

Each node should carry labels and confidence separately from identity. An address
can be a wallet, contract, known CEX address, pool, control actor, or unknown,
and that classification can improve over time.

### Edges

- `TokenTransfer`: ERC20 transfer from one address to another.
- `DenomTransfer`: ETH/WETH/stable/known-denom flow involving an observed
  address.
- `PoolTrade`: address traded against a pool, with side, token amount, denom
  amount, price, block, tx index, and pool protocol when known.
- `LpTransfer`: LP token transfer or LP balance movement.
- `LpApproval`: LP approval, especially router approval percentage.
- `FeeSourceTouches`: transaction fee payer touched another address in the same
  processed transaction. This preserves the old Python fee-source relationship.
- `Funding`: one address funded another with ETH or a known denom before or
  during the relevant token activity.
- `ControlRelation`: owner/admin/role/proxy/creator relationship to the token or
  pool.
- `SharedIntermediary`: two addresses are linked through a non-holder
  intermediary.
- `TemporalCoactivity`: addresses acted through the same noisy hub or pool in the
  same configured time/block window.

Edges should keep raw evidence counts and representative examples. A simplified
graph can collapse multiple edges, but the stored state should know direction,
amount buckets, first/last seen block, tx counts, and source evidence.

## Inputs

The network module should consume state already produced by this crate and by
`tx_processor`; it should not fetch blocks itself.

- `ProcessedBlock` / `ProcessedTransaction` from the manager loop.
- `ERC20Token.transfer_tracker` for token, ETH, WETH, denom transfers, approvals,
  bribes, and address counters.
- `ERC20Token.authority_tracker` and `status_manager` for control and policy
  relationships.
- Pool updates from `pools`, starting with V2 and later extending to V3/V4.
- Chain metadata and known-address classification from `reth_chain_query` or a
  later label provider.

## Outputs

The crate should expose compact snapshots, not UI-specific layouts:

- `TokenNetworkSnapshot`: graph summary for one token at a block.
- `AddressActivitySnapshot`: per-address balances, volume, fees, bribes, PnL
  proxies, buy/sell counts, first/last seen block, and labels.
- `NetworkEdgeSnapshot`: collapsed edge with evidence counts, amount totals,
  first/last seen block, and edge type.
- `ClusterSnapshot`: connected component or heuristic entity cluster with member
  addresses, aggregate holdings, denom spent/received, realized/unrealized PnL
  proxy, bribes, fees, pool interactions, and confidence.
- `PoolNetworkSnapshot`: pool-centric view of traders, LP holders, liquidity
  changes, and links to control actors.
- `NetworkRiskSnapshot`: bounded set of explainable signals such as concentrated
  supply, linked holders, shared funders, launch snipers, coordinated exits,
  control actor trading, LP concentration, and supernode dependence.

Asena can choose how to render these snapshots: bubble maps, tables, timelines,
cluster cards, or pool relationship panels.

## Simplification Rules

The raw graph will be noisy. Simplification should be explicit and reversible
where possible.

- Always remove or suppress the token contract, zero address, burn address, and
  tracked pools from holder-cluster views.
- Treat known CEX hot wallets, bridges, routers, DEX pools, and other high-degree
  contracts as hubs, not as normal holder evidence.
- Do not discard hubs completely. Keep them as suppressed nodes, shared
  intermediaries, or time-window nodes.
- Detect high-degree or high-frequency nodes per token and per rolling window.
- Keep fee-source links distinct from actual value transfers.
- Keep direct holder-transfer clusters separate from inferred clusters created by
  funders, deposit addresses, or time-window coactivity.
- Every inferred cluster needs an explanation and confidence. We should avoid
  presenting "same entity" as fact unless the evidence is very strong.

## Risk Signals

Initial signals should be explainable and block-local enough for live use:

- large share of supply held by one cluster,
- many top holders connected through a shared funder or shared deposit address,
- many fresh holders funded shortly before launch,
- many addresses buying in the same first blocks through the same pool,
- control addresses buying, selling, funding buyers, or receiving taxes,
- LP tokens concentrated in a small cluster,
- pool reserve or LP actions linked to owner/admin wallets,
- cluster realized profit while the token remains broadly illiquid,
- bribe-heavy launches or repeated same-fee-source activity,
- hidden-mint or tax/max-buy events connected to trading clusters.

These are features, not automatic verdicts. Scam/risk labeling should remain
separate from the evidence graph.

## Implementation Phases

1. Data model only:
   - node identifiers, node labels, edge types, activity counters, cluster
     summaries, and snapshot structs.
2. Address activity tracker:
   - port the useful Python counters using existing Rust `TokenTransferTracker`
     data.
3. Raw graph builder:
   - update incrementally from `ProcessedTransaction` inside the token manager
     flow, preserving multiedge evidence.
4. Simplified holder graph:
   - build the first Asena-facing snapshot: holders, balances, direct links,
     suppressed hubs, and connected components.
5. Intermediary discovery:
   - add Magic-Node-style non-holder connectors for shared funders, deposit
     addresses, and pass-through addresses.
6. Time-window clustering:
   - add Time-Node-style synthetic nodes for CEX/bridge/DEX/pool coactivity in
     configurable block or timestamp windows.
7. Pool network:
   - connect traders, LP holders, pools, control actors, and liquidity events.
8. Risk snapshot:
   - expose explainable network signals for the strategy and Asena layers.

## Non-Goals

- No dashboard code in this crate.
- No direct historical block fetching from this module.
- No machine-learning model in the first implementation.
- No claim that a heuristic cluster is a confirmed real-world entity.
- No requirement to match the Python JSON shape exactly.

## References

- Bubblemaps V2 overview: https://wiki.bubblemaps.io/bubblemaps-v2/how-does-it-work
- Bubblemaps Magic Nodes: https://wiki.bubblemaps.io/bubblemaps-v2/magic-nodes
- Bubblemaps Time Nodes: https://wiki.bubblemaps.io/bubblemaps-v2/time-nodes
- Bubblemaps Time Travel: https://wiki.bubblemaps.io/bubblemaps-v2/time-travel
- Victor and Luders, "Measuring Ethereum-based ERC20 Token Networks":
  https://fc19.ifca.ai/preproceedings/130-preproceedings.pdf
- Victor, "Address clustering heuristics for Ethereum":
  https://fc20.ifca.ai/preproceedings/31.pdf
- Zhang et al., "Phishing Node Detection in Ethereum Transaction Network Using
  Graph Convolutional Networks": https://www.mdpi.com/2076-3417/13/11/6430
- Li et al., "TTAGN: Temporal Transaction Aggregation Graph Network for Ethereum
  Phishing Scams Detection": https://arxiv.org/abs/2204.13442
- Josenhans et al., "Characterizing Transfer Graphs of Suspicious ERC-20
  Tokens": https://www.researchgate.net/publication/388232639_Characterizing_Transfer_Graphs_of_Suspicious_ERC-20_Tokens
