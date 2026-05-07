# Rust Live Block Processor

This binary is the Rust replacement for the retired Python live block processor.

It watches Ethereum head updates, processes each live block through the Rust transaction processor, publishes the processed block snapshot to Redis, and emits a Redis Pub/Sub notification after each block is cached.

## Current State

- Binary name: `live_block_processor`
- Source: `tx_processor/src/bin/live_block_processor/main.rs`
- Runtime service: `deploy/systemd/blockchains/ethereum/eth-live-block-processor.service`
- Config source: `/home/nima/code/crypto/blockchains/eth/config.env` through `ETH_CONFIG_PATH`
- Reth MDBX datadir default: `/home/nima/storage/samsung8tb/ethereum/reth`
- Reth HTTP default: `http://127.0.0.1:8545`
- Reth WS default: `ws://127.0.0.1:8546`
- Redis default: `LIVE_BLOCKCHAIN_DATA_REDIS_URL` from `config.env`, falling back to `redis://localhost:6379/0`
- Redis Stream default: `eth/live/blocks`
- Pub/Sub channel default: `eth/live/block_notifications`
- Log file default: `ETH_LOG_DIR/block_processor/live_block_processor_<YYYYMMDD_HHMMSS>.log`

Runtime state is intentionally not embedded here because it changes every block. Check systemd and Redis directly:

```bash
systemctl status eth-live-block-processor.service --no-pager
redis-cli GET eth/live/latest/block_number
ls -t /home/nima/code/crypto/blockchains/eth/logs/block_processor/live_block_processor_*.log | head -1
```

## What It Publishes

For each processed block, the service writes:

- `eth/live/latest/block_number` -> latest fully published block number
- `eth/live/latest/block_hash` -> latest fully published block hash
- `eth/live/blocks` -> Redis Stream of processed block notifications
- `eth/live/recent_blocks` -> sorted set of retained block numbers
- `eth/live/block/<block_number>/meta` -> JSON block metadata
- `eth/live/block/<block_number>/header` -> JSON block header payload
- `eth/live/block/<block_number>/txs` -> Redis hash of `tx_hash -> processed transaction JSON`
- `eth/live/block/<block_number>/tx_index` -> sorted set of `tx_index -> tx_hash`
- `eth/live/block/<block_number>/addresses` -> set of addresses observed in processed transactions

After the snapshot write, it appends to `eth/live/blocks` and publishes a best-effort Pub/Sub notification to `eth/live/block_notifications` with the processed block number.

The processed transaction JSON includes normalized top-level fields for downstream replay:

- `hash`
- `from_address`
- `to_address`
- `contract_address`
- `input`
- `unique_addresses`
- `erc20_contracts`
- `erc721_contracts`
- `erc1155_contracts`

## Run

One-block smoke:

```bash
cd /home/nima/code/crypto/blockchains/eth/reth
ETH_CONFIG_PATH=/home/nima/code/crypto/blockchains/eth/config.env \
LIVE_BLOCK_LIMIT=1 \
RUST_LOG=info \
cargo run --release -p tx_processor --bin live_block_processor
```

The timestamped log file keeps only compact per-block summary lines:

```text
2026-05-05 11:05:49.521 - INFO - 25028392->435|10 in 0.18s
```

The number after `|` is the count of transactions that the Rust processor failed
to decode/process. On-chain reverts with receipt status `0x0` do not increment it.
Verbose Rust tracing stays in journald; inspect it with
`journalctl -u eth-live-block-processor.service`.

Path selection order:

- `LIVE_BLOCK_LOG=/path/to/file.log` writes to that exact file.
- `LIVE_BLOCK_LOG=/path/to/dir` or `LIVE_BLOCK_LOG_DIR=/path/to/dir` creates a timestamped file there.
- Otherwise it uses `ETH_LOG_DIR/block_processor`, matching the Python logger layout.

Continuous mode:

```bash
cd /home/nima/code/crypto/blockchains/eth/reth
ETH_CONFIG_PATH=/home/nima/code/crypto/blockchains/eth/config.env \
RUST_LOG=info \
cargo run --release -p tx_processor --bin live_block_processor
```

Build for systemd:

```bash
cd /home/nima/code/crypto/blockchains/eth/reth
cargo build --release -p tx_processor --bin live_block_processor
```

Systemd deploy unit:

```bash
sudo cp /home/nima/code/crypto/deploy/systemd/blockchains/ethereum/eth-live-block-processor.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl start eth-live-block-processor.service
```

## Address To Block Index

The old Python live processor had an optional persistent address participation writer.
The Rust-side design now stores candidate block numbers instead of tx numbers:

- Python flag: `index_address_txs`
- Writer: `eth_data.database.writers.transaction_writer.TransactionAddresstoTxIndexer`
- Backend: Pyreth `AddressBlockIndexer`
- Table: `reth_index/address_to_blocks`
- Meaning: for each processed block, write every address seen in the block's processed transaction participations to an address -> block-number reverse index once.

This is not the same as the token manager's in-memory per-block address index. The token manager still needs its local per-block index for same-block replay, for example hydrating token metadata after earlier same-sender setup transactions. That in-memory index is temporary and should not be replaced by archive history.

## Do We Still Need The Persistent Address Index?

Not in the live processor hot path.

The archive Reth node now has `IndexAccountHistory` and `IndexStorageHistory`, and `reth_chain_query` exposes:

```rust
RethQueryProvider::get_address_account_history_blocks(address, start_block, end_block)
```

That is useful for finding blocks where an address' own account state changed. It is not a full replacement for `address_to_blocks`, because processed transaction participation is broader than account state changes:

- ERC20 transfer recipients usually do not mutate their own account entry.
- Approval owners/spenders/operators can appear only in logs.
- Internal call participants and protocol/pool addresses come from traces and decoded events.
- `unique_addresses` is a processed-tx concept, not a native Reth account-history concept.

Recommendation:

- Keep the Rust live processor focused on fast block processing, Redis snapshot writes, and Pub/Sub notification.
- Use archive Reth account/storage history and logs for on-demand historical investigations.
- Use `address_to_blocks` only if we need low-latency, repeated generic address -> candidate block lookups.
- If we wire it into live processing, implement it as a separate async worker that consumes Redis block snapshots after publication and writes `reth_index/address_to_blocks`, so MDBX reverse-index writes cannot delay live block publishing.
