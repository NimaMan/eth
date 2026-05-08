# model

Pure data model for the token network.

This folder owns identifiers, node kinds, edge kinds, labels, and evidence
records. It should not know how to ingest transactions, simplify graphs, compute
clusters, or produce UI-specific output.

## Files

- `ids.rs`: stable IDs for addresses, pools, tokens, synthetic nodes, and edges.
- `nodes.rs`: node kinds and node metadata.
- `edges.rs`: edge kinds and collapsed edge metadata.
- `labels.rs`: address, pool, and actor labels plus confidence metadata.
- `evidence.rs`: raw evidence attached to nodes, edges, and inferred clusters.

## First Implementation Target

Define serializable structs and enums with stable names that can be reused by
the graph, cluster, and snapshot layers without creating circular dependencies.

## Current Contract

The model layer now exposes:

- stable token, node, pool, time-window, and edge IDs;
- node kinds for token, address, pool, control actor, liquidity actor,
  intermediary, time-window, and synthetic nodes;
- edge kinds for token transfers, denom flows, pool trades, LP relations,
  fee-source links, funding, control relations, shared intermediaries, temporal
  coactivity, pool creation, and liquidity events;
- labels with source and confidence metadata;
- observations with block, timestamp, transaction, index, and log-index context;
- bounded evidence summaries for collapsed graph edges and inferred clusters.

The next layer should consume these types rather than introducing parallel graph
identifiers or ad hoc JSON fields.
