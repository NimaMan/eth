# Block Frame

This folder documents the live block boundary contract for the alpha trader.

A block frame starts when chain-server has finished applying mined block `N`.
At that point chain-server has already updated:

- tracked token state;
- tracked pool snapshots;
- gas-rank samples derived from processed block data;
- the in-memory live simulation state for block `N`.

Alpha must treat the block frame as the unit of work. Strategy decisions for a
pool update from block `N` must carry block `N` as their observed block. The
live backtest simulator may later settle a submitted execution at `N+1`, but the
strategy decision itself remains a block `N` decision.

The current API split is:

- `/eth/tokens/api/live/updates`: block-applied notification. This is the loop
  wake-up source for chain-sim live backtests.
- `/eth/tokens/api/live/pools`: latest live pool surface. This is not a block
  delta and can include newer state than the block that woke the loop.
- `/eth/tokens/api/live/state/stream`: live simulation state stream. Alpha uses
  it to publish exact in-memory block sessions into `LiveTxSimulator`.

The desired end state is a single chain-server block-frame payload containing
the updated token and pool snapshots for block `N`. Until that API exists,
alpha stores the latest fetched pool snapshots in the execution adapter before
processing events, and exact execution settlement is gated on the simulator
having the exact target block state.
