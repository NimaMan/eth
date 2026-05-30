# Processed Block Providers

This module is the block-level part of `processed_tx_provider`. It owns
processed-block loading and persistence boundaries for the ETH Rust pipeline.

- `load.rs` provides `ProcessedBlockProvider`: disk cache first, then direct
  Reth block processing once. This regular historical path has no retry loop.
- `range.rs` loads historical block ranges and fills missing disk-cache entries.
- `replay_store.rs` owns the canonical write path for the replay store:
  processed-block disk cache plus derived block-level indexes.
- `disk_cache/` owns the on-disk processed block cache.
- `compact.rs` defines the compact processed transaction representation used by
  provider storage so empty collections and zero-only optional values are not
  carried through persistent payloads.

Higher-level crates should request `ProcessedBlock` data through this module
instead of implementing disk or direct-processing fallback logic locally.

`ProcessedBlockDiskCacheStore` is intentionally a raw storage primitive.
Callers that persist processed blocks should use
`ProcessedBlockReplayStoreWriter` so the `.pblock.zst` file and derived indexes
such as `reth_index/address_to_blocks` stay in sync.

## Processed Block Disk Cache

The disk cache is a hot local replay store for the latest ~1M Ethereum mainnet
processed blocks. Its job is to let token tracking, live warmup, range builds,
and later analysis tools load block ranges quickly without re-running EVM replay.

Config is supplied by callers:

| Config | Meaning |
| --- | --- |
| `PROCESSED_BLOCK_DISK_CACHE_DIR` | Root directory for the cache. |
| `PROCESSED_BLOCK_DISK_CACHE_BLOCKS` | Retention target used by pruning helpers. |
| `RETH_INDEX_DIR` | Optional sidecar MDBX directory for derived `address_to_blocks` writes through `ProcessedBlockReplayStoreWriter`. |

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
`tx_processor/examples/blocks/cache/refresh_disk_cache.rs`.
It fills missing processed-block cache files through
`ProcessedBlockReplayStoreWriter`. When a block is already present in the disk
cache, the normal backfill path reads it and does not rewrite
`reth_index/address_to_blocks`. Use `--skip-address-block-index` only for a
deliberate cache-only refresh of missing blocks.

## Stored vs Consumed Fields

The cache persists a `CompactProcessedTransaction` per transaction (empty
collections and zero-only values are already elided). Not every field it can
carry is read back: the cache exists to serve **token tracking and token-related
analysis**, which consume the *extracted event* data, not raw execution traces.

A repo-wide consumer audit (eth_token, tx_fund_flow, eth_chain_server,
mempool_processor, alpha, risk_atlas) classified each non-trivial field as
**consumed** (read off a cached transaction to drive output) or **never read**.

Never read by any consumer — safe to omit from the persisted payload:

| Field | Why it is not needed |
| --- | --- |
| `struct_logs` | Per-opcode EVM trace. Produced for simulation only; the dominant payload field by size. No token consumer reads it. |
| `access_list` | EIP-2930 access list. Never read off a cached tx (the identically-named field on reth/alloy tx types is unrelated). |
| `blob_versioned_hashes` | EIP-4844 blob hashes. Never read off a cached tx. |
| `erc1155_contracts` | ERC-1155 contract set. Never read (the token path is ERC-20 / liquidity focused). |

Consumed — must stay (representative read sites):

| Field | Consumer |
| --- | --- |
| `erc20_transfers`, `eth_transfers`, `internal_transactions`, `internal_erc20_calls` | eth_token network ingest + PnL |
| `erc721_transfers` | eth_token uniswap v3/v4 position tracking |
| `erc1155_transfers` | **tx_fund_flow** fund-flow extraction (looks droppable, is not) |
| `unique_addresses`, `erc20_contracts`, `address_balance_changes` | eth_token touched-address / balance tracking |
| Uniswap v2/v3/v4 swap/mint/burn/modify/initialize/pool events | eth_token pool + candidate tracking |
| `uniswap_v4_protocol_fee_updates`, `uniswap_v4_dynamic_lp_fee_updates`, `uniswap_v4_protocol_fee_controller_updates` | **eth_token** v4 pool state (look droppable, are not) |
| authority/ownership/role/trading events, approvals, `permit2_events` | eth_token authority + replay triggers |
| `fees`, `bribe_amount`, `value`, `status`, `from/to`, `tx_type`, `actions` | eth_token + network ingest |
| `latest_states`, `other_events`, `input` | eth_token token-state / replay triggers, mempool_processor calldata decode |

The `Lean` benchmark field set (see below) drops exactly the four never-read
fields. `struct_logs` accounts for nearly all of the saving.

## Format Benchmark

`tx_processor/examples/blocks/cache/benchmark_cache_formats.rs` measures candidate
on-disk formats against the production codec **without re-running EVM replay**: it
reads existing cache files, re-encodes each block under each candidate, and
reports space, compress time, and decompress+decode time. Candidates combine a
field set (`Full` vs `Lean`) x codec (`BincodeJson` production vs `Msgpack`) with
a zstd level sweep and a trained-dictionary variant. The candidate codecs are
exposed from the store as `bench_serialize_block` / `bench_deserialize_block`.

```bash
cargo run -p tx_processor --release --example benchmark_cache_formats -- \
  --cache-dir /home/nima/storage/samsung8tb/ethereum/processed-block-cache --count 1000
```

Round-trip correctness is enforced per candidate via an order-independent
fingerprint over the consumed fields, so a candidate that would lose data a
consumer reads fails loudly instead of reporting a false saving.

### Results (mainnet, recent blocks; baseline = on-disk v2 = bincode+zstd-3)

Ratios are space vs the production baseline (1.00). They are stable across
sample size; absolute baseline was ~188-199 KB/block.

| Candidate | space vs base | write (ms/block) | read decode (ms/block) |
| --- | --- | --- | --- |
| baseline (per-block, zstd-3) | 1.00 | ~ | ~2-3 |
| per-block, zstd-9 | 0.91 | ~30-50 | ~2 |
| per-block, zstd-19 | 0.84 | ~700-1300 | ~2 |
| per-block, zstd-19 + trained dict | 0.80 | ~800-1500 | ~1 |
| **lean** per-block, zstd-9 | 0.90 | ~30 | ~1 |
| lean per-block, zstd-19 + dict | 0.80 | ~1000+ | ~1 |
| chunk-64 archive, zstd-9 | 0.82 | ~30-60 | ~1 (range) |
| chunk-256 archive, zstd-9 | 0.82 | ~60 | ~1 (range) |

Findings:

- **Field-dropping (`Lean`) saves ~1%.** The fields it drops are tiny in the
  live cache; in particular `struct_logs` are not populated on the production
  path (uncompressed payload barely changes: Full ~1.38 MB vs Lean ~1.376 MB per
  block). The big space target we expected is simply not present in the data.
- **Compression level is the main per-block lever:** zstd-3->9 ~ -10%, ->19
  ~ -16%, +trained dictionary ~ -20%. Read (decode) time is flat regardless of
  level - higher compression is essentially free on reads; only write CPU rises
  (zstd-19 is ~1 s/block, zstd-9 ~ tens of ms).
- **Chunked archives exploit cross-block redundancy:** compressing 64 consecutive
  blocks as one zstd frame reaches ~0.82 at level 9 - matching per-block zstd-19
  at a fraction of the write cost. Gains saturate by ~64 blocks/archive. Tradeoff:
  great for sequential range reads, but a random single-block read must
  decompress the whole archive, and per-block invalidation is lost.
- **MessagePack is not viable** without type changes: alloy types use a
  human-readable (string) vs binary (bytes) serde split, so a non-self-describing
  binary codec fails to round-trip (`byte array, expected a string`).
- **No ~10x is available.** The payload is already ~7:1 compressed and dominated
  by event data that consumers read and we cannot drop. The realistic ceiling
  with these levers is ~20% per-block (level+dict) or ~25-30% via chunked
  archives at high level. The cheap, zero-read-regression win is raising the
  per-block zstd level (and adopting `Lean`).

