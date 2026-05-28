# Live Feed Runtime

This folder owns the in-process live token runtime used by chain-server.

Boundary:

- chain-server owns mined block ingestion, token/pool state, and the server-side
  `LiveTxSimulator`;
- callers consume status, token/pool snapshots, block-applied events, and exact
  live simulation sessions through `LiveTokenRuntime`;
- Alpha live traders should not rebuild this live state locally for normal
  live execution paths.

## Module Map

- `service.rs`: runtime construction, start/stop, warmup loop, public progress,
  state, and exact simulation access.
- `block_update.rs`: `LiveBlockUpdate`, the live-tail handoff object from
  chain-server block processing into this runtime.
- `block_apply.rs`: block application, token/pool processing, bottleneck logs,
  and block-applied event emission.
- `direct_live_state.rs`: builds and publishes direct live
  `BlockStateSession`s into the server-owned `LiveTxSimulator`.
- `lifecycle.rs`: runtime live/stopped/failed state transitions.
- `reader.rs`: `LiveTokenReader` trait and snapshot reader implementation.
- `apply_report.rs`: translates token processor reports into progress counters
  and block events.
- `progress.rs`, `state.rs`, `snapshot.rs`, `event.rs`, `config.rs`: runtime
  data types.
- `errors.rs`, `helpers.rs`, `time.rs`: local support helpers.

## Live-Tail Data Flow

```text
chain-server receives execution head B
  -> LiveBlockProcessor fetches/processes block B by block hash
  -> LiveBlockProcessor fetches prestate diff frames for B by the same block hash
  -> LiveChainRuntime writes processed-block cache if needed
  -> LiveTokenRuntime::apply_live_block_update(LiveBlockUpdate B)
  -> block_apply.rs calls direct_live_state.rs first
  -> direct_live_state.rs builds BlockStateSession B
  -> direct_live_state.rs publishes LiveBlockState B into LiveTxSimulator
  -> block_apply.rs processes token/pool updates for B
  -> apply_report.rs updates progress and creates BlockApplied event B
  -> event subscribers are notified that block B is ready
```

The direct live simulation state is intentionally published before token/pool
updates and before the `BlockApplied` event is sent. That ordering gives callers
one clear block boundary: when block `B` is visible to strategy processing, the
server-owned simulator should already be able to simulate exact state for `B`.

## Direct Live State

`direct_live_state.rs` builds the exact post-block simulation state from:

- the processed block header and hash;
- the processed block parent hash;
- per-transaction prestate diff frames;
- either the cached direct parent `BlockStateSession B-1` or local Reth parent
  state.

The normal fast path advances cached direct parent session `B-1` with the
prestate diffs for block `B`. The fallback path builds from the canonical parent
state in local Reth and applies the same block `B` diffs.

This state is stored in the `LiveTxSimulator` in-memory window. Exact simulation
calls later use `LiveTxSimulator.start_chain_at(B)` through
`LiveTokenRuntime::live_simulation_chain_at(B)`.

## Reorg Rules

The runtime must never mix sessions from different canonical branches.

Required behavior for reorg-safe ownership:

- a same-height block with a different hash is a replacement head, not a
  duplicate;
- if cached direct parent `B-1` exists but its hash does not match block `B`'s
  parent hash, stale sessions from `B-1` onward must be pruned before rebuilding
  or serving simulations;
- prestate diffs used for block `B` must correspond to the same block hash that
  was processed;
- if exact state cannot be proven for a block, the simulator API must return an
  unavailable/reorg result rather than falling back to a different block.

Until those reorg rules are fully implemented, a caller should treat missing
exact state as infrastructure evidence, not as an on-chain failed trade.

## Warmup Data Flow

Warmup uses historical processed blocks:

```text
LiveTokenRuntime::start(...)
  -> resolve warmup range
  -> clear direct live sessions and LiveTxSimulator state
  -> load each warmup block through processed-block cache/Reth
  -> process token/pool state without publishing direct live simulation sessions
  -> mark runtime live
```

Warmup primes token and pool tracking. Direct live simulation sessions are only
published for live-tail blocks that include live prestate diffs.

## Public Runtime Surfaces

- `progress()`: current runtime status and counters.
- `state()`: read lock over live token state for read models.
- `subscribe()`: block-applied and lifecycle events.
- `live_simulation_chain_at(block)`: exact server-owned live simulation chain
  for a block already in the live state window.

The runtime does not guarantee historical simulation fallback from this exact
live surface. Missing exact state should stay explicit.
