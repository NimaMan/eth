# Live Backtest Adapter

This folder contains live-runner wrappers that keep no-capital live backtests
honest while reusing the same live polling loop as the real trader.

`chain_sim_gas_policy/` decorates `LiveChainSimExecutionAdapter` with the same
gas/value policy metadata used by the real runner. It must never contact Kartal
or build a broadcastable transaction.

Live backtest keeps the fundamental backtest execution model. When a strategy
observes a signal at block `N`, the simulated order is submitted at `N` and the
terminal fill/failure is produced against post-block `N+1` state. For mempool
`trading_enabled` tail-entry signals, this means the strategy can emit
`entry.tail_after_enabling_tx`, but the adapter records
`tail_entry_validation_mode=post_mine_n_plus_1` with
`exact_overlay_simulation=false`. Same-block dependency ordering is a real-live
deployment gate, not a live-backtest assumption.
