# Real Execution Wiring

This folder contains the live-runner wiring for capital-bearing execution.

It resolves engine `OrderIntent`s into live planning inputs, validates ETH tx
executor preflight state, builds the real execution adapter, and keeps valuation
routed through the chain-sim adapter. Route building, exact calldata simulation,
gas-rank policy, and ETH tx executor request construction remain in
`alpha/live/trading`.

The implemented capital path is:

```text
AlphaEngine OrderIntent
  -> TxExecutorAdapter
  -> LiveTradingPlannerBridge
  -> exact deployed V2 vault simulation and gas-rank policy
  -> gas-rank policy submission_route
     - public_rpc_broadcast for normal buys/sells
     - public_mempool_tail for mempool trading-enabled tail-entry buys
  -> ETH tx executor /eth/tx/submit with explicit submission_policy
  -> tx_executor validation/sign/dry-run-or-broadcast
  -> receipt_reconciliation/ mined receipt and vault-event settlement
```

There is no separate tail-entry submission-route switch. The selected
`StrategyGasRankPolicy` owns both the fee-rank ladder and the transaction
submission route.

Public mempool `entry.tail_after_enabling_tx` evidence is not valid input for
private relay hash-only ordering. The live path now broadcasts publicly with a
priority fee set to the observed dependency priority fee minus
`ALPHA_LIVE_TAIL_ENTRY_PRIORITY_UNDERCUT_WEI`; max fee is predicted base fee
plus `ALPHA_LIVE_TAIL_ENTRY_MAX_FEE_BUFFER_BPS`, plus that selected priority.
This is a heuristic ordering attempt, not a consensus guarantee.

The tests in `tests.rs` cover preflight and launch guards. Receipt settlement
lives one level up in `receipt_reconciliation/`.
