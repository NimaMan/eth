# Live State

Crate: `eth_live_state`

This crate owns the shared live-state protocol used through Redis. It is infrastructure shared by live feed, simulators, mempool risk, and the trading engine.

## Responsibilities

- Define Redis key builders.
- Define snapshot schemas.
- Define reader/writer traits for live-state stores.
- Own retention and TTL policy helpers.
- Version serialized schemas.
- Provide test fixtures for compatibility.

This crate intentionally does not own the block processor, processed-block pipeline, tracked-token set, or live-token object cache. Those are writer/consumer responsibilities in higher-level crates.

`eth_live_feed` should write token snapshots and block-ready markers through these contracts when it owns the write path. The processed block itself remains `tx_processor::ProcessedBlock`; this crate must not define a second processed-block shape.

## Canonical State Namespaces

Confirmed-chain state should have one canonical writer. Today that can remain the existing simulator/live processor path; once the Rust feed owns it, that writer should be `eth_live_feed`.

```text
eth/live/latest/block_number
eth/live/latest/block_hash
eth/live/latest/chain_state_block_number
eth/live/blocks
eth/live/recent_blocks
eth/live/block/<n>/header
eth/live/block/<n>/txs
eth/live/block/<n>/chain_state_snapshot
eth/live/token/snapshot/<token>
eth/live/token/snapshot/index
```

Trading state can live separately:

```text
eth/live/position/<portfolio>/<token>
```

Speculative mempool risk should not overwrite canonical state:

```text
eth/live/risk/...
eth/live/mempool/signals/...
```

## Single Writer Rule

There should be one canonical writer for confirmed state.

Mempool risk, trading engine, and simulator may read the state, but they should not mutate canonical token/pool snapshots or block-ready markers.

## Rust Boundary

The clean dependency direction is:

```text
eth_live_feed    -> eth_live_state
tx_simulator     -> eth_live_state
eth_mempool_risk -> eth_live_state
eth_alpha_engine -> eth_live_state
```

`eth_live_state` should not depend on those crates. This prevents the live-state protocol from turning into a runtime orchestrator.

## Lessons From Current Code

`LiveTxSimulator` already uses MDBX when it is caught up and otherwise uses tracked live state. The improvement is to move shared key/schema ownership out of `tx_simulator` so every crate uses one protocol.

Python also publishes token snapshots through `LiveDataPublisher`. The Rust contract keeps the same keys and preserves the token snapshot index so tracked-token discovery is explicit instead of being hidden inside a process-local cache.

When writing a block, publish atomically:

```text
write chain_state_snapshot
write token snapshots
set latest block keys last
emit block-ready notification
```
