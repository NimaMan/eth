# DB Writers

This folder owns persistence adapters used by `mempool_processor`. There are two
separate persistence concerns: public semantic signals in Postgres, and mined-tx
arrival timing in RethIndex.

## Public Signal Writing

Overview
- Goal: Persist public mempool trading/risk signals for token-server, ASENA, and
  alpha consumers.
- Storage: PostgreSQL schema `live_trading`, configured by
  `databases.mempool.url` in `blockchains/eth/config.toml`.
- Canonical writer: `UnifiedSignalWriter` in `unified_signal_writer.rs`.

Tables
- `live_trading.signal_events`: common public fields, payload JSON, dedupe key,
  token, pool identifier, protocol, actor/subject, timestamps, detector-time
  chain head (`detected_at_head_block_number` and optional hash), and severity.
- `live_trading.trading_enabled_details`: buy/sell tax at signal time.
- `live_trading.sell_blocked_details`: can-buy/can-sell, tax values, failure
  reason, and confidence.
- `live_trading.tax_change_details`: tax buckets, threshold flags, and
  cannot-sell flag.
- `live_trading.liquidity_removal_details`: remover, removed/remaining denom,
  removal percentage, and drain-risk level.
- `live_trading.lp_position_approval_details`: LP/ERC-721 position approval
  model, owner/spender, approval amount/share, liquidity share, and full-approval
  flag.
- `live_trading.token_supply_risk_details`: supply-risk type, actor/block, and
  confidence.

`signal_events` is the source of truth. Logs and ZMQ notifications are
diagnostic mirrors.

The detector-time chain head fields are part of the public wire contract. Alpha
uses them as pending-signal observed-block evidence; the current live block
frame is not a valid substitute because the signal may have been detected
before the frame currently being consumed.

## Mempool Arrival Time Writing

Overview
- Goal: Persist first-seen mempool arrival times only for transactions that get mined.
- Storage: Compact MDBX table in `reth_chain_query` (txumber → first_seen_ms).
- Writer flow: Resolve `tx_hash → txumber` via local Reth DB, then batch-write `(tx_number, first_seen_ms)` in a single transaction.

Why milliseconds?
- Milliseconds (`first_seen_ms: u64`) are sufficient for analytics and smaller than nanoseconds.
- We store epoch milliseconds (UTC). This is a local observation of when the mempool processor first saw the tx.

Data Model
- Table name: `mempool_tx_arrival_times`
- Key: `txumber` (u64, big‑endian) — the Reth sequential transaction id
- Value: `first_seen_ms` (u64, big‑endian)

Where it lives
- DB/table: `reth_chain_query/src/reth_index/{database.rs,tables/mempool_tx_arrivals.rs}`
- High-level writer: `reth_chain_query/src/reth_index/writers/mempool_arrival_writer.rs`
  - Resolves tx hashes to txumbers using a shared `ProviderFactory`
  - Performs batch writes in one MDBX RW transaction

Current live flow
1. `mempool_signal_detector` receives txs from IPC.
2. `MempoolArrivalRecorder` in `src/arrival_recorder.rs` records the earliest
   `first_seen_ms` per tx hash in memory.
3. The recorder flush task wakes on the configured interval, takes up to
   `batch_size` pending hashes, and calls
   `MempoolArrivalWriter.write_arrivals_by_hashes_ms_return_resolved(...)`
   from a blocking worker.
4. `MempoolArrivalWriter` uses the shared `ProviderFactory` to resolve
   `hash -> txumber` through `provider.transaction_id(hash)`.
5. Resolved `(tx_number, first_seen_ms)` rows are written with
   `db.put_tx_arrivals_ms(...)`.
6. Only resolved hashes are removed from memory. Unresolved hashes stay pending
   until a later flush or `max_entry_age`.

Provider Sharing (no DB reopens)
- The writer accepts an `Arc<ProviderFactory<...>>`, which we already have via `TxSimulator`.
- Pass the same shared factory used by simulators to avoid opening multiple MDBX environments.

Operational Notes
- We do NOT persist pre-inclusion hashes; only mined TX get written to MDBX.
- If the process restarts before inclusion, in-flight arrivals may be lost (by design, minimal footprint).
- Arrival times are local wall-clock observations; if necessary, we can add monotonic sanity checks later.

Current live wiring
- Binary: `src/bin/mempool_signal_detector.rs`.
- Recorder: `src/arrival_recorder.rs`.
- Index directory: `<MEMPOOL_RETH_DATADIR>/reth_index`.
- Writer: `MempoolArrivalWriter::new(db, provider_factory)`.
- Provider factory: cloned from the active `TxSimulator`/`MempoolSimulator`.
- Config: 5s flush interval, 1000 hash batch size, 2 day max pending age.
- If RethIndex init fails, arrival timing is disabled and live signal processing
  continues.

## Legacy Timestamp Tracker

`mempool_timestamp_tracker.rs` is an older optional Postgres updater that batches
`eth_db.transactions.mempool_first_seen` updates after matching mined
transactions by hash. It is not wired into the current
`mempool_signal_detector` arrival path, is not the public signal store, and
should not be used for live semantic events.
