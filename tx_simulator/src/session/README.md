## Sessions

The `session` module is the preferred high-level surface for long-lived simulation work.

### SimulationSession

`TxSimulator::simulation_session*` creates one forked state from MDBX or tracked live state.
After creation, each `step_*` call executes synchronously on that warm fork and commits only to
the in-memory overlay.

Live pipelines should usually call `LiveTxSimulator::start_latest_session()` instead. That
selects local historical context when caught up, otherwise the live block processor's tracked state.

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

The default trace engine reuses one call tracer for `callTracer`-compatible options and resets
its per-tx trace buffers between transactions. Broader geth debug options use the Reth debug
engine.

### BlockStateSession

`TxSimulator::block_state_session(block)` opens one forked state for a specific block state and
returns cheap unsigned simulation branches with `simulation_chain()`. This is the live token
pipeline shape: a live block processor keeps a per-block map of `BlockStateSession`s keyed by the
effective simulation block number, so repeated pool checks do not reopen the same state.

### BlockTxStateSession

`TxSimulator::block_tx_state_session(block)` is the historical token-pipeline shape. It opens the
parent block state once, loads the block transactions once, and keeps one canonical prefix state.
Calls to `simulation_chain_after_tx(tx_index)` advance that prefix monotonically and return an
isolated unsigned branch from the warmed state.

This avoids reopening the block pre-state for every token or pool simulation. The cost model is:

- one session open per block that needs a historical pool simulation,
- one monotonic signed-prefix replay up to the highest simulated tx index,
- one warmed branch clone per individual pool simulation.

Use `profile_block_tx_session` when checking whether a token pipeline slowdown is state-open,
prefix-replay, or branch-clone related:

```bash
cargo run --manifest-path blockchains/eth/Cargo.toml -p tx_simulator --release \
  --example profile_block_tx_session -- \
  --datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --block 25057078 --iterations 2 --warmup-iterations 1
```

If `session_load_ms` is low but `prefix_advance_ms` is high, the pre-state is being reused and the
remaining cost is replaying mined transactions before the simulated tx. If branch timings are low,
per-pool simulation is branching from the warmed block state as intended.
