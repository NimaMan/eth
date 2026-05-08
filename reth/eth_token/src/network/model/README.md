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
