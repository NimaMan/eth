# Live State

Planned crate: `eth_live_state`

This crate owns the shared live-state protocol used through Redis. It is infrastructure shared by market data, simulators, mempool risk, and the trading engine.

## Responsibilities

- Define Redis key builders.
- Define snapshot schemas.
- Provide Redis readers/writers.
- Own retention and TTL policy helpers.
- Version serialized schemas.
- Provide test fixtures for compatibility.

## Canonical State Namespaces

Confirmed-chain state should be written by `eth_market_data` only:

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

There should be one canonical writer for confirmed state: `eth_market_data`.

Mempool risk, trading engine, and simulator may read the state, but they should not mutate canonical token/pool/block snapshots.

## Lessons From Current Code

`LiveTxSimulator` already prefers Redis chain-state overlays and falls back to MDBX. That is a good design. The improvement is to move shared key/schema ownership out of `tx_simulator` so every crate uses one protocol.

When writing a block, publish atomically:

```text
write header
write processed txs
write chain_state_snapshot
write token snapshots
set latest block keys last
emit block-ready notification
```
