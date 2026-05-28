# Live Simulation

This module owns the chain-server side of live transaction simulation for
Alpha real trading.

Boundary:

- chain-server owns live state, `LiveTxSimulator`, and exact block simulation
  sessions;
- Alpha owns strategy decisions, tx planning, gas policy, and Kartal
  submission.

Flow:

```text
chain-server processes block B
  -> LiveBlockProcessor fetches/processes the mined block into ProcessedBlock B
  -> LiveBlockProcessor fetches prestateTracer diffMode frames for B
  -> LiveTokenRuntime receives LiveProcessedBlock { ProcessedBlock B, state_diffs }
  -> builds BlockStateSession B from the processed header, block context, and diffs
  -> publishes BlockStateSession B into chain-server LiveTxSimulator
  -> updates token/pool state for B
  -> Alpha sends small exact-block unsigned tx simulation requests when needed
  -> chain-server branches from LiveTxSimulator state B and returns the result
```

The live block processor does not pass a full serialized state blob to the
simulator. It fetches exact per-transaction prestate diffs for the mined block
with `debug_traceBlockByHash` using `prestateTracer` in `diffMode`, keyed by
the same block hash used to process the block. Those frames, together with the
processed block header and the parent state/session, are enough to build the
exact post-block `BlockStateSession`.

`LiveTokenRuntime` builds the direct live session in one of two ways:

- if the direct parent session `B-1` is still in memory, clone/advance that
  session with block `B` prestate diffs;
- otherwise, load the exact parent state `B-1` from local Reth/MDBX and apply
  block `B` prestate diffs.

Both paths validate block number, block hash, and parent hash before accepting
the session. Applying the diffs mutates the forked REVM overlay with account
balance, nonce, code, storage, created-account, and deleted-account changes.

The handoff into the simulator is:

```text
BlockStateSession B
  -> LiveBlockState::new(session).with_block_hash(block_hash)
  -> LiveTxSimulator.provider().publish_latest(...)
```

`publish_latest` stores the live state in the simulator's small in-memory block
window and notifies waiters for that block. Later exact-block simulation calls
use `LiveTxSimulator.start_chain_at(B)`, which branches from the stored
`BlockStateSession B`.

The publish step is synchronous with live-tail block application. It happens as
soon as chain-server has the processed block and its prestate diffs in memory,
before token/pool state is updated and before any `BlockApplied` notification
can wake Alpha. This keeps the simulation state and the later pool snapshots on
the same block boundary.

The API must reject missing or stale exact block state. It must not silently
fall back to historical Reth state for real live pre-submit simulation.
