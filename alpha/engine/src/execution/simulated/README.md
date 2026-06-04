# ChainSim Execution Adapter

EVM-backed no-capital execution. This is the only supported fill path for live
strategy evaluation and backtests.

## Behavior

- Builds swap calldata from each `OrderIntent`.
- Simulates the transaction against selected Reth state for historical replay
  or delegates exact-block live chain-sim settlement to chain-server.
- Records buy token amounts from simulated token balance deltas.
- Records sell proceeds from simulated denom balance deltas.
- Records gas used and simulated gas cost from execution transactions so
  backtest PnL can be net of gas.
- Cancels ETH/WETH-denominated sells when simulated proceeds are less than or
  equal to simulated gas cost.
- Persists failed execution reports when state, routing, decimals, or swap
  simulation cannot be proven from chain state.

There is no snapshot-price, fixed-slippage, or perfect-fill fallback in this
adapter.

## Adapters

- `ChainSimExecutionAdapter`: historical replay/backtest; the runner sets the
  block before each event.
- `LiveChainSimExecutionAdapter`: live no-capital trading; `execute()` records
  submission immediately with `receipt_status =
  live_backtest_chain_sim_submitted`. Final live-backtest fills are produced
  later by the `eth_alpha_live_runner` crate's
  `live_trader/execution_lifecycle/ChainSimSettlement`, which
  asks chain-server to simulate the submitted order against the exact expected
  execution block from the server-owned live-state window.

The live adapter never owns live EVM state and never selects local Reth
historical context. Historical/latest Reth simulation belongs to
`TxSimulator` or `LatestHistoricalTxSimulator`; live state belongs to
chain-server.
