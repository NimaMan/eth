# snapshots

Serializable output views for Asena, strategy code, and diagnostics.

This folder owns stable snapshot structs built from graph, activity, and cluster
state. Snapshots should be compact, explainable, and independent of any specific
frontend layout.

## Files

- `token.rs`: one-token network summary at a block.
- `address.rs`: address activity and label snapshot.
- `edge.rs`: collapsed edge/evidence snapshot.
- `cluster.rs`: direct and inferred cluster snapshot.
- `pool.rs`: pool-centric trader, LP, liquidity, and control-actor view.
- `risk.rs`: explainable network risk features.

## Boundaries

- Does not recompute raw graph state.
- Does not fetch labels or prices.
- Does not expose frontend-specific coordinates, colors, or table state.
- Should be versionable if the snapshot shape becomes an API contract.

## First Implementation Target

Define the first Asena-facing snapshot shape for holder graph inspection:
addresses, collapsed links, suppressed hubs, connected components, and summary
counts.
