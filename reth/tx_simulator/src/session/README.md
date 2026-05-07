## Sessions

The `session` module is the preferred high-level surface for long-lived simulation work.

### SimulationSession

`TxSimulator::simulation_session*` creates one forked state from MDBX or tracked live state.
After creation, each `step_*` call executes synchronously on that warm fork and commits only to
the in-memory overlay.

Live pipelines should usually call `LiveTxSimulator::start_latest_session()` instead. That
selects persisted MDBX when caught up, otherwise the live block processor's tracked state.

Use it when a workflow can contain any sequence of:

- unsigned transactions built locally,
- signed transactions from RPC/mempool/block data,
- read-only contract calls between steps,
- nonce and balance overrides for replay setup.

View calls use a nested no-commit overlay, so they observe the session state without advancing it.

### BlockReplaySession

`TxSimulator::block_replay_session(block)` pins block replay options in one object:

- `trace()` returns per-transaction geth/Reth trace results,
- `profile()` returns traces plus replay timing and provider-read counters,
- `execute_only_profile()` runs the no-inspector lower-bound diagnostic.

The default trace engine is the fused call tracer for `callTracer`-compatible options and the
Reth debug engine for broader geth debug options.
