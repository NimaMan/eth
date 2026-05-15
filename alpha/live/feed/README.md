# Live Feed

Crate: `eth_live_feed`

This crate is the live confirmed-chain feed consumed by the trading system. It combines processed blocks, token/pool state updates, direct live block handoff, and typed downstream events.

`eth_chain_server` owns the joined live runtime. It processes new heads, hands each processed block to this crate as a direct `LiveBlockUpdate`, updates token/pool state in-process, and exposes read-only state/events to downstream consumers.

## Responsibilities

- Consume direct `LiveBlockUpdate`s from the chain server after warmup.
- Run transaction/log/trace processing.
- Update `eth_token` token and pool state through a `BlockTokenProcessor` boundary.
- Build token and pool snapshots.
- Accept block-scoped direct live state sessions when the chain server provides prestate diffs.
- Emit `LiveFeedEvent`s for downstream consumers.

## Non-Responsibilities

- No strategy decisions.
- No portfolio state.
- No transaction signing.
- No pending mempool simulation.
- No speculative scam predictions as canonical state.
- No Redis live-block or live-state transport in the chain-server live path.

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
eth_chain_server new-head loop
  -> direct LiveBlockUpdate
  -> block token processor
  -> live-feed event sink
  -> engine / mempool risk / monitoring
```

Warmup replays old confirmed blocks from processed-block cache/Reth. After warmup, the live token runtime no longer tails Redis; the chain server calls `apply_live_block_update` directly for each processed live block.

Warmup replays old confirmed blocks with the regular Reth post-block metadata provider. Live tail blocks use the direct live block session built from the processed block header plus RPC state diffs, so same-block token and V2 pool metadata comes from the in-process post-block state before falling back to local Reth. Redis pending replay is not part of this path.

The naming should stay aligned with `eth_token`: `BlockTokenProcessor` owns one confirmed processed block at a time. `LiveBlockTokenProcessor` is the canonical writer for live token/pool state, and `LiveTokenRuntime` owns scheduling, warmup, direct live block application, and read-only consumers.

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
