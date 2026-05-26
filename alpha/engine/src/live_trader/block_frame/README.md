# Block Frame

This folder documents the live block boundary contract for the alpha trader.

A block frame starts when chain-server has finished applying mined block `N`.
At that point chain-server has already updated:

- tracked token state;
- tracked pool snapshots;
- gas-rank samples derived from processed block data;
- the chain-server `LiveTxSimulator` state for block `N`.

Alpha must treat the block frame as the unit of work. Strategy decisions for a
pool update from block `N` must carry block `N` as their observed block. The
live backtest simulator may later settle a submitted execution at `N+1`, but the
strategy decision itself remains a block `N` decision.

The current API split is:

- `/eth/tokens/api/live/updates`: block-applied notification. This is the loop
  wake-up source for chain-sim live backtests.
- `/eth/tokens/api/live/pools`: latest live pool surface. This is not a block
  delta and can include newer state than the block that woke the loop.
- `/eth/tokens/api/live/state/stream`: compatibility live simulation state
  stream. Chain-sim live backtests can use it to publish exact in-memory block
  sessions into their local `LiveTxSimulator`.
- `/api/v1/eth/live/simulations/unsigned`: exact-block live simulation owned by
  chain-server. Real live trading uses this boundary instead of rebuilding live
  state inside Alpha.

For real trading, chain-server owns live state, `LiveTxSimulator`, and
simulation sessions; Alpha owns strategy decisions, tx planning, gas policy, and
Kartal submission. Alpha sends small exact-block unsigned transaction
simulation requests and receives gas/log/revert evidence.

The desired end state is a single chain-server block-frame payload containing
the updated token and pool snapshots for block `N`. Until that API exists,
real trading uses chain-server simulation as the ordering fence, while
chain-sim live backtests may still use the live-state stream.
