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
only public broadcast exception is the explicit Alpha11 hold16 deploy strategy,
enabled with `--allow-public-mempool-live-validation` and
`--strategy-set alpha11-univ2-lp30-pool-update-block-hold16`.
That strategy spec has no `max_entry_pools`, no `--replay-current`, no `--once`,
`0.01 ETH` buy size, and a `0.555 ETH` bankroll cap.
While entry is otherwise in validation mode, live-real startup also requires a
resolved strategy bankroll of at most `0.555 ETH`; buys consume that starting
bankroll, confirmed sells replenish it, and profits can be redeployed. The
no-capital live chain-sim runner is `eth_alpha_live_backtest_trader`; historical
replay is `eth_alpha_backtest_trader`. Strategy economics and gates belong to
the selected strategy spec, so operators choose hold3/hold15/etc. by strategy
name instead of passing run parameters for buy size, liquidity floors, bankroll,
entry-pool cap, or hold duration.

The deployed Uniswap V2 trading vault is
`0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`, also recorded in the root
`config.toml` and `config.env`. Live final simulation and Kartal submission for
Mode A must target this vault. Direct Uniswap V2 router simulation is retained
only as a gas and behavior baseline; it is not the final pre-submit check for a
real live order.

Exact pre-submit simulation is also the source of gas-used for live tx
economics. Route builders carry gas limits, not fallback gas-used estimates.
After the final exact simulation the planner raises `route.estimated_gas_used`
to at least the simulated gas used plus the configured buffer, currently 2500
bps / 25%, before gas-rank lookup, value-cap budgeting, and Kartal request
metadata are built.

Production gas-rank readiness is tracked in:

`../readiness/gates/production_gas_rank/`

The real Alpha runner currently requires gas-rank candidates from
`eth_chain_server_gas_rank`, uses `p85 -> p75 -> p50 -> normal` for entries,
uses `p85 -> p75 -> p50 -> normal` for routine strategy exits, maps mempool
LP/removal race exits to `mempool_race`, maps mined LP approval exits to
`p90 -> p75 -> p50 -> normal`, caps selected priority fee at `3.5 gwei`, caps
entry estimated gas fee at `0.0012 ETH`, and caps exit estimated gas fee at
`0.002 ETH`. `mempool_race` is dependency-relative: it reads the triggering
pending tx priority fee and bids above it with a deterministic per-tx buffer in
the configured `0.1` to `0.2 gwei` range.

The decision loop is:

| Strategy signal | Decision reason | Execution category | Gas ladder | Execution route | Current usage |
| --- | --- | --- | --- | --- | --- |
| Eligible Alpha11 pool entry | `entry.buy_eligible_pool_once` | `entry_buy` | `p85 -> p75 -> p50 -> normal` | Kartal V2 vault buy | Active Alpha11 path |
| Max-hold / normal strategy exit | `exit.max_hold_active_blocks` or other strategy exit | `normal_exit` | `p85 -> p75 -> p50 -> normal` | Kartal V2 vault sell | Active Alpha11 path |
| Mined LP approval after entry | `exit.lp_approval_mined_race` or `exit.lp_approval_buy_confirm_block` | `lp_approval_exit` | `p90 -> p75 -> p50 -> normal` | Kartal priority V2 vault sell | Available confirmed-chain risk exit |
| Mined liquidity removal after entry | `exit.liquidity_removal` from `pool_update` | `mined_liquidity_removal_exit` | `p90 -> p75 -> p50 -> normal` | Kartal priority V2 vault sell | Confirmed-chain removal, not a mempool race |
| Mempool LP/removal risk | `exit.lp_approval` from `mempool_signal` or `exit.mempool_liquidity_removal_signal` | `mempool_race_exit` | `mempool_race` | Kartal priority V2 vault sell | Requires a pending dependency tx hash |
| Mempool trading-enabled tail entry | `entry.tail_after_enabling_tx` | `tail_entry_buy` | relative placement policy | Reserved V2 vault buy | Not Alpha11 default |
| Extreme emergency | strategy-specific emergency reason | `emergency_priority_exit` | disabled by default | Reserved priority sell | Reserved |

The design for turning a pending `trading_enabled` signal into a same-block
tail entry is documented in `docs/mempool_overlay_entry.md`.

`P50`, `P75`, `P85`, `P90`, and `P95` mean mined priority-fee percentiles.
They do not mean transaction positions. The separate `normal` profile is the
low-cost rank-target fallback: sampled priority needed to beat roughly
transaction position 50.

The gas-rank service adds `ALPHA_GAS_RANK_PRIORITY_TIE_BREAKER_GWEI` to the
selected raw sample value before returning a submit candidate. This avoids
submitting common whole-gwei prices such as exactly `2 gwei`, which would leave
our tx in the same priority-fee bucket as many other transactions.

Missing before the main hold15 strategy can use this crate for public real
capital:

- Review one complete hold3 public validation trade with mined buy, mined sell,
  actual fees, tx indexes, and 3-confirmation rechecks persisted in alpha.
- Keep hold16 entry bounded to a validation bankroll, currently
  `0.555 ETH`, until mined validation evidence and receipt operations are
  reviewed.

## Pre-Live Mined Validation Gate

The canonical readiness gate lives outside this crate README:

`../readiness/gates/pre_live_mined_validation/`

Alpha11's concrete hold3 validation instance is:

`../readiness/strategies/alpha11/hold3_mined_validation.md`

Keep this README focused on the transaction-prep boundary. Gate criteria,
operator runbooks, evidence requirements, and failed-attempt notes belong in the
readiness folder.

## Kartal Execution Handoff

The wire contract is `eth_unsigned_tx`, documented in
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
limited to `alpha11-univ2-lp30-pool-update-block-hold16` with the explicit
public-mempool flag. The deployed V2 vault buy and sell paths simulate the exact
prepared calldata against local Reth state, reject stale simulation state, use
simulated gas with a buffer for fee economics, and fetch route-specific gas-rank
recommendations from `eth_chain_server` before building the Kartal request.

## Tx Submission Data Flow

Canonical implemented live-capital flow:

```text
Alpha strategy/engine
  -> LiveTradingPlannerBridge
  -> exact deployed V2 vault simulation
  -> gas-rank policy
  -> Kartal direct-raw request
  -> tx_executor validation/sign/dry-run-or-broadcast
  -> alpha live_trader receipt reconciliation
```

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

## Gas-Rank Integration

The detailed bribe and gas-rank policy lives in `../../block_tx_rank/README.md`.
This crate consumes that evidence after exact route simulation and before Kartal
submission.

For `eth_unsigned_tx`, the selected bribe is the EIP-1559 priority fee in
`max_priority_fee_per_gas`; the same selected fee is mirrored into the Kartal
request `bribe` object for auditability. `tx_prep` applies the live gas-rank
ladder, required source gate, value cap, and request metadata. If no ranked
candidate fits, the planner rejects instead of creating a synthetic fallback.

Mempool LP approval and mempool liquidity-removal exits use dependency-relative
`mempool_race`; mined LP approval exits start from the configured P90 ladder.
Selected requests persist the gas plan and budget in metadata, while reject
outcomes include the candidate list, strategy gas policy, required source, and
budget for later analysis.

## LP Approval Priority Exit

The first live strategy contract is the LP approval priority-exit policy.

Question:

```text
If a pool's LP token is already materially approved, or becomes materially
approved while we hold the position, should we block entry or submit a priority
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
| `MempoolLpApproval` | LP approval was seen before mining. | Submit a dependency-relative `mempool_race` priority sell before the approval/removal path lands. |
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

## Deployment Readiness Gates

This crate is not enough to run real capital. Before real-capital deployment,
the live strategy needs:

- the explicit `eth_alpha_live_trader` real execution entrypoint;
- a production `LiveTxPlanningInputResolver` for entry and priority sell intents;
- live pre-submit simulation against the current state for any route beyond the
  deployed V2 vault buy/emergency sell;
- live gas-rank and base-fee inputs from `eth_chain_server` reviewed against
  mined validation evidence;
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
  `reason_source`, and `decision_reason`. Production callers can require a
  specific gas-rank source so fixed/test gas candidates cannot pass the real
  execution boundary.
- `src/planner/` orchestrates live transaction planning from
  `LivePrioritySellPlannerInput` to `LiveTraderTxSignal`. Route builders support
  direct Uniswap V2 ETH/WETH sells and deployed Uniswap V2 trading vault buys
  and emergency sells through `tx_simulator::tx_builders`. The deployed V2 vault
  simulator executes the exact route calldata against local Reth state, while
  the live-real runner uses `ChainServerGasRankProvider` for route-specific
  recommendations. Fixed gas-rank providers remain only for tests and
  calibration fixtures.
- `src/kartal_executor.rs` exposes the Kartal client and JSON contract for
  submitting already-prepared direct raw transactions.
- `src/kartal/` exposes status and policy-journal readers used by calibration
  and operator checks.
- `src/calibration/` runs repeatable policy-calibration suites against Kartal in
  dry-run mode.
