# Live Token Network Analysis

## Overview

The network module builds a dynamic view of token flows and address
relationships for every ERC20 token tracked by the system. It
translates raw transaction activity into graph-based insights that can
be consumed by higher level monitoring, alerting, and analytics
workflows.

The central object, `LiveTokenNetwork`, aggregates per-transaction
changes, keeps compact statistics for each address, and produces
derived metrics such as related address clusters, net balances, and
bribe exposure.

## Architecture

```
┌──────────────────────────────────────────┐
│            LiveTokenNetwork              │
│  (network/token_network.py)              │
├───────────────────┬──────────────────────┤
│ Graph Builder     │ Subgraph Analysis    │
│ ┌───────────────┐ │ ┌──────────────────┐ │
│ │LiveTokenNetwork│ │ │NetworkSubgraph  │ │
│ │Builder         │ │ │Analyzer         │ │
│ └───────┬────────┘ │ └────────┬────────┘ │
│         │            │        │           │
│ ┌───────▼────────┐   │   ┌────▼────┐      │
│ │AddressToken    │   │   │Metrics  │      │
│ │ActivityTracker │   │   │(aggregates)│   │
│ └───────┬────────┘   │   └─────────┘      │
└─────────┴────────────┴────────────────────┘
```

- **LiveTokenNetworkBuilder** (`live_token_network_builder.py`)
  - Ingests processed transactions (`tx_dict`)
  - Calculates address state deltas using
    `AddressBalanceChangeCalculator`
  - Maintains a `networkx.MultiDiGraph` with per-edge metadata (transfer
    amount, type, fee source)
  - Updates `AddressTokenActivityTracker` instances attached to graph nodes

- **AddressTokenActivityTracker** (`address_activity_tracker.py`)
  - Holds per-address counters (token in/out, denom in/out)
  - Tracks realised/unrealised PnL proxies, fee spend, bribe exposure
  - Maintains rolling movement dictionaries bounded by history limits

- **NetworkSubgraphAnalyzer** (`subgraph_analyzer.py`)
  - Simplifies the graph to reduce noise
  - Detects connected components / strongly connected components
  - Produces cluster-level statistics (component size, combined balance,
    counterparty overlap)

## Data Flow

1. `ERC20TokenData.update_from_transaction` persists transfer and fee
   information keyed by `tx_hash`.
2. `LiveTokenNetwork.update_from_transaction` receives the same transaction:
   - Builder resolves `tx_index`, block metadata, and fee source.
   - State diff calculator extracts meaningful balance changes.
   - Address trackers are updated with token/denom movements and fees.
3. After the builder updates the graph, the network triggers subgraph
   analysis when needed (e.g., upon request or at throttled intervals).

## Key Features

- **Fee Source Tracking**
  - Attributes each transaction to a "fee source" (the paying address).
  - Maintains per-source aggregates of bribes and gas expenditure.

- **Movement Dictionaries**
  - Token movements (`token_in_dict`, `token_out_dict`) and denomination
    movements (`denom_in_dict`, `denom_out_dict`) are kept as bounded
    dictionaries, enabling quick aggregation without unbounded growth.

- **Graph Metrics**
  - Node-level: total inflow/outflow, fee totals, bribe totals, entry
    block/index for first observation.
  - Edge-level: amount, transfer direction, whether created by fee source.
  - Component-level: cluster sizes, combined balances, relationship maps.

- **Bribe Awareness**
  - Reads `ERC20Token.total_bribe_amount` / `bribe_amounts_by_tx` to
    surface bribe exposure directly on address trackers.

## Usage Tips

```python
network = token.token_network

# Get aggregate activity for each address (DataFrame)
activity_df = network.get_agg_user_activity_df()

# Access the underlying graph (networkx.MultiDiGraph)
graph = network.graph

# Recompute or fetch cached connected components
components = network.connected_components  # populated by subgraph analyzer

# Explore related addresses for an entity of interest
related = network.subgraph_analyzer.get_related_addresses(target_address)
```

## Performance Characteristics

- **Bounded history**: Trackers and dictionaries respect the global
  `history_limit`, preventing unbounded memory growth.
- **Incremental updates**: Only touched nodes/edges are updated per
  transaction, making the builder suitable for real-time streams.
- **NetworkX dependency**: The graphs leverage `networkx` algorithms,
  but heavy operations (e.g., full component scans) should be scheduled
  selectively in high-throughput environments.

## Extensibility

- Custom metrics can be computed by iterating over `network.graph.nodes`
  and inspecting the attached `AddressTokenActivityTracker`.
- Additional anomaly detectors can plug into the subgraph analyzer by
  consuming its simplified graph and component statistics.
