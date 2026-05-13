# Block Context Loader

Shared logic for reconstructing block headers and state when local Reth context
has not yet exposed the requested block.

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
     loop when the block is available through local historical context.
   * If the block is ahead of local historical context:
     - Fetch the exact `ChainStateSnapshot` written by the live block processor.
     - Restore a `ForkedState` by opening the snapshot's persisted base block
       and applying the serialized REVM cache overlay.
     - Reject stale or mismatched snapshots instead of approximating state.

3. **Reuse everywhere**
   * `contract_simulation`, `single_tx::unsigned`, block tracer, token
     metadata, etc., should all call these helpers instead of directly touching
     MDBX/Redis.

## Implementation Notes

- The live block processor builds tracked state from `prestateTracer` diffMode
  results, so the simulator restores post-block state without replaying a Redis
  window on every call.
- `LiveTxSimulator::latest_state_status()` should be used by latency-sensitive
  callers to confirm whether a request is using tracked live state or local
  historical context.
