# Live Backtest Adapter

This folder contains live-runner wrappers that keep no-capital live backtests
honest while reusing the same live polling loop as the real trader.

`chain_sim_gas_policy/` decorates `LiveChainSimExecutionAdapter` with the same
gas/value policy metadata used by the real runner. It must never contact Kartal
or build a broadcastable transaction.

Live backtest keeps the fundamental backtest execution model. When a strategy
observes a signal at block `N`, the simulated order is submitted at `N` and the
terminal fill/failure is produced against post-block `N+1` state.

Mempool `trading_enabled` signals are observed but not routed into the
chain-sim strategy as buy triggers. Chain-sim live backtests enter from the
normal mined pool-update path once trading-enabled state is available. Same-block
dependency ordering is a real-live public-tail gas/submission policy, not a
live-backtest assumption.
