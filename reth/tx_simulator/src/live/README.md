# Live Simulation

The `live` module is the entry point for latency-sensitive pipelines where the
target state can be ahead of the locally persisted Reth MDBX database.

`LiveTxSimulator` chooses the latest exact state source in this order:

1. Redis chain-state overlay written by the live block processor, but only when
   that exact snapshot is ahead of persisted MDBX.
2. Latest persisted MDBX block when MDBX is caught up or no live overlay is
   available.

Regular `TxSimulator` APIs remain the lower-level historical/direct-DB surface.
Live pipelines should depend on `tx_simulator::live::LiveTxSimulator` so latest
state selection stays explicit.

Useful entry points:

- `latest_state_status()` reports the selected block, selected source, latest
  persisted block, live head, and latest exact chain-state snapshot.
- `start_latest_session()` opens a mixed signed/unsigned `SimulationSession`
  at the selected live-first state.
- `simulate_sequence()` and `simulate_mixed_sequence()` use that live-first
  session path, so Redis snapshots are the fast path whenever MDBX lags.
