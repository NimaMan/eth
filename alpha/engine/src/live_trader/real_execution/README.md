# Real Execution Wiring

This folder contains the live-runner wiring for capital-bearing execution.

It resolves engine `OrderIntent`s into live planning inputs, validates Kartal
preflight state, builds the real execution adapter, and keeps valuation routed
through the chain-sim adapter. Route building, exact calldata simulation,
gas-rank policy, and Kartal request construction remain in `alpha/live/trading`.

The tests in `tests.rs` cover preflight and launch guards. Receipt settlement
lives one level up in `receipt_reconciliation.rs`.
