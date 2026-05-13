# live_block_processor

Agent operating map for the Rust live processed-block publisher.

## Purpose

- Watch Ethereum head updates, process each live block through `tx_processor`,
  and publish a compact processed-block snapshot to Redis.
- Emit stream/pubsub notifications and enqueue a background replay-store write
  for the processed-block disk cache and address-block index.

## Owns

- Binary `live_block_processor`.
- Live block polling/subscription, block processing, Redis snapshot writes, and
  compact block log output.
- Background sink: `LiveProcessedBlockReplayStoreSink`.

## Does Not Own

- Core transaction decoding logic; use `tx_processor/src/tx_processor/`.
- Token registry mutation; use `eth_token` via `eth_token_server`.
- Mempool signals; use `mempool_processor`.
- Systemd deployment ownership; service files live under `node/systemd/user/`.

## Data Flow

```text
Reth head update
  -> BlockProcessor processes block with traces
  -> Redis block snapshot keys
  -> Redis stream eth/live/blocks
  -> Pub/Sub eth/live/block_notifications
  -> background ProcessedBlockReplayStoreWriter
  -> processed-block disk cache + address_to_blocks
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Binary entrypoint | `main.rs` |
| Live processor types | `tx_processor/src/live/` |
| Replay-store writer | `tx_processor/src/processed_tx_provider/block/replay_store.rs` |
| Service units | `node/systemd/user/eth-rust-live-block-processor.service` |
| Runtime config | `config.env` |

## Commands

One-block smoke:

```bash
ETH_CONFIG_PATH=/home/nima/code/crypto/blockchains/eth/config.env \
LIVE_BLOCK_LIMIT=1 \
RUST_LOG=info \
cargo run --release -p tx_processor --bin live_block_processor
```

Continuous mode:

```bash
ETH_CONFIG_PATH=/home/nima/code/crypto/blockchains/eth/config.env \
RUST_LOG=info \
cargo run --release -p tx_processor --bin live_block_processor
```

Runtime checks:

```bash
redis-cli GET eth/live/latest/block_number
redis-cli XRANGE eth/live/blocks - +
ls -t /home/nima/code/crypto/blockchains/eth/logs/block_processor/live_block_processor_*.log | head -1
```

## Published Redis Keys

- `eth/live/latest/block_number`
- `eth/live/latest/block_hash`
- `eth/live/blocks`
- `eth/live/recent_blocks`
- `eth/live/block/<block_number>/meta`
- `eth/live/block/<block_number>/header`
- `eth/live/block/<block_number>/txs`
- `eth/live/block/<block_number>/tx_index`
- `eth/live/block/<block_number>/addresses`

## Current Hazards

- Address index and disk-cache writes are intentionally off the Redis
  publication hot path. Queue saturation logs warnings but does not block live
  publication.
- Compact block logs include failed Rust processing count after `|`; reverted
  on-chain transactions with receipt status `0x0` do not increment it.
- Do not replace the token manager's temporary same-block address index with
  archive history; they serve different replay needs.
