# Real Execution Adapter

`TxExecutorAdapter` is the engine boundary for real on-chain submission. The
engine still owns order approval, order recording, and position state
transitions. Live transaction construction belongs to `alpha/live/trading`.

## Flow

```text
StrategyDecision::submit_order(reason)
  -> OrderIntent with decision_reason
  -> AlphaEngine risk policy
  -> TxExecutorAdapter
  -> LiveTradingPlannerBridge
  -> LivePrioritySellPlanner in alpha/live/trading
  -> LiveTraderTxSignal / LiveDirectRawTransactionRequest
  -> Kartal POST /eth/tx/direct-raw
  -> tx_executor validate/sign/dry-run-or-broadcast
  -> ExecutionReport
  -> TradingStore position and execution report updates
```

Real live uses the same lifecycle as backtest, but it uses different
confirmation evidence. Kartal broadcast can only move an order to
`BuySubmitted` or `SellSubmitted`. A later receipt/reconciliation worker must
emit the `BuyConfirmed`, `SellConfirmed`, `BuyFailed`, or `SellFailed`
`ExecutionReport`.

## Data Contract

`TxExecutorAdapter` receives an already-approved `OrderIntent`. It does not
build calldata and it does not choose gas. Its only live-capital job is to obtain
a `LiveTraderTxSignal`, send that signal to Kartal, and convert Kartal's response
into the normal engine `ExecutionReport` path.

The bridge/resolver split is intentional:

- `LiveTradingPlannerBridge` is generic glue from the engine trait to
  `alpha/live/trading::PrioritySellPlanner`.
- `LiveTxPlanningInputResolver` is the runtime loader. It must gather the open
  position, latest pool snapshot, signer context, observation metadata,
  deadline, and min-output evidence.
- `alpha/live/trading` validates those inputs, builds the route, checks
  allowance, runs final simulation, chooses the gas-rank candidate under the
  value cap, and returns the direct-raw Kartal request.

The bridge should return a planning failure when tx prep rejects. The engine
then records a failed `ExecutionReport`, which keeps rejected real-capital orders
visible in the same store path as simulator failures.

## Current Status

This adapter is a tested crate-private boundary used by the guarded real live
entrypoint. It is intentionally not re-exported from `eth_alpha_engine`, so
external backtest crates cannot import it through the public engine API.
`eth_alpha_live_trader` selects `TxExecutorAdapter` through
`LiveTradingPlannerBridge`; `eth_alpha_live_backtest_trader` selects
`LiveChainSimExecutionAdapter` and has no Kartal path.

The engine now has `LiveTradingPlannerBridge` and
`LiveTxPlanningInputResolver`. The bridge calls `alpha/live/trading`'
`PrioritySellPlanner` and returns a prepared `LiveTraderTxSignal`; the resolver
is the production hook that must load the matched position, latest pool snapshot,
wallet tx context, observation metadata, and deadline/min-out inputs.

The remaining production pieces are production-grade live simulation,
gas-rank/allowance providers, buy-route support, and receipt reconciliation.
Route discovery, calldata construction, slippage, gas-rank policy, and
value-capped bribe logic must stay outside the engine in `alpha/live/trading`.

## Responsibilities

- `alpha/engine`: risk-approved order routing, submission result mapping, and
  position DB updates through the existing `ExecutionReport` path.
- `alpha/live/trading`: route-specific live tx prep, pre-submit simulation
  evidence, slippage policy, gas-rank/value-cap policy, and audit metadata.
- Kartal: HTTP auth/config boundary for `tx_executor`.
- `tx_executor`: request validation, nonce reservation, signing, dry-run or
  broadcast, and executor journal records.

## Safety Rules

- The real adapter must be explicitly selected by runtime configuration.
- The planner must include simulation evidence and value-cap metadata before a
  signal crosses the Kartal boundary.
- Gas-rank selection must happen before Kartal. Kartal should receive the chosen
  EIP-1559 fee caps and audit metadata, not a request to pick a bribe.
- The resolver must prove the `OrderIntent`, open `Position`, and
  `PoolSnapshot` all refer to the same token-scoped pool before calling the live
  planner.
- `nonce` should normally be `null`; nonce reservation belongs to `tx_executor`.
- A Kartal `broadcast` response becomes a submitted order in alpha. Receipt
  tracking must later feed confirmed or failed `ExecutionReport`s back into the
  engine/store before a trade is considered settled.
- A Kartal `dry_run` response is treated as cancelled, because no transaction
  was broadcast and no on-chain fill can arrive.
- Backtest confirmation semantics must not leak into this adapter. Real live
  cannot mark a buy or sell as confirmed from planning, simulation, or Kartal
  request acceptance alone.
- Do not deploy this adapter without a receipt/reconciliation worker. Submitted
  is not confirmed.
