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
