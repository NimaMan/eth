# Runtime

Runtime owns the `AlphaEngine` event loop and order execution flow.

- `mod.rs`: applies market/risk/execution events and runs strategies.
- `event_flow.rs`: small helpers for event block numbers and submitted reports.
- `execution_flow.rs`: applies strategy decisions, risk policy, execution reports,
  pending reports, and position state updates.
