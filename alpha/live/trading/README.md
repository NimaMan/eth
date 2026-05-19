# Live Trading

`eth_live_trading` contains deployable live-trading policy scaffolding and the
guarded handoff to Kartal's ETH tx executor. It owns live transaction
preparation and audit metadata, but it does not sign, reserve nonces, or
broadcast transactions locally. Strategy code prepares an explicit transaction
request, attaches audit metadata, and submits it to Kartal.

## Current Deployment Status

This crate is now the transaction-prep boundary plus the first priority-sell
planner scaffold. It can model LP approval priority exits, build a Uniswap V2
ETH/WETH sell route, require a pre-existing allowance, consume final simulation
and gas-rank inputs, value-cap the priority fee, build the Kartal JSON request,
and submit that request to Kartal.

Missing before a live strategy can use this crate for real capital:

- Wire a production `LiveTxPlanningInputResolver` that maps `OrderIntent` plus
  current pool/position/store state into `LivePrioritySellPlannerInput`.
- Replace `FixedPreSubmitSimulator` with a live final simulation provider and
  produce `PreSubmitSimulation` for the exact planned calldata.
- Replace `FixedGasRankProvider` with live gas-rank/base-fee inputs that produce
  `RankedFeeCandidate`s.
- Replace `StaticAllowanceChecker` with an allowance reader or enforce a
  documented pre-approval requirement before positions can be opened.
- Feed `prepare_priority_sell` rejects/submits back into the engine's
  `TxExecutorAdapter` from an explicit real execution mode.
- Reconcile Kartal tx hashes into confirmed or failed alpha execution reports.

## Kartal Execution Handoff

The wire contract is `eth_direct_raw_v1`, documented in
`../../../tx_executor/README.md`. Alpha prepares the request and metadata;
Kartal hosts the HTTP endpoint; `tx_executor` validates, signs, and broadcasts
or dry-runs.

The first execution surface is prepared direct raw transaction submission:

```text
LiveTraderTxSignal
  -> LiveDirectRawTransactionRequest
  -> POST /eth/tx/direct-raw on Kartal's order server
  -> tx_executor validation/sign/dry-run-or-broadcast
```

`KartalExecutorClient::submit_signal` adds strategy metadata before submission:

- `strategy_name`
- `strategy_run_id`
- `trade_id`
- `token_address`
- `pool_address`
- `observed_block`

The request must include `to`, `data`, value, gas limit, EIP-1559 fee caps, and
optional simulation reference before it crosses the Kartal boundary. The tx-prep
modules in this crate own the live-trading policy and request assembly; reusable
route discovery, quoting, calldata builders, and simulation engines can stay in
lower-level crates and be wrapped here.

Current bottleneck: the running `eth_alpha_trader` still uses chain-state
simulation. It does not instantiate the engine's real `TxExecutorAdapter` or the
`LiveTradingPlannerBridge`, and the planner's production context resolver,
simulation provider, gas-rank provider, allowance reader, and receipt
reconciliation are not wired.

## Tx Submission Data Flow

The live-capital path has two separate decisions: the strategy decides whether an
order is required, then the live planner decides whether an exact transaction is
safe and valuable enough to submit.

```text
mempool/confirmed risk signal
  -> strategy emits StrategyDecision::SubmitOrder
  -> engine records OrderIntent and applies risk/capital gates
  -> TxExecutorAdapter asks LiveTradingPlannerBridge for a signal
  -> LiveTxPlanningInputResolver loads:
       open Position
       latest PoolSnapshot
       signer/from address
       current block and deadline
       min-output/slippage evidence
       observation metadata such as signal_id and source tx
  -> LivePrioritySellPlanner validates intent/position/pool consistency
  -> SellRouteBuilder builds PreparedSellRoute:
       router address
       calldata
       ETH value
       gas limit and estimated gas used
  -> AllowanceChecker proves the sell token can be spent
  -> PreSubmitSimulator simulates the exact calldata against current state
  -> GasRankProvider converts recent block-rank evidence into fee candidates
  -> tx_prep computes the value cap and chooses/rejects the gas plan
  -> LiveTraderTxSignal carries LiveDirectRawTransactionRequest to Kartal
  -> tx_executor validates, reserves nonce, signs, journals, and dry-runs or broadcasts
  -> TxExecutorAdapter records submitted/cancelled/failed ExecutionReport
  -> receipt reconciler later records confirmed/reverted/replaced outcome
```

The planner must reject instead of submitting when any critical input is stale,
inconsistent, unverifiable, or not worth the required gas/priority fee. Kartal
and `tx_executor` should receive only a final direct-raw transaction request,
never an abstract trading instruction.

## Miner Bribe / Gas Rank Policy

For the first direct-raw version, "miner bribe" means the EIP-1559 priority fee
paid through `max_priority_fee_per_gas`. The request also carries a `bribe`
object with the same selected priority fee so Kartal and executor journals can
audit the intent. Direct `block.coinbase` payments, bundles, and private relay
payments are not part of `eth_direct_raw_v1`.

The selection rule is:

1. Final simulation estimates the ETH value protected by landing before the
   scam path.
2. `PriorityFeeBudget` subtracts expected late recovery, safety buffer, and
   mandatory base-fee cost. The remainder is the maximum priority spend.
3. `eth_block_tx_rank` should produce candidate bands such as `p50_top_10`,
   `p50_top_25`, or `next_block_aggressive`, each with priority fee, max fee,
   expected rank, gas-before, and source window.
4. `tx_prep` filters out candidates whose priority spend or total max fee
   exceeds the protected-value cap.
5. Among eligible candidates, `tx_prep` chooses the best ranked candidate. If no
   candidate fits, public routing should reject; value-cap fallback should be
   reserved for explicitly configured protected/private routes.

This deliberately prevents us from paying more to escape than the position can
recover. The practical tuning question is not "highest bribe wins"; it is "what
recent rank band is fast enough for this signal, and does that band fit inside
the protected value?" Mempool LP approval exits should start at aggressive
recent-rank bands because the expected edge is pre-mine. Mined LP approval exits
should be more conservative unless the next-block removal probability is high.

Every submitted request should persist the chosen gas-rank label, priority fee,
max fee, estimated ETH spend, rank position, gas-before estimate, predicted base
fee, protected value, late recovery, and safety buffer in metadata.

## LP Approval Priority Exit

The first live strategy contract is the LP approval priority-exit policy.

Question:

```text
If a pool's LP token is already materially approved, or becomes materially
approved while we hold the position, should we block entry or submit an urgent
sell before direct liquidity removal?
```

Current answer:

- If LP approval above the configured threshold is visible before entry, block
  the buy.
- If we already hold the pool and a mempool LP approval appears, submit a
  priority sell immediately. This is the cleanest live edge because the signal
  is pre-mine.
- If we already hold the pool and an LP approval is first observed in a mined
  block, submit a priority sell for the next block if direct removal has not
  already been observed.
- If direct liquidity removal is already mined in the same or earlier block, the
  priority-exit edge is gone. We can still do recovery bookkeeping, but we
  cannot sell before a transaction that has already landed.

The default threshold is `>30%` of LP supply approved. This matches the shared
backtest rule we added after finding pools where LP approval was already visible
at buy confirmation.

## Live Signal Semantics

The policy separates three signal sources:

| Source | Meaning | Action |
| --- | --- | --- |
| `MempoolLpApproval` | LP approval was seen before mining. | Submit urgent sell before the approval/removal path lands. |
| `MinedLpApproval` | LP approval is confirmed, but removal is not confirmed yet. | Race the next block with a priority sell. |
| `MinedLiquidityRemoval` | Direct removal is already confirmed. | Mark too late for the priority edge. |

The one-block lead-time case is the critical live behavior:

```text
block N: LP approval mined
block N+1: likely direct liquidity removal
```

When block `N` lands and we hold the pool, the live trader should immediately
prepare and submit a sell with protected/private routing and strict fee caps.
This can only work when the removal is not already mined. It does not guarantee
ordering against private or unseen liquidity-removal transactions.

## Priority / Bribe Model

For this crate, "bribe" means a priority execution plan:

- choose a gas-rank candidate that is fast enough for the signal type;
- use a capped priority fee for public routing;
- enforce a max total ETH fee per trade;
- record the observed block and reason on the sell order.

Private/protected builder relay submission is the preferred future route for
pre-mine scam exits, but the current direct-raw executor only supports dry-run
and public mempool broadcast.

## Deployment Readiness Gates

This crate is not enough to run real capital. Before real-capital deployment,
the live strategy needs:

- an explicit real execution mode in `eth_alpha_trader`;
- a production `LiveTxPlanningInputResolver` for priority sell intents;
- live pre-submit simulation against the current state;
- live gas-rank and base-fee inputs wired into `tx_prep`;
- real allowance/pre-approval policy for sell tokens;
- per-trade max fee, capital limit, and circuit breaker enforcement;
- dry-run and shadow-mode evidence through Kartal's direct-raw path;
- receipt tracking that proves whether the sell landed before removal.

## Current Module

- `src/lp_approval_exit.rs` exposes `plan_lp_approval_response`. It is pure
  policy logic so it can be tested independently from RPC, signing, or
  broadcasting.
- `src/tx_prep/` turns an approved priority-exit plan plus a prepared sell route,
  simulation result, and gas-rank fee candidates into either a Kartal direct-raw
  request or an explicit reject reason. It enforces value-capped priority fees:
  the selected bribe must fit inside the ETH value protected by escaping before
  the scam path, after late-recovery and safety buffers. The request metadata
  includes structured decision-rationale fields such as `reason_code`,
  `reason_source`, and `decision_reason`.
- `src/planner/` orchestrates the v1 live priority-sell path from
  `LivePrioritySellPlannerInput` to `LiveTraderTxSignal`. The first route builder
  supports Uniswap V2 ETH/WETH sells through `tx_simulator::tx_builders`; the
  simulation, gas-rank, and allowance providers are injected so production code
  can replace the fixed test implementations.
- `src/kartal_executor.rs` exposes the Kartal client and JSON contract for
  submitting already-prepared direct raw transactions.
