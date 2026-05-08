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
