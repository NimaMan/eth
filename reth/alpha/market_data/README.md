# Market Data

Planned crate: `eth_market_data`

This crate is the confirmed-chain state producer. It combines block processing and token/pool projection because both belong to the same canonical state boundary.

## Responsibilities

- Subscribe to or fetch confirmed blocks.
- Run transaction/log/trace processing.
- Update `eth_token` token and pool state.
- Build token and pool snapshots.
- Build live chain-state overlays for fresh simulation.
- Write canonical live state through `eth_live_state`.
- Emit `MarketEvent`s for downstream consumers.

## Non-Responsibilities

- No strategy decisions.
- No portfolio state.
- No transaction signing.
- No pending mempool simulation.
- No speculative scam predictions as canonical state.

## Why Block Processor And Token Tracker Belong Together

Token state is a confirmed-chain projection. The block processor sees the ordered block data; the token tracker applies that data to token/pool state. Splitting those too early would create extra synchronization without improving correctness.

## Output Contract

For each processed block:

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

## Lessons From Python

The Python `LiveBlockTokenProcessor` plus `LiveTokenTracker` already showed the value of:

- queueing per-block token updates
- writing Redis snapshots for restart recovery
- publishing lightweight invalidation messages

The Rust version should keep that pattern but narrow the role: market data produces confirmed state and events, nothing more.
