# Real Execution Wiring

This folder contains the live-runner wiring for capital-bearing execution.

It resolves engine `OrderIntent`s into live planning inputs, validates Kartal
preflight state, builds the real execution adapter, and keeps valuation routed
through the chain-sim adapter. Route building, exact calldata simulation,
gas-rank policy, and Kartal request construction remain in `alpha/live/trading`.

The implemented capital path is:

```text
AlphaEngine OrderIntent
  -> TxExecutorAdapter
  -> LiveTradingPlannerBridge
  -> exact deployed V2 vault simulation and gas-rank policy
  -> gas-rank policy submission_route
     - public_rpc_broadcast for normal buys/sells
     - flashbots_mev_share_tail for entry.tail_after_enabling_tx
  -> Kartal /eth/tx/submit with explicit submission_policy
  -> tx_executor validation/sign/dry-run-or-broadcast
  -> receipt_reconciliation/ mined receipt and vault-event settlement
```

There is no separate tail-entry submission-route switch. The selected
`StrategyGasRankPolicy` owns both the fee-rank ladder and the transaction
submission route.

The tests in `tests.rs` cover preflight and launch guards. Receipt settlement
lives one level up in `receipt_reconciliation/`.
