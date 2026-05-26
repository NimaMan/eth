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
  -> Kartal /eth/tx/direct-raw
  -> tx_executor validation/sign/dry-run-or-broadcast
  -> receipt_reconciliation/ mined receipt and vault-event settlement
```

The tests in `tests.rs` cover preflight and launch guards. Receipt settlement
lives one level up in `receipt_reconciliation/`.
