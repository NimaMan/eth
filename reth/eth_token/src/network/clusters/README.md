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
