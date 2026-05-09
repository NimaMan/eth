# Processed Block Provider

This module owns processed-block loading and persistence boundaries for the ETH
Rust pipeline.

- `load.rs` loads one block through disk cache first, then direct Reth block
  processing as fallback.
- `live.rs` provides `LiveProcessedBlockProvider`, which hydrates live
  processed blocks from Redis first and falls back to disk/direct processing.
- `range.rs` loads historical block ranges and fills missing disk-cache entries.
- `disk_cache/` owns the on-disk processed block cache.
- `compact.rs` defines the compact processed transaction representation used by
  provider storage so empty collections and zero-only optional values are not
  carried through persistent payloads.

Higher-level crates should request `ProcessedBlock` data through this module
instead of implementing Redis, disk, or direct-processing fallback logic locally.

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
config hash, cache schema id, header, compact processed transactions, and
per-transaction processing errors.

The filename is stable on purpose. Block hash and trace config hash are payload
validation fields, not lookup fields. A range read can derive every cache path
directly from `start_block..=end_block` without fetching headers first.

Only this layout is current. Older `.json.zst`, `.bin.zst`, and `token-chain-*`
cache layouts should be removed from disk; runtime code does not read or
migrate them.

The historical backfill entrypoint is
`tx_processor/examples/block/cache/refresh_processed_block_disk_cache.rs`.
It fills missing processed-block cache files and, by default, updates derived
block-level indexes such as `reth_index/address_to_blocks` from the same
`ProcessedBlock` values. Use `--skip-address-block-index` only for a deliberate
cache-only refresh.
