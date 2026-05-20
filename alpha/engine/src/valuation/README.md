# Valuation

Valuation owns position snapshots and mark-to-market helpers.

- `snapshots.rs`: builds zero-value, simulated-value, and pool-metric snapshots.
- `snapshot_flow.rs`: decides when open positions should be snapshotted for
  market updates.
