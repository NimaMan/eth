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
  -> LiveTxPlanner / LiveTradingPlannerBridge
  -> deployed-vault buy or priority-sell planner in alpha/live/trading
  -> LiveTraderTxSignal / LiveDirectRawTransactionRequest
  -> ETH tx executor POST /eth/tx/direct-raw
  -> tx_executor validate/sign/dry-run-or-broadcast
  -> ExecutionReport
  -> TradingStore position and execution report updates
```

Real live uses the same lifecycle as backtest, but it uses different
confirmation evidence. ETH tx executor broadcast can only move an order to
`BuySubmitted` or `SellSubmitted`. A later receipt/reconciliation worker must
emit the `BuyConfirmed`, `SellConfirmed`, `BuyFailed`, or `SellFailed`
`ExecutionReport`.

The first reconciliation worker lives in the `eth_alpha_live_runner` crate at
`alpha/live/runner/src/live_trader/receipt_reconciliation/`.
It waits until the live processed-block watermark reaches at least
`submitted_block + 1`, then polls `eth_getTransactionReceipt` for submitted tx
hashes and confirms only when the receipt succeeded and the deployed V2 vault
emitted the expected fill event. Missing vault evidence is treated as unresolved,
not confirmed.

## Data Contract

`TxExecutorAdapter` receives an already-approved `OrderIntent`. It does not
build calldata and it does not choose gas. Its only live-capital job is to obtain
a `LiveTraderTxSignal`, send that signal to the ETH tx executor, and convert the
executor response into the normal engine `ExecutionReport` path.

The planner/resolver split is intentional:

- `LiveTradingPlannerBridge` is generic glue from the engine trait to
  `alpha/live/trading::PrioritySellPlanner`.
- `LiveTxPlanningInputResolver` is the runtime loader. It must gather the open
  position, latest pool snapshot, signer context, observation metadata,
  deadline, and min-output evidence.
- `EthTxExecutorRealPlanner` is the real planner. It dispatches sell
  intents to the priority-sell planner and buy intents to the deployed V2 vault
  buy builder. The buy path quotes with exact calldata and zero min-output,
  derives the production min-output from the quote, then simulates the final
  exact calldata before ETH tx executor submission.
- `alpha/live/trading` validates those inputs, builds the route, checks
  allowance, runs final simulation, chooses the gas-rank candidate under the
  value cap, and returns the direct-raw ETH tx executor request.

The bridge returns a cancelled `ExecutionReport` when tx prep or pre-submit
simulation rejects before broadcast. The exact reject reason remains on the
report, while the high-level state stays distinct from a mined failed sell.

## Current Status

This adapter is a tested boundary used by the guarded real live entrypoint. It
lives behind `eth_alpha_engine::execution::real` (`#[doc(hidden)] pub`), reachable
only by the `eth_alpha_live_runner` crate; external backtest crates do not import
it. `eth_alpha_live_trader` selects `TxExecutorAdapter` through
`LiveTradingPlannerBridge`; `eth_alpha_live_backtest_trader` selects
`LiveChainSimExecutionAdapter` and has no real-executor path.

The engine now has `LiveTradingPlannerBridge` and
`LiveTxPlanningInputResolver`. The bridge calls `alpha/live/trading`'
`PrioritySellPlanner` and returns a prepared `LiveTraderTxSignal`; the resolver
is the production hook that must load the matched position, latest pool snapshot,
wallet tx context, observation metadata, and deadline/min-out inputs.

The deployed V2 vault buy and sell paths now derive min-out from provisional
exact-calldata simulation and run a second exact-calldata simulation before
ETH tx executor submission. The live-real binary permits entries only when the resolved
strategy bankroll is at most `0.555 ETH`; buys consume that bankroll, confirmed
sells replenish it, and profits can be redeployed. The
remaining production pieces are live gas-rank providers and final
broadcast/finality operations around the receipt worker. Route discovery,
calldata construction, slippage, gas-rank policy, and value-capped bribe logic
must stay outside the engine in `alpha/live/trading`.

## Responsibilities

- `alpha/engine`: risk-approved order routing, submission result mapping, and
  position DB updates through the existing `ExecutionReport` path.
- `alpha/live/trading`: route-specific live tx prep, pre-submit simulation
  evidence, slippage policy, gas-rank/value-cap policy, and audit metadata.
- `tx_executor_service`: HTTP auth/config boundary for `tx_executor`.
- `tx_executor`: request validation, nonce reservation, signing, dry-run or
  broadcast, and executor journal records.

## Safety Rules

- The real adapter must be explicitly selected by runtime configuration.
- The planner must include simulation evidence and value-cap metadata before a
  signal crosses the ETH tx executor boundary.
- Exact pre-submit simulation must run at or after the signal's
  `required_state_block`. For launch buys this is the max of the decision block,
  observed tx block, pool creation block, and latest pool snapshot block. If the
  chain-server live simulator cannot serve that block, alpha records a deferred
  execution, not a failed buy.
- Gas-rank selection must happen before submission. The ETH tx executor should
  receive the chosen EIP-1559 fee caps and audit metadata, not a request to pick
  a bribe.
- The resolver must prove the `OrderIntent`, open `Position`, and
  `PoolSnapshot` all refer to the same token-scoped pool before calling the live
  planner.
- `nonce` should normally be `null`; nonce reservation belongs to `tx_executor`.
- An ETH tx executor `broadcast` response becomes a submitted order in alpha. Receipt
  tracking must later feed confirmed or failed `ExecutionReport`s back into the
  engine/store before a trade is considered settled.
- Planner policy rejects, pre-submit simulation rejects, and ETH tx executor
  `dry_run`/`rejected` responses are treated as cancelled, because no
  transaction was broadcast and no on-chain fill can arrive.
- Backtest confirmation semantics must not leak into this adapter. Real live
  cannot mark a buy or sell as confirmed from planning, simulation, or ETH tx
  executor request acceptance alone.
- Do not run public broadcast without receipt reconciliation enabled and
  monitored. Submitted is not confirmed.
