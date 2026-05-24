# Live Trader

Live trader owns the live polling process.

- `run_live_backtest()` uses live chain-state simulation and never contacts
  Kartal.
- `run_live_real()` uses the crate-private real execution boundary, requires
  Kartal dry-run, and requires each entry-enabled strategy to resolve to a
  bankroll of at most `0.555 ETH` during validation. Strategy specs provide
  buy size, liquidity floors, bankroll, entry-pool caps, and hold windows; live
  runs select those parameters by strategy name.

Real tx wiring belongs in `real_execution/`; common live polling stays in
`mod.rs`. Supporting code is split by responsibility:

- `entrypoints.rs`: public binary entrypoints.
- `support.rs`: shared config, watermarks, observation persistence, and
  heartbeat helpers.
- `restored_state.rs`: persisted position, seen-pool, hold-counter, and
  bankroll restoration.
- `strategy_setup.rs`: strategy construction from live strategy specs.
- `poll_error.rs`: token-server poll failure handling.
- `risk_annotation.rs`: mempool-signal evidence enrichment.

## Runtime Config

The normal live trader service path reads polling settings from the shared root
`config.env`:

- `ALPHA_LIVE_TRADER_POLL_INTERVAL_MS`: full trader loop sleep. Keep this below
  one second for mempool signal handling; startup rejects values above
  `1000`.
- `ALPHA_LIVE_MEMPOOL_SINCE_DAYS`: lookback window used when fetching stored
  mempool signals from the chain server.
- `ALPHA_LIVE_SIGNAL_LIMIT`: max signal rows fetched per trader loop.

The CLI flags `--poll-interval-ms`, `--mempool-since-days`, and
`--signal-limit` are explicit operator overrides only. The checked-in systemd
services do not set separate copies of these values.

## Real Receipt Reconciliation

`receipt_reconciliation.rs` owns the first real-live settlement worker. On each
live trader tick it:

1. loads submitted execution reports with tx hashes from `alpha_store`;
2. waits until the live processed-block watermark is at least
   `submitted_block + 1`;
3. calls `eth_getTransactionReceipt` on the same RPC URL Kartal reports in
   `/eth/tx/status`;
4. turns receipt `status = 0x0` into a failed `ExecutionReport`;
5. turns receipt `status = 0x1` into a confirmed `ExecutionReport` only when
   the expected V2 vault event is present:
   - `BoughtV2` for buys;
   - `EmergencySoldV2` for sells.

Successful receipts without matching vault evidence stay unresolved. The trader
logs them but does not mark the position confirmed.

Final receipt reports include `mined_evidence` in the persisted JSON payload:
receipt block number/hash, transaction index, cumulative gas, receipt status,
actual gas used, effective gas price, paid gas cost, selected gas limit,
selected max fee, selected priority fee, selected bribe metadata, and the
live-backtest comparison fields `submitted_block_number`,
`expected_confirmation_block`, and `confirmation_lag_blocks`.

The validation finality policy is explicit in the evidence: a mined receipt is
accepted after `1` confirmation for lifecycle accounting, and should be rechecked
at `3` confirmations during the first live validation trades.

## Gate 3 Validation

The live-readiness Gate 3 tests live in `src/gate3_validation.rs` and
`src/live_trader/receipt_reconciliation.rs`. Run them with:

```bash
cargo test -p eth_alpha_engine gate3 --lib
```

The suite currently locks these real-live assumptions:

- Kartal `broadcast`, `received`, `signed`, `dry_run`, `broadcast_error`, and
  `rejected` responses never become confirmed without receipt evidence.
- `dry_run`, executor `rejected`, planner policy rejects, and pre-submit
  simulation rejects map to cancelled reports because no transaction was
  broadcast.
- Submitted reports carry the selected gas limit, max fee, priority fee, and
  bribe metadata that will later be merged into mined receipt evidence.
- Exact-simulation min-output is non-zero and reverting/full-slippage cases
  reject before submission.
- Simulator state lag is an infrastructure deferral. It must not write
  `buy_failed`, must not consume bankroll, and must not synthesize a submitted
  tx. Deferred buy attempts are excluded from restored seen-pool state so the
  strategy can retry while the entry window remains valid.
- Successful receipts confirm only when the expected deployed V2 vault event is
  present, and the resulting report uses actual vault event amounts plus receipt
  gas cost.
- Receipt evidence records the mined block hash, transaction index,
  cumulative gas used, effective gas price, paid gas cost, the N+1 backtest
  comparison, and the `1`/`3` confirmation policy.
- A buy that is only submitted, pending, or dry-run-cancelled is not sellable.
