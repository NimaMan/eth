# State

Live trader state is split by responsibility:

- chain-server owns mined token, pool, gas-rank, and live simulation state;
- `TokenServerClient` reads live status, latest pool snapshots, mempool signals,
  and block-update notifications;
- `live_state.rs` subscribes to the live simulation state stream and publishes
  exact block sessions into `LiveTxSimulator`;
- the execution adapter keeps the latest pool snapshots needed to build swap
  parameters for a token/pool;
- Postgres is the source of truth for order intents, submitted reports,
  position state, strategy decisions, and final execution reports.

The in-memory live simulation provider keeps a small exact-block window, not
only a single global latest block. This is required because chain-sim settlement
targets a specific block: if a strategy submitted at block `N`, the live
backtest normally settles against block `N+1`. If the main loop observes
`N+2` before settlement code runs, the simulator must still be able to select
the exact `N+1` state from the window.

The window is not a historical fallback. It only contains live block sessions
that chain-server has just published through the live-state stream. If a target
block is missing from the window, settlement stays pending and logs the missing
state instead of fabricating an execution result from a different block.

Live state construction is strict about parent state:

- frame `N` is built by advancing the exact in-memory parent session `N-1`
  when that parent is still in the live-state window;
- startup/bootstrap may build frame `N` from local Reth only when the exact
  historical parent state and header for `N-1` are available and the parent
  hash matches the frame;
- if neither exact parent source is available, no live state frame is published
  for `N`; chain-sim settlement waits instead of simulating against an older
  historical base.
