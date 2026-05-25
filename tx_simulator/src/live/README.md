# Live Simulation

The `live` module is the entry point for latency-sensitive callers that want a
latest-state simulation surface.

`LiveTxSimulator` is live-only. It uses a small in-memory window of mined
`BlockStateSession` values published by the live block processor and must not
select headers or state from local Reth historical context. If the requested
block is unavailable in that live window, it fails rather than falling back.

The window exists for block-coupled live trading. A strategy can submit at block
`N` and settle in block `N+1`; if the main loop reaches `N+2` before settlement
runs, `LiveTxSimulator` must still select the exact `N+1` live state. This is
not a historical query path. Only state frames published by the live block
processor are eligible.

Historical/latest-Reth callers should use `TxSimulator` directly or the explicit
`LatestHistoricalTxSimulator` adapter.

Direct live processors build sessions with:

- `block_state_session_from_prestate_diffs`
- `block_state_session_from_parent_prestate_diffs`

Those APIs keep the live block pipeline in-process and avoid external cache
hydration inside `tx_simulator`.
