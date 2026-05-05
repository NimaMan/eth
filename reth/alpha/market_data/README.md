# Market Data

Crate: `eth_market_data`

This crate is the confirmed-chain state producer. It combines block processing and token/pool state updates because both belong to the same canonical state boundary.

For now this crate does not replace the working Redis population inside `tx_simulator` and the existing live processors. It defines the pipeline contracts that let those producers and future Rust services plug into one flow without duplicating runtime orchestration.

## Responsibilities

- Subscribe to or fetch confirmed blocks.
- Run transaction/log/trace processing.
- Update `eth_token` token and pool state through a token-state processor boundary.
- Build token and pool snapshots.
- Accept live chain-state overlays when the simulator provides them.
- Publish canonical live state through an injected `eth_live_state::LiveStateWriter` when this pipeline owns the write path.
- Emit `MarketEvent`s for downstream consumers.

## Non-Responsibilities

- No strategy decisions.
- No portfolio state.
- No transaction signing.
- No pending mempool simulation.
- No speculative scam predictions as canonical state.
- No Redis client implementation while `tx_simulator` owns that working path.

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
confirmed block source
  -> processed block input
  -> token state processor
  -> optional live-state writer
  -> market-data event sink
  -> engine / mempool risk / monitoring
```

The current Redis writer can stay in `tx_simulator`. The important Rust migration is that new components consume typed events and snapshots instead of reaching into Python-era process-local caches.

## Lessons From Python

The Python `LiveBlockTokenProcessor` plus `LiveTokenTracker` already showed the value of:

- queueing per-block token updates
- writing Redis snapshots for restart recovery
- publishing lightweight invalidation messages

The Rust version should keep that pattern but narrow the role: market data produces confirmed state and events, nothing more.
