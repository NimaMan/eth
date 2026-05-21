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
2. calls `eth_getTransactionReceipt` on the same RPC URL Kartal reports in
   `/eth/tx/status`;
3. turns receipt `status = 0x0` into a failed `ExecutionReport`;
4. turns receipt `status = 0x1` into a confirmed `ExecutionReport` only when
   the expected V2 vault event is present:
   - `BoughtV2` for buys;
   - `EmergencySoldV2` for sells.

Successful receipts without matching vault evidence stay unresolved. The trader
logs them but does not mark the position confirmed.
