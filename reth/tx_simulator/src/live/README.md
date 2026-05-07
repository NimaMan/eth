# Live Simulation

The `live` module is the entry point for latency-sensitive pipelines where the
target state can be ahead of the locally persisted Reth MDBX database.

`LiveTxSimulator` chooses state with one rule:

1. Use MDBX when MDBX is caught up.
2. Otherwise use the state tracked by the live block processor.

If the live head is ahead of MDBX but the tracked live state is missing or
stale, live simulation returns an error instead of falling back to stale MDBX
state.

Regular `TxSimulator` APIs remain the lower-level historical/direct-DB surface.
Live pipelines should depend on `tx_simulator::live::LiveTxSimulator` so latest
state selection stays explicit.

Useful entry points:

- `latest_state_status()` reports the selected block, selected source, latest
  persisted block, live head, and latest tracked state block.
- `start_latest_session()` opens a mixed signed/unsigned `SimulationSession`
  at the selected live-first state.
- `simulate_sequence()` and `simulate_mixed_sequence()` use that live-first
  session path, so the live processor's tracked state is used whenever MDBX lags.
