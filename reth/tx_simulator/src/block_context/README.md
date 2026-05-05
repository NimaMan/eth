# Block Context Loader

Shared logic for reconstructing block headers and state when MDBX has not yet
indexed the requested block.

This module should expose helper functions used by every component that needs
block data (headers, state providers, etc.). The goal is to centralize the logic
for fallbacking to Redis + simulation when the local database lags behind the
live feed.

## Responsibilities

1. **load_block_header(block)**
   * Try `provider.header_by_number` first.
   * If MDBX returns `None`, pull `eth/live/block/<n>/header` from Redis and
     decode it into a `SealedHeader`.

2. **load_state_for_block(block)**
   * Attempt `provider.history_by_block_number(block)` with the current retry
     loop.
   * If the block is ahead of MDBX:
     - Determine the latest persisted block (`best_block_number`).
     - Build a `ForkedState` rooted at that block using existing helpers.
     - For each missing block (`best_block+1` ..= `block`):
       + Fetch the sealed header + transactions from Redis.
       + Replay them sequentially using `UnsignedTxChainSimulation` so the
         fork reflects the same state MDBX would eventually hold.
     - Wrap the fork’s backing database in a temporary `StateProvider`
       implementation (e.g., via `MemoryOverlayStateProvider`) so callers
       receive a `StateProviderBox`.

3. **Reuse everywhere**
   * `contract_method_simulator`, `single_tx::unsigned`, block tracer, token
     metadata, etc., should all call these helpers instead of directly touching
     MDBX/Redis.

## Implementation Notes

- Add small helpers in `live_chain_data/live_chain_cache/redis_cache.rs` to
  fetch `(SealedHeader, Vec<UnsignedTransaction>)` for a block so the replay loop
  doesn’t duplicate JSON parsing.
- Consider building a lightweight `StateProvider` wrapper around the forked
  state by embedding a `MemoryOverlayStateProvider` or similar so the rest of
  the simulator continues to operate on `StateProviderBox` without intrusive
  changes.
