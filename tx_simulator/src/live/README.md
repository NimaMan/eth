# Live Simulation

The `live` module is the entry point for latency-sensitive callers that want a
latest-state simulation surface.

`LiveTxSimulator` currently selects the latest local historical context exposed
by Reth. Direct live processors that already hold headers and `prestateTracer`
diffMode output should use the lower-level block state session APIs:

- `block_state_session_from_prestate_diffs`
- `block_state_session_from_parent_prestate_diffs`

Those APIs keep the live block pipeline in-process and avoid external cache
hydration inside `tx_simulator`.
