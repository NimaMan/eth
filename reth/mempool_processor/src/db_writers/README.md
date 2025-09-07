Mempool Arrival Time Writing

Overview
- Goal: Persist first-seen mempool arrival times only for transactions that get mined.
- Storage: Compact MDBX table in `reth_chain_query` (TxNumber → first_seen_ms).
- Writer flow: Resolve `tx_hash → TxNumber` via local Reth DB, then batch-write `(tx_number, first_seen_ms)` in a single transaction.

Why milliseconds?
- Milliseconds (`first_seen_ms: u64`) are sufficient for analytics and smaller than nanoseconds.
- We store epoch milliseconds (UTC). This is a local observation of when the mempool processor first saw the tx.

Data Model
- Table name: `mempool_tx_arrival_times`
- Key: `TxNumber` (u64, big‑endian) — the Reth sequential transaction id
- Value: `first_seen_ms` (u64, big‑endian)

Where it lives
- DB/table: `rust/reth_chain_query/src/reth_index/{database.rs,tables/mempool_tx_arrivals.rs}`
- High-level writer: `rust/reth_chain_query/src/reth_index/writers/mempool_arrival_writer.rs`
  - Resolves tx hashes to TxNumbers using a shared `ProviderFactory`
  - Performs batch writes in one MDBX RW transaction

End-to-end Flow
1) mempool_signal_detector receives txs from IPC.
2) mempool_timestamp_tracker records `first_seen_ms` for `tx_hash` in memory (keep the earliest per hash).
3) Periodic flush (or delay window):
   - Take up to `batch_size` pending entries: `[(hash, first_seen_ms)]`
   - Call `MempoolArrivalWriter.write_arrivals_by_hashes_ms(entries)`
   - Writer uses shared `ProviderFactory` to resolve `hash → TxNumber` via `provider.transaction_id(hash)`
   - Batch write resolved `(tx_number, first_seen_ms)` with `db.put_tx_arrivals_ms(...)`
   - Remove only resolved hashes from the pending map; re-try unresolved later

Provider Sharing (no DB reopens)
- The writer accepts an `Arc<ProviderFactory<...>>`, which we already have via `TxSimulator`.
- Pass the same shared factory used by simulators to avoid opening multiple MDBX environments.

Operational Notes
- We do NOT persist pre-inclusion hashes; only mined TX get written to MDBX.
- If the process restarts before inclusion, in-flight arrivals may be lost (by design, minimal footprint).
- Arrival times are local wall-clock observations; if necessary, we can add monotonic sanity checks later.

Integration Checklist
- Tracker config: add `reth_datadir` and `arrival_index_dir`.
- Construct once at startup:
  - `Arc<RethIndexDB>::open(arrival_index_dir)`
  - `Arc<TxSimulator>` → `Arc<ProviderFactory>`
  - `MempoolArrivalWriter::new(db, provider_factory)`
- In the main loop: call `tracker.record_transaction(hash)` per tx.
- On flush: resolve + write via `writer.write_arrivals_by_hashes_ms(...)`.

