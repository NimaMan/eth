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
  -> fetches prestate diffs for B
  -> builds BlockStateSession B from the processed block header, block context, and diffs
  -> publishes BlockStateSession B into chain-server LiveTxSimulator
  -> updates token/pool state for B
  -> Alpha sends small exact-block unsigned tx simulation requests when needed
  -> chain-server branches from LiveTxSimulator state B and returns the result
```

The publish step is synchronous with live-tail block application. It happens as
soon as chain-server has the processed block and its prestate diffs in memory,
before token/pool state is updated and before any `BlockApplied` notification
can wake Alpha. This keeps the simulation state and the later pool snapshots on
the same block boundary.

The API must reject missing or stale exact block state. It must not silently
fall back to historical Reth state for real live pre-submit simulation.
