# Live Feed

Crate: `eth_live_feed`

This crate is the live confirmed-chain feed consumed by the trading system. It combines processed blocks, token/pool state updates, optional live-state writes, and typed downstream events.

The live block processor still owns Redis population for processed blocks and live chain-state overlays. This crate owns the consumer-side token runtime: warm from processed-block cache, tail the canonical Redis block stream, update token/pool state, and expose read-only state/events to the token server and mempool consumers.

## Responsibilities

- Subscribe to confirmed processed-block notifications from Redis.
- Run transaction/log/trace processing.
- Update `eth_token` token and pool state through a `BlockTokenProcessor` boundary.
- Build token and pool snapshots.
- Accept live chain-state overlays when the simulator provides them.
- Publish canonical live state through an injected `eth_live_state::LiveStateWriter` when this pipeline owns the write path.
- Emit `LiveFeedEvent`s for downstream consumers.

## Non-Responsibilities

- No strategy decisions.
- No portfolio state.
- No transaction signing.
- No pending mempool simulation.
- No speculative scam predictions as canonical state.
- No Redis live-state writer ownership while the live block processor owns that working path.

## Why Block Processor And Token Tracker Belong Together

Token state is a confirmed-chain read model. The block processor sees the ordered block data; the token tracker applies that data to token/pool state. Splitting those too early would create extra synchronization without improving correctness.

## Output Contract

For each processed block, the pipeline emits:

```text
BlockProcessed {
  block_number,
  block_hash,
  updated_tokens,
  updated_pools,
  state_snapshot_available
}
```

Downstream services can hydrate full state from `eth_live_state`.

## Runtime Flow

```text
confirmed processed-block Redis stream
  -> processed block input
  -> block token processor
  -> optional live-state writer
  -> live-feed event sink
  -> engine / mempool risk / monitoring
```

The current Redis writer stays with the live block processor. The live token runtime consumes its stream and cache output, then keeps canonical token/pool state in-process.

Warmup replays old confirmed blocks with the regular Reth post-block metadata provider. The pending-aware live metadata provider is only for live tail blocks, where same-block token metadata may need the simulator's pending/live overlay.

The naming should stay aligned with `eth_token`: `BlockTokenProcessor` owns one confirmed processed block at a time. `LiveBlockTokenProcessor` is the canonical writer for live token/pool state, and `LiveTokenRuntime` owns scheduling, warmup, Redis stream tailing, and read-only consumers.

`LiveTokenRuntime` mutates one `LiveBlockTokenProcessor` in place. It does not
clone the processor for every block. The state lock is held only after a
processed block has been loaded and only while applying that block and updating
progress. This keeps warmup cost proportional to block work instead of to the
full accumulated token registry.

## Lessons From Python

The Python `LiveBlockTokenProcessor` plus `LiveTokenTracker` already showed the value of:

- queueing per-block token updates
- writing Redis snapshots for restart recovery
- publishing lightweight invalidation messages

The Rust version should keep that pattern but narrow the role: live feed produces confirmed-chain events, nothing more.
