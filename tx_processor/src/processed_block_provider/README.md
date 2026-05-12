# Processed Block Provider

This module owns processed-block loading and persistence boundaries for the ETH
Rust pipeline.

- `load.rs` loads one block through disk cache first, then direct Reth block
  processing as fallback.
- `live.rs` provides `LiveProcessedBlockProvider`, which hydrates live
  processed blocks from Redis first and falls back to disk/direct processing.
- `range.rs` loads historical block ranges and fills missing disk-cache entries.
- `replay_store.rs` owns the canonical write path for the replay store:
  processed-block disk cache plus derived block-level indexes.
- `disk_cache/` owns the on-disk processed block cache.
- `compact.rs` defines the compact processed transaction representation used by
  provider storage so empty collections and zero-only optional values are not
  carried through persistent payloads.

Higher-level crates should request `ProcessedBlock` data through this module
instead of implementing Redis, disk, or direct-processing fallback logic locally.

`ProcessedBlockDiskCacheStore` is intentionally a raw storage primitive.
Callers that persist processed blocks should use
`ProcessedBlockReplayStoreWriter` so the `.pblock.zst` file and derived indexes
such as `reth_index/address_to_blocks` stay in sync.

## Processed Block Disk Cache

The disk cache is a hot local replay store for the latest ~1M Ethereum mainnet
processed blocks. Its job is to let token tracking, live warmup, range builds,
and later analysis tools load block ranges quickly without re-running EVM replay.

Primary access is by block number, so the on-disk layout is intentionally
block-number based:

```text
processed-block-cache/
  ethereum-mainnet/
    25050000.pblock.zst
```

The cache is not token-specific. Each `<block_number>.pblock.zst` file stores a
single compact binary `ProcessedBlock` payload compressed with zstd. The payload
contains the network, chain id, block number, block hash, trace engine, trace
config hash, header, compact processed transactions, and
per-transaction processing errors.

The filename is stable on purpose. Block hash and trace config hash are payload
validation fields, not lookup fields. A range read can derive every cache path
directly from `start_block..=end_block` without fetching headers first.

The cache has one current payload shape. It does not persist extra format
markers; when the shape changes, refresh the affected cache directory instead
of carrying compatibility branches.

Only this layout is current. Older `.json.zst`, `.bin.zst`, and `token-chain-*`
cache layouts should be removed from disk; runtime code does not read or
migrate them.

## Current Size And Read Time

Measured on 2026-05-12 with the current payload shape, isolated cache directory,
and Ethereum mainnet blocks `25052270..=25053269`.

```text
cache_dir=/home/nima/storage/samsung8tb/ethereum/processed-block-cache-profiles/single_current_25052270_25053269
files=1000
total_bytes=196704726
avg_bytes_per_block=196704.7
min_bytes=11456
max_bytes=608885
```

Read-only timing over those 1,000 files:

```text
read_avg_ms_per_block=3.303
read_median_ms=2.904
read_p95_ms=6.937
read_max_ms=13.708
parallel_range_read_wall_ms=434.641
parallel_range_read_wall_ms_per_block=0.435
```

The historical backfill entrypoint is
`tx_processor/examples/block/cache/refresh_processed_block_disk_cache.rs`.
It fills missing processed-block cache files through
`ProcessedBlockReplayStoreWriter`. When a block is already present in the disk
cache, the normal backfill path reads it and does not rewrite
`reth_index/address_to_blocks`. Use `--skip-address-block-index` only for a
deliberate cache-only refresh of missing blocks.
