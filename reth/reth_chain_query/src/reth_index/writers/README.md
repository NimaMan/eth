# AddressTxWriter Performance Notes (Nov 2025)

This folder hosts the Rust writers that back the PyReth Python bindings. The
`AddressTxWriter` in `address_tx_writer.rs` is responsible for persisting the
address → transaction participation index via MDBX (see
`reth_index/database.rs`). Recent profiling from the live block processor shows
that the time spent inside this writer dominates the end-to-end latency when we
operate at Ethereum head. This document captures the findings so future work on
the writer can focus on the actual bottlenecks.

**Nov 2025 update:** the Python `TransactionAddresstoTxIndexer` now buffers each
block until it is at least `PYRETH_ADDRESS_TX_WRITE_LAG_SECONDS` old (defaults
to 120 s) before calling back into PyReth. Combined with
`AddressTxWriter::flush_pending_blocks` flushing pending entries in ascending
block order, backlog flushes remain append-only and no longer trigger a
full-shard rewrite when the node briefly lags transaction indices. The Rust
writer batches all ready blocks into a single MDBX transaction (capped via
`PYRETH_ADDRESS_TX_MAX_FLUSH_BLOCKS`, default 20) and the database layer keeps
an append-only fast path that touches only the tail shard whenever new
transaction numbers are strictly increasing. To shrink fsync time the MDBX
environment now defaults to `PYRETH_INDEX_DB_SYNC_MODE=safe-no-sync`; set the
variable to `durable` if you prefer the original fully-synchronous behavior
(writes will return to the ~10 s/block range as we trade durability).

### Storage layout rewrite (Nov 2025)

Older versions stored transaction numbers per address in 2 000-entry shards.
Every append had to read, decode, and rewrite the entire shard, which meant
touching ~16 KiB per address even when we only needed to add a single tx number.
The new layout is intentionally simple: the MDBX table is `DUP_SORT |
DUP_FIXED`, the key is just the 20-byte address, and every tx participation is
stored as an 8-byte duplicate value. Appends therefore become pure
`cursor.put(address, tx_number)` calls with no read/modify/write overhead, and
queries iterate dup values directly.

**Migration:** MDBX cannot convert the old table in place. If you see
`AddressTx index table exists with a legacy layout` on startup, delete the old
`reth_index/address_to_txs` files (or the whole `reth_index` directory) and rerun
the indexer so it can rebuild the table with dup-sort flags.

## Symptoms Observed From Production Logs

- Live pipeline logs at
  `/home/nima/code/crypto/eth/logs/block_processor_pipeline/` record per-block
  metrics. For the 2025‑11‑09 15:32–15:35 CET window we saw:

  | Block      | Addresses appended | `pyreth_time` | Notes                                    |
  |------------|-------------------:|---------------|------------------------------------------|
  | 23762320   | 0                  | 0.00 s        | Reth indices missing → block deferred    |
  | 23762321   | 0                  | 0.00 s        | Deferred (same reason)                   |
  | 23762322   | 1 148              | 34.82 s       | First block whose `BlockBodyIndices` were available; writer flushed pending cache + current block, so the entire 34 s was inside `PyAddressTxIndexer.write_transactions` |
  | 23762323   |   710              | 8.71 s        | Regular write with no backlog still costs ~9 s |
  | 23762324   |   920              | 8.43 s        | Regular write                            |
  | 23762328   |   294              | 51.94 s       | Three blocks (23 320–23 327) deferred, so this block flushed them all |

- Across the full log, every entry with `appended > 0` showed
  `pyreth_time ≈ index_time`, while entries with `appended = 0` completed in
  < 5 ms. The median time per appended address is roughly **25 ms** on the
  current NVMe, and spikes to **170 ms / address** when a backlog is flushed.

## Where The Time Is Spent

1. **Obtaining block indices**
   - `AddressTxWriter::process_block` (lines 106‑132) calls
     `RethQueryProvider::get_block_tx_indices`.
   - When the node is at head and the transaction lookup stage has not sealed
     the latest block yet, `get_block_tx_indices` returns an error. The writer
     then caches the whole block in `pending_blocks` and bails out (“Indexed
     block … appended=0” log entry).
   - Once a later block *does* have indices, `ingest_block_participation`
     flushes all pending blocks before processing the new block, so the first
     “available” block pays the cost for the entire backlog. This explains the
     34–56 s spikes.

2. **Single MDBX writer transaction per block**
   - `RethIndexDB::append_address_transactions_batch` (database.rs:120‑222)
     opens a RW transaction, iterates over every address touched in the block,
     and commits synchronously.
   - For each address we:
     1. Use `cursor.set_range` to load every shard for the address.
     2. Reconstruct the current list of transaction numbers (so we can dedup
        future writes).
     3. Append the new numbers (or, if they somehow fall before the current max,
        rewrite all shards).
     4. Call `cursor.put` / `append_additional_shards` to write the updated
        shards.
   - Even in the “append only” case (which is the norm because tx numbers are
     strictly increasing), we still perform a read + encode + write per address
     and fsync the entire transaction on commit. With ~600‑1 400 addresses per
     block, this yields 8–10 s of wall clock time despite running on NVMe.

3. **Deferred blocks magnify the problem**
   - Every time `get_block_tx_indices` lags tip, the writer stores another block
     in `pending_blocks` (address_tx_writer.rs:78‑105). When the lookup data
     appears, `flush_pending_blocks()` replays *all* pending blocks in the same
     call, so the RW transaction includes multiple blocks worth of writes. That
     pushes the transaction time into the 30–50 s range.

## Implications for Tuning/Prioritization

- **Publishing delays are caused by LMDB writes, not RabbitMQ or Python.** The
  Python side now runs the writer in a background thread and logs `pyreth_time`
  separately. Whenever `pyreth_time` is zero the publish path completes in
  < 0.2 s.

  ⇒ Any attempt to optimize Python/RabbitMQ without addressing the writer will
  not move the needle.

- **The writer is constrained by MDBX’s single-writer model and synchronous
  fsyncs.** Each block opens its own RW transaction (`env.begin_rw_txn()`),
  writes hundreds of keys, and commits. There is no batching across blocks or
  async flush, so throughput at chain tip is bound by disk latency.

- **Backlog flushes are unavoidable while we read from a node that trails its
  own WebSocket heads.** As soon as the node emits a `newHeads` notification
  before the TransactionLookup stage has finished, we queue work in
  `pending_blocks`. Without a second writer, the next block with indices will
  always suffer a multi-second pause.

## Next Investigation / Optimization Ideas

1. **Decouple the writer from the live processor.** Even with perfect tuning the
   LMDB writes will occasionally take seconds. Moving `write_transactions` into
   a dedicated worker/service ensures those stalls never block head ingestion.

2. **Batch multiple blocks per MDBX transaction.** Instead of opening one RW
   transaction per block, accumulate N blocks (or a time slice) and append all
   addresses before committing. That amortizes the fsync cost.

3. **Investigate MDBX geometry & sync settings.** The current environment uses
   the default geometry (`Geometry::default()`), so MDBX may be extending the
   map one page at a time. Tuning `set_geometry` (larger growth step, disabling
   autoshrink) and enabling `ENABLE_WRITEMAP` could reduce copy-on-write
   overhead. If durability requirements allow, enabling `MDBX_SAFE_NOSYNC` or a
   relaxed sync policy would further reduce fsync stalls.

4. **Pre-warm block indices before writing.** If we can detect that
   `get_block_tx_indices` is lagging, a separate task could poll until the data
   is available, so the hot writer path never sees the “indices missing” error.
   This avoids populating `pending_blocks` and the subsequent mega-flush.

5. **Measure per-address shard rewrites.** Right now we always read/encode every
   shard for every address. Adding a fast path that only appends to the last
   shard when the new tx_numbers are contiguous (the common case) would shave
   off a large fraction of CPU and I/O.

Keeping these findings under version control next to the writer’s source should
help anyone investigating future latency spikes understand that the critical
path lives in `AddressTxWriter`/`RethIndexDB`, not in the Python publish logic.
