# Live Simulation

The `live` module is the entry point for latency-sensitive callers that want a
latest-state simulation surface.

`LiveTxSimulator` is live-only. It uses a small in-memory window of mined
`BlockStateSession` values published by the live block processor and must not
select headers or state from local Reth historical context. If the requested
block is unavailable in that live window, it fails rather than falling back.

Publishing a live block state also notifies waiters for that block. In the
chain-server live path, the server publishes the exact `N` session into its
`LiveTxSimulator` before publishing the corresponding token/pool update for
block `N`. Real Alpha runners do not subscribe to the full live-state stream;
they ask chain-server to simulate small exact-block unsigned transactions
against that server-owned session. Chain-sim live backtests can still use the
notification path to hydrate a local simulator.

The window exists for block-coupled live trading. A strategy can submit at block
`N` and settle in block `N+1`; if the main loop reaches `N+2` before settlement
runs, `LiveTxSimulator` must still select the exact `N+1` live state. This is
not a historical query path. Only state frames published by the live block
processor are eligible.

## Live-Tail Data Flow

```text
chain-server receives execution head B
  -> LiveBlockProcessor fetches/processes block B by block hash
  -> LiveBlockProcessor fetches prestate diff frames for B by the same block hash
  -> LiveChainRuntime writes processed-block cache if needed
  -> LiveTokenRuntime::apply_live_block_update(LiveBlockUpdate B)
  -> direct_live_state.rs builds BlockStateSession B
  -> direct_live_state.rs publishes LiveBlockState B into LiveTxSimulator
  -> LiveTxSimulator stores B in the in-memory live-state window
  -> LiveTxSimulator notifies waiters for block B
  -> later exact-block calls use LiveTxSimulator.start_chain_at(B)
```

`LiveTxSimulator` is only the storage and branching surface for the exact live
sessions. It does not fetch or repair chain state by itself; the chain-server
runtime must publish canonical block sessions in the right order.

Historical/latest-Reth callers should use `TxSimulator` directly or the explicit
`LatestHistoricalTxSimulator` adapter.

Direct live processors build sessions with:

- `block_state_session_from_prestate_diffs`
- `block_state_session_from_parent_prestate_diffs`

Those APIs keep the live block pipeline in-process and avoid external cache
hydration inside `tx_simulator`. `block_state_session_from_prestate_diffs`
requires the exact parent block state from local Reth; it must not construct a
live frame from an older historical base. When a caller already has the parent
live session, `block_state_session_from_parent_prestate_diffs` is the preferred
path.
