# Real Execution Adapter

On-chain submission adapter. Not yet implemented.

## Planned Behavior

1. Accept `OrderIntent`.
2. Resolve pool protocol → `AmmSwapRoute`.
3. Build swap calldata via `tx_simulator::tx_builders`.
4. Optionally pre-simulate via `tx_simulator` (revert guard).
5. Query `eth_block_tx_rank` for rough mined-block position/gas-before evidence.
6. Persist the rank evidence with the order decision before submission.
7. Submit the final `DirectRawTransactionRequest` to `tx_executor::EthTxExecutor`.
8. Poll for receipt and map real `tx_hash` back to `TokenPoolId`.
9. Return `ExecutionReport` with on-chain fill results.

## Safety Rules

- Must be explicitly enabled via a config flag.
- Must enforce a capital limit and circuit breaker.
- Must only be used after chain-sim PnL is positive for 7+ days.

## Why Separate from `tx_executor`

`tx_executor` is a generic transaction broadcaster. It knows nothing about swaps,
slippage, AMM routes, or rank decisions. This adapter is the **only** component
in `alpha/engine` that talks to `tx_executor`; block-rank checks happen before
that boundary through `eth_block_tx_rank`.
