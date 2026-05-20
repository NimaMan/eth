# Live Trader

Live trader owns the live polling process.

- `run_live_backtest()` uses live chain-state simulation and never contacts
  Kartal.
- `run_live_real()` uses the crate-private real execution boundary and currently
  requires Kartal dry-run plus `--disable-entry`.

Real tx wiring belongs in `real_execution.rs`; common live polling and
observation persistence stays in `mod.rs` and `support.rs`.
