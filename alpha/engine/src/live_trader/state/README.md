# State

Live trader state is split by responsibility:

- chain-server owns mined token, pool, gas-rank, and live simulation state;
- `TokenServerClient` reads block-pinned live trading frames, live status,
  exact simulation responses, and mempool signals;
- the execution adapter keeps the latest block-frame pool snapshots needed to
  build swap parameters for a token/pool;
- Postgres is the source of truth for order intents, submitted reports,
  position state, strategy decisions, and final execution reports.

Alpha does not run a local `LiveTxSimulator`. The in-memory live simulation
window lives inside chain-server. Chain-sim settlement targets a specific
block: if a strategy submitted at block `N`, the live backtest normally settles
against block `N+1`. Alpha asks chain-server to simulate the stored order at
that exact block and includes the submitted block hash when available so
chain-server can reject reorged parent state.

The chain-server simulator window is not a historical fallback. It only
contains exact live block sessions produced by the live block processor. If a
target block is missing or its parent hash no longer matches the submitted
block hash, chain-server returns an explicit unavailable/reorg response. Alpha
keeps the submitted execution pending and logs the infrastructure gap instead
of fabricating a fill from a different block.

Chain-sim live backtests use the chain-server block-frame boundary:

```text
read LiveBlockFrame N from /api/v1/eth/live-trading/block-frames/next
  -> publish pool snapshots into the execution adapter
  -> run strategy decisions and chain-sim execution
  -> persist submitted reports with submitted block/hash and expected block N+1
  -> settlement asks chain-server to simulate the order at exact block N+1
```

No Alpha component subscribes to live-state stream frames or waits on a local
simulation provider. Real live trading uses the same ownership boundary for
pre-submit unsigned transaction simulation: chain-server owns
`LiveTxSimulator`; Alpha owns decisions, tx planning, gas policy, and submission.

Live state construction is strict about parent state:

- frame `N` is built by advancing the exact in-memory parent session `N-1`
  when that parent is still in the live-state window;
- startup/bootstrap may build frame `N` from local Reth only when the exact
  historical parent state and header for `N-1` are available and the parent
  hash matches the frame;
- if neither exact parent source is available, no live state frame is published
  for `N`; chain-sim settlement waits instead of simulating against an older
  historical base.
