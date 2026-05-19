# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only active bottlenecks here:
what is limiting the pipeline, the evidence, the owning subsystem, and the next
action. Completed work belongs in focused docs or commit history.


## Bogaz Destination Goal

Deploy a real ETH strategy whose policy has been validated by reproducible
recent backtests, live shadow/live-backtest evidence, and audited trade-level
decisions.

This is the Bogaz roadmap target, not the current Codex execution target.

The non-capital path is not the deployment target. It is the calibration layer:
use it to estimate policy behavior against current live data, compare it with
historical backtests, and catch signal/simulation drift before real execution.

Finished means:

- A production strategy policy is selected from the current strategy set.
- The selected policy is backtested over the exact last two-week block window.
- The same policy has live shadow/live-backtest evidence against the current
  chain-server and mempool pipeline.
- Top 10 and worst 10 positions are reviewed with entry reason, skip reason,
  exit reason, gas/slippage assumptions, and PnL snapshot.
- The decision ledger can explain every buy, skip, retry, exit, and failed
  simulation needed for frontend review.
- Execution gates are explicit before real orders are sent: max exposure,
  sizing, retry cadence, exit restrictions, stale-data thresholds, simulation
  freshness, and kill switch behavior.


## Current Limiting Factor

As of 2026-05-19, the live-capital bottleneck is production runtime wiring, not
the direct-raw wire protocol. Kartal and `tx_executor` can already accept,
validate, sign, journal, dry-run, and publicly broadcast a prepared direct
transaction. `alpha/live/trading` now has a v1 priority-sell planner scaffold and
the engine has a `LiveTradingPlannerBridge`, but the running trader still does
not instantiate that path with production providers.

The practical consequence is:

- A manual direct-raw transaction can be sent to Kartal when the signer/token/RPC
  config is present.
- `eth_alpha_trader` cannot be promoted to real capital by flipping an env var;
  its CLI still only accepts `chain-sim`.
- The `TxExecutorAdapter` and `LiveTradingPlannerBridge` exist in
  `alpha/engine`, but no runtime mode instantiates them.
- `alpha/live/trading::LivePrioritySellPlanner` can build either a direct
  Uniswap V2 ETH/WETH sell route or a Baygus V2 vault emergency-sell route,
  consume simulation and gas-rank providers, value-cap the bribe, and emit
  `LiveTraderTxSignal`.
- The production `LiveTxPlanningInputResolver`, final simulation provider,
  gas-rank provider, route policy, and receipt reconciler are still missing from
  the live critical path.

## Live Tx Submission Audit 2026-05-19

Current intended path:

```text
eth_chain_server live pools + mempool signal rows
  -> eth_alpha_trader
  -> LiveSnipeAllStrategy / strategy suite
  -> StrategyDecision::SubmitOrder
  -> OrderIntent
  -> AlphaEngine risk policy + order recording
  -> real TxExecutorAdapter
  -> LiveTradingPlannerBridge
  -> alpha/live/trading LivePrioritySellPlanner + tx_prep
  -> LiveTraderTxSignal / LiveDirectRawTransactionRequest
  -> Kartal POST /eth/tx/direct-raw
  -> tx_executor validate -> reserve nonce -> sign -> dry-run/public broadcast
  -> ExecutionReport back into alpha store
  -> receipt tracker later confirms/rejects the on-chain fill
```

Current actual running path:

```text
eth_alpha_trader
  -> LiveSnipeAllStrategy / strategy suite
  -> StrategyDecision::SubmitOrder
  -> OrderIntent
  -> AlphaEngine risk policy + order recording
  -> LiveChainSimExecutionAdapter
  -> tx_simulator chain-state buy/sell simulation
  -> ExecutionReport with no real tx hash
  -> alpha store
```

The codebase now has the middle and lower pieces, but they are not connected in
the live binary:

| Layer | Current state | Deployment gap |
| --- | --- | --- |
| Strategy decision | `LiveSnipeAllStrategy` emits sell `OrderIntent`s for liquidity removal, LP approval, max-hold, retries, tax/scam when enabled. | Strategy does not encode execution urgency, route, gas-rank evidence, or private/public submission preference beyond reason/config fields. |
| Engine | `AlphaEngine` records decisions and can route any approved `OrderIntent` to an `EngineExecutionAdapter`. | `eth_alpha_trader` constructs `LiveChainSimExecutionAdapter`; real mode is rejected by CLI parsing. |
| Real adapter | `TxExecutorAdapter` maps a prepared live tx signal to Kartal and returns an `ExecutionReport`; `LiveTradingPlannerBridge` adapts the engine to `alpha/live/trading::PrioritySellPlanner`. | No runtime instantiates it, no production context resolver is wired, and receipt reconciliation is not implemented. |
| Live tx prep | `alpha/live/trading::tx_prep` enforces route validation, pre-submit simulation, value-capped bribe policy, gas-rank candidate selection, and protocol metadata. `LivePrioritySellPlanner` now orchestrates those pieces for v1 priority sells. | Its production resolver/providers are missing: current-position lookup, latest pool snapshot, exact min-out/deadline, final simulation, gas-rank candidates, and route policy. |
| Route/calldata | `UniswapV2SellRouteBuilder` wraps direct V2 sells and `BaygusV2VaultSellRouteBuilder` wraps Mode A vault emergency sells into `PreparedSellRoute`. Rust builders exist in `tx_simulator::tx_builders`. | Production min-out evidence, slippage policy, vault deployment config, and route selection still need to be wired. |
| Gas rank | `alpha/block_tx_rank` estimates mined-block rank and gas-before evidence. | No adapter pulls recent samples, produces `RankedFeeCandidate`s, and passes them into `tx_prep` on the live critical path. |
| Kartal | `/eth/tx/direct-raw` is mounted on the order server and deserializes into `tx_executor::DirectRawTransactionRequest`. | Service config must be live-checked for token, signer, RPC reachability from the container, and broadcast mode before deployment. |
| tx_executor | Direct raw validation, caps, nonce reservation, local signing, journal, dry-run, and public mempool broadcast exist. | No private relay/builder route, no receipt watcher, and no execution fill reconciliation back to alpha. |

## Bribe Selection Discussion

For `eth_direct_raw_v1`, the bribe is the EIP-1559 priority fee selected before
Kartal submission. We should not ask Kartal or `tx_executor` to discover the
bribe. Alpha must choose it from current strategy context, recent block-rank
evidence, and protected trade value.

The live planner should use `eth_block_tx_rank` to generate a small set of
ranked candidates from recent mined blocks:

```text
candidate label
priority_fee_gwei
max_fee_per_gas_gwei
expected rank position
expected gas before us
sample/source window
```

Then `alpha/live/trading::tx_prep` applies the economic cap:

```text
avoidable_loss = simulated_recovery_if_fast
               - expected_late_recovery_if_slow
               - safety_buffer

max_priority_spend = avoidable_loss - estimated_base_fee_cost
max_priority_fee_gwei = max_priority_spend / estimated_gas_used
```

Only candidates inside both the priority-spend cap and total-fee cap are
eligible. Public mempool routing should reject if no ranked candidate fits. That
is safer than broadcasting a weak transaction that advertises our exit without a
credible chance of landing early. The planner must not synthesize an unranked
value-cap candidate when rank evidence is missing or too expensive.

Open tuning questions before live capital:

- Which recent-block window should rank against: last block only, rolling N
  blocks, or signal-specific windows around prior scam transactions?
- Which rank band is required by signal type: mempool LP approval likely needs a
  top-of-block band, while mined LP approval may accept a lower band unless
  removal probability is high.
- How stale can gas-rank evidence be during a fast exit path before the planner
  rejects and avoids sending a bad request?
- Should candidate labels and rejected alternatives be persisted with the order
  even when `tx_prep` rejects, so later analysis can tune the cap?

## Immediate Next Actions

1. Add an explicit real execution mode to `eth_alpha_trader`, guarded by config:
   Kartal URL/token, signer address, max exposure, broadcast mode, kill switch,
   and dry-run/live-capital mode must be visible in the run record.
2. Implement the production `LiveTxPlanningInputResolver`: load the matched open
   position, current pool snapshot, configured signer, current block/deadline,
   min-out evidence, and strategy observation metadata for each sell intent.
3. Replace fixed planner providers with live providers: route policy,
   allowance/vault-spend policy, final simulation, and gas-rank/base-fee
   provider.
4. Run a final pre-submit simulation on the exact planned calldata and turn the
   result into `PreSubmitSimulation`; reject if it reverts or expected recovery is
   dust.
5. Wire `eth_block_tx_rank` into the planner so it produces
   `RankedFeeCandidate`s and lets `tx_prep` reject bribes that exceed protected
   value.
6. Add receipt tracking for Kartal tx hashes. A Kartal `broadcast` response is
   only submitted, not settled; alpha must later record confirmed, failed, or
   replaced execution reports.
7. Decide public mempool versus private relay. The current executor only supports
   public mempool. That is acceptable for a first dry-run/public smoke test, but
   racing scam liquidity removals may need private/builder submission.
8. Run shadow mode through the exact real planner path while Kartal remains
   `dry_run`, then compare planned tx metadata, gas caps, and simulated recovery
   against chain-sim live-backtest outcomes.

## Limiting Factors Of Each Module

| Order | Issue | Owner | Latest Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Real planner not wired in the trader** | `alpha/engine`, `alpha/live/trading` | `LivePrioritySellPlanner` and `LiveTradingPlannerBridge` exist and are unit-tested, but `eth_alpha_trader` still constructs `LiveChainSimExecutionAdapter`. | Add an explicit guarded real mode that instantiates `TxExecutorAdapter` with the bridge and production providers. |
| 2 | **Trader binary is chain-sim only** | `alpha/engine/src/bin/eth_alpha_trader.rs` | `--mode` normalization accepts only `chain-sim`; the binary always constructs `LiveChainSimExecutionAdapter`. | Add a guarded `kartal-direct-raw` or equivalent mode that records all real-capital config and refuses unsafe defaults. |
| 3 | **Receipt/fill reconciliation is missing** | `alpha/engine`, `tx_executor`, `kartal` | `TxExecutorAdapter` maps Kartal `broadcast` to submitted, but there is no watcher that turns tx hashes into confirmed/failed alpha execution reports. | Add a receipt tracker keyed by attempt id/tx hash and feed final reports into the existing store path. |
| 4 | **Private relay/builder execution is not implemented** | `tx_executor`, `kartal` | `BroadcastMode` supports `dry_run` and `public_mempool`; direct coinbase/private bundle bribes are explicitly outside `eth_direct_raw_v1`. | Decide if public mempool is enough for first live test; otherwise add a new protocol/version for relay or bundle submission. |
| 5 | **Gas rank is not on the live critical path** | `alpha/block_tx_rank`, `alpha/live/trading` | The rank crate exists and `tx_prep` accepts ranked fee candidates, but the planner currently only has a fixed test provider. | Build a gas-rank provider that emits conservative candidate bands from recent blocks plus next-block base-fee prediction. |
| 6 | **Direct EOA allowance policy is unresolved** | `alpha/live/trading`, `tx_simulator::tx_builders`, `solidity/baygus-executor` | Baygus Mode A now has vault emergency-sell calldata and internal approve+sell semantics. Direct EOA sells still need a live allowance reader or explicit pre-approval deployment policy. | Prefer Baygus Mode A for scam exits; only enable direct EOA sells after documenting pre-approval, permit/multicall, or two-transaction approval behavior. |
| 7 | **Live strategy evidence still needs real-planner shadowing** | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | Chain-sim live-backtest evidence exists, but it does not include Kartal-shaped tx metadata, gas-rank rejects, or signer/RPC failures. | Run the real planner with Kartal `dry_run` and compare every planned priority exit against live chain-sim outcomes. |
