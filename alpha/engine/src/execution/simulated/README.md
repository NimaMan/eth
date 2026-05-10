# Simulated Execution Adapter

EVM-backed pre-flight adapter. Not yet implemented.

## Planned Behavior

1. Accept `OrderIntent` + `AmmSwapRoute`.
2. Build swap calldata via `tx_simulator::tx_builders`.
3. Run the unsigned tx through `tx_simulator::TxSimulator` against the current
   Reth MDBX head.
4. Return `ExecutionReport` with:
   - Actual revert status
   - Actual gas used
   - Actual `filled_amount` from trace output

## Use Cases

- Pre-submission validation before real execution.
- Most honest paper fills when pool-snapshot math is insufficient (e.g. complex
  V4 routes, MEV-protected bundles).

## Why Separate from `modeled/`

`modeled/` uses fast math (<1ms). `simulated/` requires DB access and REVM
execution (10–100ms). They serve different latency budgets.
