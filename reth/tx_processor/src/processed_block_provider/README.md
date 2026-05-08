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
