# Live Trading

`eth_live_trading` contains deployable live-trading policy scaffolding and the
guarded handoff to Kartal's ETH tx executor. It owns live transaction
preparation and audit metadata, but it does not sign, reserve nonces, or
broadcast transactions locally. Strategy code prepares an explicit transaction
request, attaches audit metadata, and submits it to Kartal.

## Current Deployment Status

This crate is now the transaction-prep boundary plus the first deployed V2 vault
planner scaffold. It can model LP approval priority exits, build Uniswap V2
ETH/WETH direct-sell calldata, build deployed Uniswap V2 trading vault buy and
emergency-sell calldata, derive a non-zero min-output from provisional
exact-calldata simulation, run a second exact-calldata pre-submit simulation for
the final deployed V2 vault calldata, consume gas-rank inputs, value-cap the
priority fee, build the Kartal JSON request, and submit that request to Kartal.

Current live-runner integration:

`eth_alpha_live_trader` now instantiates `TxExecutorAdapter` and the
`LiveTradingPlannerBridge` for deployed Uniswap V2 trading-vault buys and
emergency sells. That runner refuses non-dry-run Kartal status by default. The
only public broadcast exception is the explicit one-pool Alpha11 hold3 mined
validation run, enabled with `--allow-public-mempool-live-validation` and
`--strategy-set alpha11-live-univ2-lp30-pool-update-block-hold3-validation`.
That run must use `--max-entry-pools 1`, no `--replay-current`, no `--once`, and
`0.01 ETH` buy/bankroll caps so it can buy once, reconcile the mined receipt,
sell after 3 pool-update blocks, and record the mined evidence. While entry is
otherwise in validation mode, live-real startup also requires a resolved
strategy bankroll of at most `0.225 ETH`; buys consume that starting bankroll,
confirmed sells replenish it, and profits can be redeployed. The no-capital
live chain-sim runner is `eth_alpha_live_backtest_trader`; historical replay is
`eth_alpha_backtest_trader`.

The deployed Uniswap V2 trading vault is
`0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`, also recorded in the root
`config.toml` and `config.env`. Live final simulation and Kartal submission for
Mode A must target this vault. Direct Uniswap V2 router simulation is retained
only as a gas and behavior baseline; it is not the final pre-submit check for a
real live order.

Missing before the main hold15 strategy can use this crate for public real
capital:

- Replace `FixedGasRankProvider` with live gas-rank/base-fee inputs that produce
  `RankedFeeCandidate`s.
- Review one complete hold3 public validation trade with mined buy, mined sell,
  actual fees, tx indexes, and 3-confirmation rechecks persisted in alpha.
- Keep hold15 entry bounded to a small validation bankroll, currently
  `0.225 ETH`, until mined validation evidence and receipt operations are
  reviewed.

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

## Kartal Calibration

`src/calibration/` is the repeatable end-to-end dry-run test harness for this
boundary. It checks Kartal status, submits one or more prepared direct-raw
requests, reads Kartal's policy journal, and writes a verdict report under
`alpha/lab/reports/kartal_calibration/`.

Safe reject-by-default check:

```bash
cd /home/nima/code/crypto/blockchains/eth
ETH_TX_EXECUTOR_API_TOKEN=... cargo run -p eth_alpha_engine --bin eth_alpha_kartal_calibrate -- \
  --request alpha/live/trading/fixtures/kartal_calibration/reject_policy_request.json
```

Full dry-run signing check, after Kartal has a signer and narrow policy:

```bash
cargo run -p eth_alpha_engine --bin eth_alpha_kartal_calibrate -- \
  --planner-fixture \
  --planner-fixture-from 0x... \
  --expect dry-run-signed \
  --refresh-simulation-block
```

Use `--write-request-path /tmp/kartal-planner-produced-request.json` with
`--write-request-only` first when the operator needs to update Kartal's target
and selector allowlists to the exact planner-produced transaction before
submitting the signed dry-run.

Planner-fixture calibration appends a UTC timestamp to the generated
`attempt_id` by default. That keeps repeated dry-run signing attempts separate
in Kartal's policy journal and spend ledger.

Current bottleneck: the real-runner boundary exists, and public broadcast is
limited to the one-pool Alpha11 hold3 validation path. The deployed V2 vault buy
and sell paths simulate the exact prepared calldata against local Reth state and
reject stale simulation state. Gas rank still uses fixed values from the trader
CLI. Main-strategy public broadcast must remain disabled until gas-rank/base-fee
inputs, hold3 mined evidence, and receipt operations are reviewed.

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
       tx target address
       calldata
       ETH value
       gas limit and estimated gas used
     For Mode A Uniswap V2 live trading, the target is the deployed
     UniswapV2TradingVault, not the router. Buy calldata leaves output tokens in
     the vault. Emergency-sell calldata lets the vault approve the exact token
     amount internally, execute the router sell, and send recovered ETH to the
     configured treasury.
  -> AllowanceChecker proves the sell token can be spent, or marks the trading
     vault route as internally approved by the emergency-sell call
  -> PreSubmitSimulator simulates the exact calldata against current state
       for the deployed V2 vault and extracts tokens from BoughtV2 or recovered
       ETH from EmergencySoldV2
  -> GasRankProvider converts recent block-rank evidence into fee candidates
  -> tx_prep computes the value cap and chooses/rejects the gas plan
  -> LiveTraderTxSignal carries LiveDirectRawTransactionRequest to Kartal
  -> tx_executor validates, reserves nonce, signs, journals, and dry-runs or broadcasts
  -> TxExecutorAdapter records submitted/cancelled/failed ExecutionReport
  -> receipt reconciler later records confirmed/reverted outcome plus mined
     block, tx index, fee, bribe, and finality evidence
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
   candidate fits, the planner rejects. It must not synthesize an unranked
   value-cap candidate.

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

- the explicit `eth_alpha_live_trader` real execution entrypoint;
- a production `LiveTxPlanningInputResolver` for entry and priority sell intents;
- live pre-submit simulation against the current state for any route beyond the
  deployed V2 vault buy/emergency sell;
- live gas-rank and base-fee inputs wired into `tx_prep`;
- route policy that selects the deployed trading vault for Mode A scam exits, or
  a real allowance/pre-approval policy before direct EOA sells are allowed;
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
- `src/planner/` orchestrates live transaction planning from
  `LivePrioritySellPlannerInput` to `LiveTraderTxSignal`. Route builders support
  direct Uniswap V2 ETH/WETH sells and deployed Uniswap V2 trading vault buys
  and emergency sells through `tx_simulator::tx_builders`. The deployed V2 vault
  simulator executes the exact route calldata against local Reth state, while
  gas-rank and allowance providers remain injected so production code can
  replace fixed test implementations.
- `src/kartal_executor.rs` exposes the Kartal client and JSON contract for
  submitting already-prepared direct raw transactions.
- `src/kartal/` exposes status and policy-journal readers used by calibration
  and operator checks.
- `src/calibration/` runs repeatable policy-calibration suites against Kartal in
  dry-run mode.
