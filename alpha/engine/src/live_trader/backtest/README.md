# Live Backtest Adapter

This folder contains live-runner wrappers that keep no-capital live backtests
honest while reusing the same live polling loop as the real trader.

`chain_sim_gas_policy.rs` decorates `LiveChainSimExecutionAdapter` with the
same gas/value policy metadata used by the real runner. It must never contact
Kartal or build a broadcastable transaction.
