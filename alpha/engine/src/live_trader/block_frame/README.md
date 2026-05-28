# Block Frame

This folder documents the live block boundary contract for the alpha trader.

A block frame starts when chain-server has finished applying mined block `N`.
At that point chain-server has already updated:

- tracked token state;
- tracked pool snapshots;
- gas-rank samples derived from processed block data;
- the chain-server `LiveTxSimulator` state for block `N`.

## Upstream Live-Tail Data Flow

```text
reth/node publishes new execution head N
  -> chain-server receives it
  -> chain-server fetches block N by hash
  -> chain-server fetches prestate diffs for the same hash
  -> chain-server updates LiveTxSimulator session for N
  -> chain-server updates token/pool state for N
  -> chain-server builds/publishes LiveBlockFrame N
  -> Alpha live backtest / Alpha real consume next frame N
  -> strategies make decisions pinned to N/hash
  -> chain-sim mode settles via chain-server simulation
  -> real mode builds tx plan and submits through Kartal
  -> Kartal validates/signs/broadcasts according to policy
```

Alpha must only treat block `N` as strategy-visible after that upstream sequence
has finished. The key ordering is simulator state first, token/pool updates
second, block-applied notification last.

Alpha must treat the block frame as the unit of work. Strategy decisions for a
pool update from block `N` must carry block `N` as their observed block. The
live backtest simulator may later settle a submitted execution at `N+1`, but the
strategy decision itself remains a block `N` decision.

The current API split is:

- `/api/v1/eth/live-trading/block-frames/next`: alpha strategy input. This
  waits for or returns the next replayable block frame after `after_block`.
  This is the push/long-poll boundary; Alpha does not fetch an unpinned latest
  pool surface to discover confirmed-chain strategy events.
- `/api/v1/eth/live-trading/block-frames/latest`: latest recorded block frame
  for supervision/debugging.
- `/api/v1/eth/live-trading/block-frames/{block}`: replay a retained block
  frame by block number.
- `/api/v1/eth/live-token-tracker/pools`: latest live pool surface. This is not
  a block delta and must not drive alpha confirmed-chain decisions.
- `/api/v1/eth/live-tx-simulator/simulations/unsigned-transaction`: exact-block live simulation owned by
  chain-server. Real live trading uses this boundary for pre-submit checks.
- `/api/v1/eth/live-tx-simulator/simulations/alpha-order`: exact-block order simulation owned by
  chain-server. Chain-sim live backtests use this for submitted buy/sell
  settlement.

For both real trading and chain-sim live backtesting, chain-server owns live
state, `LiveTxSimulator`, and simulation sessions; Alpha owns strategy
decisions, tx planning, gas policy, and submission lifecycle. Alpha sends small
exact-block simulation requests and receives gas/log/revert or execution-report
evidence.

The live decision path now uses the block-frame payload. Mempool signals remain
their own speculative input; they are annotated from Alpha's block-frame pool
cache rather than by fetching the latest pool list every loop.
