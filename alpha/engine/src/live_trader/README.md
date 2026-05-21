# Live Trader

Live trader owns the live polling process.

- `run_live_backtest()` uses live chain-state simulation and never contacts
  Kartal.
- `run_live_real()` uses the crate-private real execution boundary, requires
  Kartal dry-run, and requires each entry-enabled strategy to resolve to a
  bankroll of at most `0.225 ETH` during validation. Strategy specs can provide
  defaults; `--entry-bankroll-eth` overrides them.

Real tx wiring belongs in `real_execution.rs`; common live polling and
observation persistence stays in `mod.rs` and `support.rs`.

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

## Gate 3 Validation

The live-readiness Gate 3 tests live in `src/gate3_validation.rs` and
`src/live_trader/receipt_reconciliation.rs`. Run them with:

```bash
cargo test -p eth_alpha_engine gate3 --lib
```

The suite currently locks these real-live assumptions:

- Kartal `broadcast`, `received`, `signed`, `dry_run`, `broadcast_error`, and
  `rejected` responses never become confirmed without receipt evidence.
- `dry_run` is evidence only and maps to a cancelled report because no
  transaction was broadcast.
- Exact-simulation min-output is non-zero and reverting/full-slippage cases
  reject before submission.
- Successful receipts confirm only when the expected deployed V2 vault event is
  present, and the resulting report uses actual vault event amounts plus receipt
  gas cost.
- A buy that is only submitted, pending, or dry-run-cancelled is not sellable.
