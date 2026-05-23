# Live V2 Metadata Bottleneck

Archived postmortem. Redis references below describe the old live-state
fallback path that caused the bottleneck. Current live block processing uses
direct in-process handoff and does not use Redis live-state snapshots.

## Summary

Live Uniswap V2 pool metadata lookups are cheap only when the requested block is
already readable from Reth historical state. At the live tail, the lookup can be
forced through the live-state fallback path. That path can read and deserialize a
large Redis chain-state snapshot before running a tiny `token0()` or `token1()`
view call.

The immediate mitigation is to keep the live V2 pool metadata timeout low. As of
2026-05-13, `LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS` is `250ms`.

The real fix was to stop using Redis-serialized live state as the normal
in-process path between the live block processor and the live token tracker.

## Observed Evidence

From the 2026-05-13 live run before lowering the timeout:

- V2 metadata timeout warnings: `944`
- Slow block apply bottleneck samples: `269`
- Slow blocks overlapping metadata timeout blocks: `267 / 269`
- Worst observed block had `9` metadata timeouts.
- With the old `2500ms` timeout, `9 * 2500ms` accounted for most of a roughly
  `24s` block apply.

Direct profiling showed the call itself is not inherently slow:

- Pool `0xcfce41372bff48addd0946b8dacd03bae3e81f85` at block `25086964`
  took `6524ms` for `token0/token1` while Reth historical state was still behind
  that block.
- The same pool/block later took about `15ms` after Reth caught up.

Redis evidence from the same machine:

- Each recent `eth/live/block/<block>/chain_state_snapshot` key was about
  `335MB`.
- Redis used memory was about `12.66GB`.

## Current Code Path

The live token tracker applies a processed block through `eth_token`:

```text
eth_live_feed runtime
  -> BlockTokenProcessor::process_block_live...
  -> ProcessedTokenUpdateRouter
  -> optional_uniswap_v2_pool_identity(...) for candidate routing
  -> optional_uniswap_v2_pool_metadata(...) only when registering a pool
  -> RethChainMetadataProvider::uniswap_v2_pool_identity/metadata(...)
  -> RethQueryProvider::uni_v2_get_tokens(...)
  -> TxSimulator::simulate_view_function(...)
  -> BlockContextLoader::load_block_context(...)
```

As of 2026-05-14, candidate routing uses the cheaper identity path. It may issue:

- `token0()`
- `token1()`
- `factory()`

Known V2 protocol validation is local CREATE2 pair-address calculation. It
should not simulate factory `getPair(token0, token1)` in the candidate path.

Full V2 metadata is still used when a pool is actually registered, and may issue:

- token decimals for each side

Before the Redis live-state cleanup, those view calls did not use one
block-scoped context. If the block was ahead of Reth's readable historical
state, each call could enter the live context loader path independently.

## Why It Gets Stuck

`TxSimulator::simulate_view_function` first prepares a block context. In the old
live-tail path, the context loader tried historical state and then fell back to
Redis live state when historical state was unavailable.

That means a cheap view call can pay for:

- Redis read of a large `chain_state_snapshot`
- bincode deserialize of hundreds of MB
- restoration of a forked state from a historical base
- merge of the live overlay
- EVM execution of the actual tiny view call

The timeout is applied around the whole metadata lookup. If the timeout fires
while a blocking task is already running, Tokio cannot necessarily cancel that
blocking work immediately; it only stops waiting on it. So lowering the timeout
reduces block-apply wait time, but it does not make the underlying state-loading
work cheap.

## Why Redis Should Not Have Been The Critical Path

The old architecture treated Redis as a process boundary and
recovery/diagnostic surface, but the hot path was doing unnecessary work:

```text
live block processor builds current live state
  -> serialize massive state snapshot to Redis
  -> token tracker reads snapshot back from Redis
  -> deserialize and rebuild forked state
  -> run a small view call
```

When both components run on the same host and can be owned by the same runtime,
that write/read/deserialize cycle is the wrong data path for live-tail token
tracking.

So a "cheap" V2 identity or metadata lookup can fetch, deserialize, and merge a
roughly 335MB live-state snapshot just to run a `token0()` or `token1()` view
call. That is not an acceptable live hot path. The problem is not only timeout
length; the data path itself is backwards.

## Real Fix Assessment

The real fix is a topology and ownership change: live token tracking should
consume the live block processor's current block state directly, not reconstruct
that state from serialized external snapshots for metadata and simulation view
calls.

The acceptable live-state source order should be explicit:

1. Use local Reth historical state when the requested block is already readable.
2. Use an in-process live block context/session when processing the current live
   block.
3. Defer optional metadata discovery when neither source is cheap.
4. Defer optional metadata when direct live state is unavailable instead of
   falling back to serialized external snapshots.

The current `BlockStateSession` pattern is the right local primitive because it
keeps one block state warm and gives callers isolated branches for individual
view calls. The old Redis-hydrated path was only a partial fix because it
reduced many expensive snapshot loads to one expensive snapshot load per block;
it did not remove the expensive snapshot from the hot path.

For correctness, deferral must be explicit. A live-tail metadata lookup that
cannot get cheap state should be recorded as `state_unavailable` or `deferred`,
not cached as "not a V2 pool." When Reth catches up, or when an in-process
context becomes available, the tracker can resolve the metadata and replay the
affected pool/token updates if needed.

## Fix Plan

### 1. Keep The Timeout Low

Keep `LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS = 250`.

This prevents multiple metadata misses from adding seconds each to block apply.
It is a mitigation, not the root fix.

### 2. Add Negative Caching For Live Metadata Misses

Cache timeout/miss results by at least:

```text
(block_number, pool_address, tracked_token_address)
```

A short-lived per-run or per-block cache is enough. The important behavior is:

- do not retry the same pool multiple times in the same block
- optionally avoid retrying the same pool for a small live-tail TTL
- record whether the skip was `timeout`, `miss`, or `state_unavailable`

This is low risk and directly reduces repeated serial waits.

### 3. Reuse One Block Context For Metadata Calls

Before changing process topology, fix the current code path to reuse one
block-scoped state/session when applying a block.

The pool simulation path already has the right shape: it keeps a
`BlockStateSession` map for the block and branches from that session. V2
metadata should use the same pattern.

Proposed local change:

- Add a block-scoped metadata provider for live block apply.
- Create or reuse one `BlockStateSession` for the current block.
- Implement V2 metadata view calls against a branch from that session instead of
  calling global `simulate_view_function` for each small view.
- Keep token decimals and V2 pool metadata caches above that provider.

This still may pay one live-state snapshot load per block when Reth is behind,
but it avoids paying it once per metadata view call.

### 4. Defer Discovery When State Is Not Cheap

For live-tail blocks, V2 pool discovery is useful but should not block token
state application.

If Reth historical state is behind and no in-process live state is available,
enqueue the metadata lookup and continue applying the block.

The deferred worker should:

- resolve the pool metadata after Reth catches up or when a cheap context exists
- apply discovered pool metadata to the registry
- optionally replay buffered affected blocks if correctness requires pool state
  history from the discovery block
- emit structured counters for `metadata_deferred`, `metadata_resolved`, and
  `metadata_expired`

This makes the live tracker prefer freshness over immediate pool discovery while
keeping a path to eventual consistency.

### 5. Co-Locate Live Block Processing And Token Tracking

Best long-term fix: run the live block processor and live token tracker in the
same process/runtime, and share live block/state objects directly in memory.

Target hot path:

```text
reth/live chain ingestion
  -> processed block + live state context
  -> in-process channel
  -> live token tracker block apply
  -> token/pool read models
```

The old plan allowed Redis as a fallback/output boundary for external
consumers, restart recovery, and diagnostics. The current implementation does
not use Redis in live block processing; token tracking should use direct live
state or defer optional metadata.

Suggested in-process interface:

```text
LiveProcessedBlockEnvelope {
  block_number,
  header,
  processed_block: Arc<ProcessedBlock>,
  block_context: Arc<BlockContext or BlockStateSession>,
  source_timestamps,
}
```

Then the live token tracker can use:

- the `ProcessedBlock` for token/pool event application
- the shared block context/session for metadata and simulation view calls
- a bounded in-memory LRU keyed by block number for recent block contexts

The context should be immutable or branchable. Do not share mutable REVM state
directly across tasks; share a session/snapshot that can cheaply create isolated
branches for individual view calls.

## Recommended Execution Order

1. Keep `250ms` timeout deployed.
2. Add miss/timeout cache for V2 metadata lookup.
3. Add profiling fields for metadata lookup source and context load time.
4. Introduce block-scoped metadata provider using the existing
   `BlockStateSession` pattern.
5. Add deferred discovery for live-tail state-unavailable cases.
6. Build the co-located live runtime and keep live block processing on direct
   in-process state.

## Logging To Keep

Do not add another log file for this unless needed. Use existing files:

- `events.jsonl`: WARN/ERROR metadata timeouts and state-unavailable events
- `pipeline_bottlenecks.jsonl`: slow block samples
- `token_pipeline_profile.jsonl`: detailed metadata/context timing rows
- `pipeline_health.jsonl`: aggregate counters in heartbeat metrics

Useful new fields:

```text
metadata_lookup_source = historical_state | in_process_live_state | deferred
metadata_context_load_ms
metadata_view_call_ms
metadata_snapshot_bytes
metadata_cache_hit
metadata_timeout_cached
```
