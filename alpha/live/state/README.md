# Live State

Crate: `eth_live_state`

This crate owns the shared live-state contracts used by the Ethereum alpha
runtime. The current live block path is in-process: `eth_chain_server` applies
confirmed blocks and hands direct live updates to token tracking. There is no
external live-block or live-state transport in that path.

## Responsibilities

- Define snapshot schemas.
- Define reader/writer traits for live-state stores.
- Own retention policy helpers.
- Version serialized schemas.
- Provide in-memory fixtures for tests and backtests.

This crate intentionally does not own the block processor, processed-block
pipeline, tracked-token set, or live-token object cache. Those are
writer/consumer responsibilities in higher-level crates.

`eth_live_feed` writes token snapshots and block-ready markers through these
contracts when it owns the write path. The processed block itself remains
`tx_processor::ProcessedBlock`; this crate must not define a second
processed-block shape.

## Canonical State

Confirmed-chain state should have one canonical writer. For the current runtime,
that writer is the live token runtime hosted by `eth_chain_server`.

Trading state, speculative mempool risk, and durable analytics must live in
their own stores. They may read live snapshots, but they must not overwrite
canonical token/pool state.

## Single Writer Rule

There should be one canonical writer for confirmed state.

Mempool risk, trading engine, and simulator may read the state, but they should not mutate canonical token/pool snapshots or block-ready markers.

## Rust Boundary

The clean dependency direction is:

```text
eth_live_feed     -> eth_live_state
tx_simulator      -> eth_live_state
mempool_processor -> eth_live_state
eth_alpha_engine  -> eth_live_state
```

`eth_live_state` should not depend on those crates. This prevents the live-state
contracts from turning into a runtime orchestrator.

## Lessons From Current Code

`LiveTxSimulator` is live-only and expects an in-memory mined block session from
the live block processor. Latest local historical context stays under
`TxSimulator` / `LatestHistoricalTxSimulator`. The shared snapshot schemas and
store traits stay outside `tx_simulator` so every crate depends on one contract.

When writing a block, publish atomically:

```text
write chain_state_snapshot
write token snapshots
set latest block keys last
emit block-ready notification
```
