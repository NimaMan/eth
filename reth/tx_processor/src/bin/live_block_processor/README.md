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
- Address block participation index default: enabled, writing to `<reth_datadir>/reth_index/address_to_blocks`
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

After Redis publication succeeds, the service also enqueues the processed block
for two background sinks:

- `LiveAddressBlockParticipationIndexWorker` writes `address_to_blocks`.
- `LiveProcessedBlockDiskCacheSink` writes the shared `tx_processor::processed_block_provider` disk cache.

Both sinks use bounded in-process queues and run outside the Redis publication
path. If either queue is full, the live block remains published and the worker
logs the dropped side-effect; the data can be backfilled from processed blocks.

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

## Address Block Participation Index

The old Python live processor had an optional persistent address participation writer.
The Rust-side design now stores candidate block numbers instead of tx numbers:

- Python flag: `index_address_txs`
- Writer: `eth_data.database.writers.transaction_writer.TransactionAddresstoTxIndexer`
- Backend: Pyreth `AddressBlockParticipationIndexer`
- Table: `reth_index/address_to_blocks`
- Meaning: for each processed block, write every address seen in the block's processed transaction participations to an address -> block-number reverse index once.

This is not the same as the token manager's in-memory per-block address index. The token manager still needs its local per-block index for same-block replay, for example hydrating token metadata after earlier same-sender setup transactions. That in-memory index is temporary and should not be replaced by archive history.

### Runtime Model

The index is part of the live block processor service, but it is not on the live
block publication critical path:

```text
LiveBlockProcessor
  -> process block
  -> publish Redis live snapshot and stream event
  -> enqueue block for AddressBlockParticipationIndexWorker
  -> continue with next block

AddressBlockParticipationIndexWorker
  -> read queued ProcessedBlock
  -> extract tx-level AddressParticipation records
  -> write address_to_blocks once per address per block
```

The worker writes through `AddressBlockParticipationWriter`, which deduplicates
the final `(address, block_number)` rows. Replay is idempotent.

### Configuration

No extra `config.env` variables are required for this index. The live block
processor already resolves the Reth datadir from the existing node config, and
the worker writes to `<reth_datadir>/reth_index/address_to_blocks`.

The in-process queue is bounded at 256 blocks. If we later need runtime tuning,
we should add it through the live processor config layer instead of adding more
top-level env variables.

### Logs

Successful writes emit tracing lines like:

```text
indexed live address block participation block_number=... tx_count=... participating_txs=... inserted=... write_ms=...
```

Queue saturation or write failures are warnings. The compact timestamped block
log remains focused on block processing; inspect worker logs through journald.

## Why Keep It Off The Hot Path?

The archive Reth node now has `IndexAccountHistory` and `IndexStorageHistory`, and `reth_chain_query` exposes:

```rust
RethQueryProvider::get_address_account_history_blocks(address, start_block, end_block)
```

That is useful for finding blocks where an address' own account state changed. It is not a full replacement for `address_to_blocks`, because processed transaction participation is broader than account state changes:

- ERC20 transfer recipients usually do not mutate their own account entry.
- Approval owners/spenders/operators can appear only in logs.
- Internal call participants and protocol/pool addresses come from traces and decoded events.
- `unique_addresses` is a processed-tx concept, not a native Reth account-history concept.

The persistent index is useful for low-latency repeated generic address ->
candidate-block lookups, but it should not delay block publication. Measurements
on blocks `25029968-25030967` wrote `1,165,988` address-block rows across 1,000
blocks in `1.0254s` total writer time, while block processing took `364.0362s`.
The worker model keeps that small write cost isolated from live publication and
still lets us backfill if the queue ever falls behind.
