# ETH Fund Flow Network Builder

This document describes the algorithm implemented in `FundFlowNetworkBuilder` to construct and expand an interactive graph representing significant **Ethereum (ETH) flows** between addresses.

**Context:** This component is part of the broader Sarigoz analytics platform. It focuses specifically on visualizing direct and indirect ETH transfers exceeding a defined threshold, providing insights into capital movement. It is distinct from other analyses like token-specific PnL tracking or general address profiling.

## Objective

To build and interactively expand a directed graph centered around a user-specified seed address, where nodes represent Ethereum addresses and edges represent aggregated ETH transfers above `eth_threshold`. The graph helps visualize funding sources, destinations, and intermediate hops.

## Core Components

-   **`FundFlowNetworkBuilder`**: The main class orchestrating the graph building and expansion. It holds the `networkx.MultiDiGraph` state.
-   **`AddressActivityProvider`**: (Dependency) Manages fetching transaction data for addresses via `ProcessedTransactionProvider` and calculating activity metrics (net ETH transfers, metadata, category). It caches results per address.
-   **`AddressDataFetcher`**: (Dependency used by `AddressActivityProvider`) Fetches metadata (like contract status, tags) for addresses.
-   **`ProcessedTransactionProvider`**: (Dependency used by `AddressActivityProvider`) Provides processed transaction data, which includes inferred ETH transfers based on transaction value and internal traces.

## Key Concepts

-   **Levels**: Nodes are assigned a level representing the minimum number of significant ETH transfer hops from the seed address (Level 0).
-   **ETH Threshold (`eth_threshold`)**: A configurable minimum ETH amount (default: 0.05 ETH) for a net transfer between two addresses to be considered significant and included as an edge.
-   **Node Attributes**: Nodes store `level`, `category` (Wallet, Contract, CEX, etc.), `color`, `shape`, and `metadata` (including labels, tags fetched via `AddressDataFetcher`).
-   **Edge Attributes**: Edges represent aggregated ETH flow and store `amount` (total ETH transferred), `tx_count` (number of unique transactions contributing), `type` ("eth_transfer"), and `color`. A deterministic `key` (e.g., "src:dst") ensures aggregation.
-   **Aggregation**: Multiple ETH transfers between the same `src` and `dst` are combined into a single edge, summing the `amount` and `tx_count`.
-   **Shortest Path Level**: If a node is encountered again via a shorter path from the seed, its `level` is updated to the minimum.

## Algorithm Steps

### 1. Initialization (`build_init_network`)

1.  **Input**: `seed_address`, block range parameters (`num_blocks`, `start_block`, `end_block`).
2.  **Clear Graph**: Empty the internal `networkx.MultiDiGraph`.
3.  **Ingest Seed (Level 0)**:
    -   Call `_ingest_address(seed_address, level=0, ...)`.
4.  **Identify Level 1 Neighbors**: Get immediate successors and predecessors of the seed node *resulting from its ingested activity*.
5.  **Ingest Neighbors (Level 1)**:
    -   For each unique neighbor identified in step 4:
        -   Call `_ingest_address(neighbor_address, level=1, ...)`.
6.  **Format Output**:
    -   Call `_format_network_for_output()` to generate the response dictionary (`{nodes, links, expansion_candidates}`).

### 2. Expansion (`expand_network`)

1.  **Input**: `address_to_expand`, block range parameters.
2.  **Check Node**: Verify `address_to_expand` exists in the graph. If not, return current formatted output.
3.  **Get Current Level**: Retrieve the current `level` of `address_to_expand` from the graph node attributes.
4.  **Re-Ingest Target Node**: (Optional but present in code for robustness)
    -   Call `_ingest_address(address_to_expand, level=current_level, ...)` to ensure its activity data is up-to-date.
5.  **Identify New Neighbors**: Get successors and predecessors of `address_to_expand` based on its ingested activity.
6.  **Ingest New Neighbors (Level N+1)**:
    -   For each unique neighbor identified in step 5:
        -   Call `_ingest_address(neighbor_address, level=current_level + 1, ...)`.
7.  **Format Output**:
    -   Call `_format_network_for_output()` to generate the updated response.

### 3. Address Ingestion (`_ingest_address`)

1.  **Input**: `address`, `level`, block parameters.
2.  **Fetch & Process Transactions**: Call `address_activity_provider.process_address_transactions(...)` for the given address and block range. This triggers fetching/processing relevant transactions and updates the internal state of the `AddressActivity` object managed by the provider.
3.  **Get Activity**: Retrieve the `AddressActivity` summary object for the `address` from the `address_activity_provider`.
4.  **Ensure Node**: Call `_ensure_node(activity, level)` to add the node to the graph if new, or update its level if a shorter path is found. Ensures essential attributes (`category`, `color`, `shape`, `metadata`) are present.
5.  **Ensure Edges**: Iterate through the `activity.net_eth_transfers_with_addresses` dictionary provided by the `AddressActivity` object.
    -   For each `counter_address` where `abs(signed_net_eth_transfer) >= self.eth_threshold`:
        -   Determine `src`, `dst`, and positive `amt` based on the sign of the net transfer.
        -   Retrieve the set of `tx_hashes` contributing to this net transfer from the `activity` object.
        -   Call `_ensure_edge(src, dst, amt, tx_hashes)` to add a new edge or update an existing one by aggregating `amount` and `tx_count`.

### 4. Node Handling (`_ensure_node`)

1.  **Input**: `activity` object, `level`.
2.  **Check Existence**: Check if `activity.address` is already in the graph.
3.  **Node Exists**: Update its `level` attribute to `min(current_level, level)`. Ensure other attributes (`category`, `color`, `shape`, `metadata`) are present, initializing them from the `activity` object if they were missing (e.g., if the node was previously added implicitly via an edge).
4.  **Node is New**: Fetch metadata via `activity.get_metadata()`. Add the node using `graph.add_node()` with `level` and all attributes derived from the `activity` object.

### 5. Edge Handling (`_ensure_edge`)

1.  **Input**: `src`, `dst`, `amt_eth`, `tx_hashes` set.
2.  **Generate Key**: Create a deterministic key `f"{src}:{dst}"`.
3.  **Check Existence**: Use `graph.has_edge(src, dst, key=key)`.
4.  **Edge Exists**: Aggregate values: Add `amt_eth` to the existing `amount`. Increment `tx_count` by the number of *new* transaction hashes (`len(tx_hashes - existing_tx_hashes)`). Update the internal `tx_hashes` set on the edge.
5.  **Edge is New**: Add the edge using `graph.add_edge()` with the provided `key`, initial `amount=amt_eth`, `tx_count=len(tx_hashes)`, the `tx_hashes` set (stored internally but not typically included in final output), `type="eth_transfer"`, and a default `color`.

### 6. Output Formatting (`_format_network_for_output`)

1.  **Format Nodes**: Iterate through `graph.nodes(data=True)`. For each node, retrieve its attributes (`level`, `category`, `color`, `shape`, `metadata`) safely using `.get()` with defaults or fallbacks from the corresponding `AddressActivity` object. Also retrieve `total_eth_volume` from the activity object to use as `magnitude`. Create a list of node dictionaries.
2.  **Format Links**: Iterate through `graph.edges(keys=True, data=True)`. For each edge, create a link dictionary containing `source`, `target`, `key`, and edge attributes (`type`, `amount`, `tx_count`, `color`) retrieved safely using `.get()`. **Crucially, omit the internal `tx_hashes` set from the output.** Create a list of link dictionaries.
3.  **Get Expansion Candidates**: Call `_get_expansion_candidates()`.
4.  **Return**: `{"nodes": nodes, "links": links, "expansion_candidates": candidates}` dictionary, ready for JSON serialization.

### 7. Candidate Generation (`_get_expansion_candidates`)

1.  **Initialize `cand` list.**
2.  **Iterate Graph Nodes**: For each `addr` in `graph.nodes`:
    -   Get the corresponding `AddressActivity` object (`act`).
    -   **Skip if `act.is_cex` is true.**
    -   Safely get the node's `level` from the graph attributes (`graph.nodes[addr].get("level", 0)`).
    -   Calculate the degree (number of unique neighbors).
    -   Append a dictionary containing `id`, `level`, `category`, `neighbor_count`, `magnitude` (from `act.total_eth_volume`), and `metadata` (summary from `act.get_summary()`) to the `cand` list.
3.  **Sort Candidates**: Sort `cand` list primarily by `level` (ascending) and secondarily by `magnitude` (descending).
4.  **Return**: The sorted candidate list.

## Graph Insights (`FundFlowNetworkInsights`)

Once a graph is built using `FundFlowNetworkBuilder`, the `FundFlowNetworkInsights` class provides methods to query semantic information directly from the graph structure without further data fetching:

-   **`get_upstream_sources(target, max_depth=6, stop_at_cex=True)`**: Performs a breadth-first search *backwards* from the `target` address against the direction of edges. It aggregates the total ETH amount flowing *into* the target from each upstream source discovered within `max_depth`, optionally stopping the traversal at CEX nodes.
-   **`get_downstream_sinks(source, max_depth=6, stop_at_cex=True)`**: Performs a breadth-first search *forwards* from the `source` address along the direction of edges. It aggregates the total ETH amount flowing *out of* the source to each downstream sink discovered within `max_depth`, optionally stopping at CEX nodes.
-   **`paths_to_cex(start, direction='out', max_depth=6)`**: Enumerates all *simple* (no repeated nodes) paths from the `start` address to the first CEX node encountered, following either outgoing (`direction='out'`) or incoming (`direction='in'`) edges up to `max_depth`. Returns a list of paths, including the total amount transferred along each path.
