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

## Current Simulator Fix State 2026-05-28

Status: the simulator ownership blocker is fixed in the current code and
deployed to the running live-backtest service.

Where this started:

- Live real trading and live backtests were using confusingly separate
  simulation paths. Real pre-submit simulation had moved toward chain-server,
  but chain-sim live-backtest settlement still depended on an Alpha-local
  `LiveTxSimulator`.
- A reorg/stale-frame case around blocks `25188709..25188710` wedged the
  previous live-backtest run because Alpha's local simulator had a stale parent
  hash and could never settle the exact execution block.
- Earlier direct live-state hydration could also construct a frame from an
  older historical base plus only the current block diff, which produced false
  router failures such as `INSUFFICIENT_LIQUIDITY` for pools that historical
  exact simulation could buy.

What is now addressed:

- Chain-server owns the single live `LiveTxSimulator` for ETH.
- Chain-server processes live heads by block hash and fetches matching prestate
  diffs by hash for direct live-state construction.
- Same-height canonical replacement heads are no longer skipped just because
  the block number was already seen.
- Direct live-state construction validates parent hash continuity and exact
  parent availability instead of silently falling back to an older state base.
- Alpha no longer maintains the normal live-backtest settlement simulator path.
  Chain-sim settlement asks chain-server
  `/api/v1/eth/live-tx-simulator/simulations/alpha-order` for exact-block
  order simulation.
- Real pre-submit simulation asks chain-server
  `/api/v1/eth/live-tx-simulator/simulations/unsigned-transaction`.
- Alpha and the mempool processor now use the versioned live-token and mempool
  APIs under `/api/v1/eth/...` instead of direct legacy `/eth/tokens/api/...`
  paths.

Current deployed evidence:

- Chain-server, mempool processor, `asena-static`, and the live backtester are
  running from rebuilt release binaries.
- Current live-backtest run:
  `alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-server-sim-20260528-151530Z`.
- At the latest check, live token tracker and chain-server `LiveTxSimulator`
  were both on block `25195314` with the same selected block hash, and
  `state_source="chain_server_live_tx_simulator"`.
- The current live-backtest result set is running with `poll_error=null`,
  `chain_sim_settlement_pending=0`, `chain_sim_settlement_waiting_state=0`, and
  `chain_sim_settlement_missing_block=0`.

## Block-Pinned Alpha Inputs 2026-05-28

Status: implemented for the live decision path; follow-up evidence metadata
remains.

Problem:

- Alpha used the block-applied update as a wakeup, then read the latest
  pool/token surface.
- That surface was live state, not a block-pinned frame. If block `N` woke the
  loop but block `N+1` was already applied before Alpha read pools, a decision
  could be recorded against block `N` while using newer pool state.
- EVM simulation was already exact-block and chain-server-owned, but strategy
  inputs needed the same block-frame boundary.

Required behavior:

- Chain-server exposes a replayable block-frame payload for block `N` after
  this ordered sequence finishes: live simulator session updated, token/pool
  updates applied, progress updated, block-applied event emitted.
- Alpha consumes that block frame for confirmed-chain pool updates. It does not
  make a block `N` strategy decision from an unpinned "latest" pool surface.

Implemented:

1. `eth_live_feed::LiveTokenEvent::BlockApplied` carries compact updated token
   snapshots.
2. `eth_chain_server` records `LiveBlockFrame` entries and serves
   `/api/v1/eth/live-trading/block-frames/next`, `latest`, and `{block}`.
3. Alpha live traders consume block frames and removed the live decision path
   that fetched `/live-token-tracker/pools`.

Remaining follow-up: implemented — see Third-Tier Implementation Items 4 and 5.

## Tier One Issue - Evidence Sufficiency Before Real Capital 2026-05-28

Status: active blocker for deployment.

Problem:

- The current replacement live backtest is healthy, but a healthy run is not by
  itself a promotion decision.
- We still need a reproducible current historical backtest window, a live
  shadow/live-backtest comparison, and trade-level audit over top/worst
  positions before starting new live-capital execution.

Next action:

1. Let the current live-backtest run collect enough fresh positions on the
   chain-server simulator path.
2. Audit top 10 and worst 10 positions with entry reason, skip/exit reason,
   exact decision block, exact simulation block, gas model, slippage/min-output,
   and PnL snapshot.
3. Compare current live-backtest behavior against the exact latest historical
   two-week backtest for the selected strategy policy.
4. Only after that review, start a real-live validation run with explicit
   bankroll, kill switch, max exposure, retry policy, and route policy visible.


## Current Limiting Factor

As of 2026-05-28, the live-capital bottleneck is production-readiness evidence
and policy/config ownership, not real transaction submission architecture. The
implemented real flow is documented in `alpha/live/trading/README.md` and
`alpha/engine/src/live_trader/real_execution/README.md`:

```text
Alpha strategy/engine
  -> planner
  -> exact deployed V2 vault simulation
  -> gas-rank policy
  -> Kartal eth_unsigned_tx request with explicit submission_policy
  -> tx_executor
  -> receipt reconciliation
```

The practical consequence is:

- The pipeline for tracking live real trades is implemented: Kartal
  `eth_unsigned_tx` submission, explicit public broadcast/tail submission
  policy, receipt reconciliation, vault event parsing, live trade pages, and the
  basic receipt lifecycle are in place.
- Before promoting a policy, the evidence base must expand from the current
  V2-centered executable backtests to the full protocol surface Risk Atlas sees,
  or explicitly account for every excluded protocol/pool as not routeable.
- Risk Atlas must become a formal policy contract: versioned cohorts, features,
  thresholds, targets, exclusions, and decision questions that Alpha and Asena
  can both cite.
- Real-live validation can public-broadcast through Kartal only when the live
  service is started with explicit public-broadcast validation enabled.
- Broader production remains blocked by evidence quality, block-pinned Alpha
  inputs, and operator visibility, not by missing real-trade tracking or missing
  gas outcome fields.
- The current real-validation bankroll limit is `0.555 ETH`, and the live
  gas-rank policy is configured/documented. The remaining parameter task is
  operator visibility: expose the live trading parameters in one review page
  instead of keeping a long bottleneck-register table here.

## Alpha Structure State 2026-05-24

Cleanup passes completed so far were intentionally behavior-preserving:

- `alpha/engine/src/live_trader/mod.rs` was reduced from `1489` lines to `993`
  by extracting bankroll restoration, config resolution, entrypoints, poll-error
  handling, restored runtime state, risk annotation, run metadata, and strategy
  construction.
- `alpha/engine/src/execution/real/mod.rs` now keeps production adapter code in
  the module and moved tests to `execution/real/tests.rs`.
- `alpha/engine/src/live_trader/real_execution.rs` is now folder-shaped:
  `live_trader/real_execution/mod.rs`, `tests.rs`, and `README.md`.
- `live_trader/`, `live_trader/backtest/`, and
  `live_trader/real_execution/` each have a single README explaining the folder
  boundary.
- `alpha/engine/src/live_trader/backtest/chain_sim_gas_policy.rs` is now
  folder-shaped: `mod.rs`, `policy_context.rs`, `metadata.rs`, `route.rs`,
  `shadow_outcome.rs`, and `README.md`.
- Stale empty alpha folders were removed.
- No execution semantics changed. Core buy/sell EVM simulation remains in
  `tx_processor/src/trade_simulation`; Alpha still owns strategy/runtime
  orchestration, valuation snapshots, execution reports, and persistence.

Verification run after the split:

```bash
cargo check -p eth_alpha_engine
cargo check -p eth_alpha_engine --bins
cargo check -p eth_alpha_backtest
cargo test -p eth_alpha_engine gate3 --lib
cargo test -p eth_alpha_engine execution::real --lib
cargo test -p eth_alpha_engine live_trader::real_execution --lib
```

Follow-up verification for the chain-sim gas-policy split:

```bash
cargo fmt -p eth_alpha_engine
cargo test -p eth_alpha_engine chain_sim_gas_policy --lib
cargo check -p eth_alpha_engine
```

Remaining structure bottlenecks:

| Area | Current state | Next action |
| --- | --- | --- |
| Live runner crate boundary | Live backtest and real-live service wiring still live under `eth_alpha_engine::live_trader`. | Create `alpha/live/runner` when we are ready to change package ownership of the live binaries. |
| Real execution runtime wiring | `live_trader/real_execution/mod.rs` is still `889` lines. | Split resolver, planner, preflight, gas-selection, and adapter-builder modules. |
| Simulated execution adapters | `execution/simulated/mod.rs` is still `881` lines. | Split historical adapter, live adapter, swap execution wrapper, params, and report helpers. |
| Receipt reconciliation | `live_trader/receipt_reconciliation.rs` is still `873` lines. | Split receipt provider, vault event decoder, evidence builder, and batch reconciler. |
| Engine tests | `engine/src/tests.rs` is still `1344` lines. | Split by runtime, valuation, lifecycle, and snapshot invariants. |

## Live Tracker V2-Heavy Pool Count Audit 2026-05-24

Question: why does the live token tracker currently report mostly V2 pools,
for example `tracked_v2_pools=227`, `tracked_v3_pools=0`,
`tracked_v4_pools=1`?

Findings:

- The current local API confirms the skew. At `2026-05-24T22:41:29+02:00`,
  `/eth/tokens/api/live-token-tracker/status` reported block `25167605`, warmup
  `25160532..25167531`, `live_blocks_processed=74`, `tracked_pools=228`,
  `tracked_v2_pools=227`, `tracked_v3_pools=0`, and `tracked_v4_pools=1`.
- This is not a simple V3/V4 decoder or discovery gap. The same status payload
  reported cumulative discovery of `discovered_v3_pools_unique=83` and
  `discovered_v4_pools_unique=26`.
- `/eth/tokens/api/live-token-tracker/pools` is backed by the current in-memory registry, not
  by the cumulative discovery sets. The read model iterates
  `state.processor.registry().tokens.values()` and builds `PoolView`s from the
  currently retained token pools in
  `eth_chain_server/src/read_models/live.rs:203`.
- Discovery is wired for all three protocols in the token router:
  `eth_token/src/tracking/token_update_router/mod.rs:245` calls V2 discovery,
  line `246` calls V3 discovery, and line `247` calls V4 discovery.
  V3 pool creation is handled from `tx.uniswap_v3_pools` in
  `pool_discovery/uniswap_v3.rs:20`; V4 pool creation is handled from
  `tx.uniswap_v4_initializes` in `pool_discovery/uniswap_v4.rs:19`.
- Progress counters are split into cumulative discovered sets and current
  tracked registry counts in `alpha/live/feed/src/runtime/apply_report.rs`.
  Lines `61..83` accumulate discovered/updated protocol sets, while
  lines `175..198` recompute current tracked V2/V3/V4 pools from the registry.
- Live retention is active. The live retention policy from
  `/eth/tokens/api/live-token-tracker/retention` has `min_weth_denom_reserve=0.1`,
  `min_stable_denom_reserve=1000.0`, `min_other_denom_reserve=0.0`, and
  `retain_liquidity_removal_pools_for_blocks=15000`.
- The same status payload reported `retention_dropped_v2_pools=144`. That
  counter name is misleading: the retention code evaluates `token.all_pool_bases()`
  in `eth_token/src/tracking/retention/live.rs:111`, and the removal loop deletes
  matching addresses from `v2_pools`, `v3_pools`, `v4_pools`, Curve, and Balancer
  maps at lines `175..180`.
- The retained current pool surface is mostly terminal/scam V2. At the same API
  snapshot, `/eth/tokens/api/live-token-tracker/pools` returned `228` pools:
  `227 UNISWAP-V2`, `1 UNISWAP-V4`, `0 UNISWAP-V3`; `207` of the V2 pools were
  marked scam, and only `17` pools were active. The single retained V4 pool had
  an unsupported/non-WETH currency and `denom_reserve=0.0`, which is retained by
  the current `min_other_denom_reserve=0.0` policy.

Likely cause:

- The V2-heavy live count is mainly a retention/read-model artifact, not proof
  that V3/V4 ingestion is absent. V3/V4 pools are discovered during warmup/live
  processing, but most are not present in the current retained registry by the
  time the `/live-token-tracker/pools` and `tracked_*` counters are rendered.
- There is still a real observability bug: retention/drop metrics and field
  names say `v2` even though the implementation applies to all pool types. We
  currently cannot tell from the public API exactly how many V3 vs V4 pools were
  dropped, or by which retention reason, after the fact.
- The current strategy and executable simulation path are intentionally
  V2-centered, but the live tracker itself is broader than V2. The current pool
  count should therefore be read as "currently retained route surface", not
  "all protocol discoveries seen by the live tracker".

Verification commands:

```bash
curl -fsS http://127.0.0.1:40019/eth/tokens/api/live-token-tracker/status |
  jq '.progress | {status,current_block,warmup_start_block,warmup_end_block,live_blocks_processed,discovered_v2_pools_unique,discovered_v3_pools_unique,discovered_v4_pools_unique,tracked_v2_pools,tracked_v3_pools,tracked_v4_pools,tracked_pools,retention_dropped_v2_pools,pool_simulation_failures}'

curl -fsS http://127.0.0.1:40019/eth/tokens/api/live-token-tracker/pools |
  jq '{count,total_count,protocols:(.pools|group_by(.protocol)|map({protocol:.[0].protocol,count:length}))}'

curl -fsS http://127.0.0.1:40019/eth/tokens/api/live-token-tracker/pools |
  jq '{total:.total_count, scam_count:.scam_count, active_count:.active_count, v2_scam:(.pools|map(select(.protocol=="UNISWAP-V2" and .is_scam==true))|length), v2_non_scam:(.pools|map(select(.protocol=="UNISWAP-V2" and .is_scam!=true))|length)}'

curl -fsS http://127.0.0.1:40019/eth/tokens/api/live-token-tracker/retention |
  jq '{policy:.policy, progress:{current_block:.progress.current_block,tracked_v2_pools:.progress.tracked_v2_pools,tracked_v3_pools:.progress.tracked_v3_pools,tracked_v4_pools:.progress.tracked_v4_pools}}'
```

Recommended next steps:

1. Rename retention fields that say `v2` but apply to all pools, or add
   protocol-specific fields alongside them.
2. Add live API counters for `discovered`, `retained`, and `dropped` by protocol
   and drop reason. This should include the latest dropped V3/V4 examples.
3. Decide whether `/live-token-tracker/pools` should remain a retained-registry view or also
   expose a cumulative discovery/history view. Both are useful, but they answer
   different questions.
4. Before making protocol-readiness claims, compare retained vs discovered V3/V4
   examples against chain truth and the executable route simulator. If they are
   intentionally excluded, the frontend should show the exclusion reason.

## Tail-Entry Production Parity Blockers 2026-05-24

The current tail-entry hardening is useful, but it does not yet prove final
production parity for same-block entry after a trading-enabled mempool
dependency. These are active blockers before trusting tail-entry with live
capital:

| Order | Blocker | Required evidence |
| --- | --- | --- |
| 1 | Non-vacuous coverage checks | Strategy validation must count `trading_enabled` signals, signals with `mempool_entry_evidence`, successful exact-vault eligible signals, tail-entry intents, and tail-entry submitted/confirmed/deferred/failed/cancelled outcomes. Alpha11 tail-entry validation is blocked when this path is not actually exercised. |
| 2 | Config-bound exact-vault evidence | `mempool_entry_evidence` must prove the configured production vault address, chain id, buy amount, and route version, not just a generic exact-vault-shaped route. |
| 3 | Real receipt ordering preservation | Real order journal and receipt reconciliation must preserve `tail_after_tx_hash`, dependency priority fee, dependency gas price, selected priority fee, and ordering intent in final mined evidence. |
| 4 | Dependency-relative tail gas policy | `tail_entry_buy` gas selection must compare our selected fee against the enabling transaction and validate the intended behind-dependency ordering. |
| 5 | Same-block overlay proof | Backtest/live-backtest must either simulate dependency transaction plus exact vault calldata in the same overlay state or explicitly mark the run as post-mine `N+1` only and block same-block readiness. |

## Live Trading Parameter Surface

The old hardcoded-value register has been removed from this bottleneck ledger.
For the current validation path, use the configured `0.555 ETH` bankroll limit
and the live gas-rank policy documented in:

- `alpha/live/trading/README.md`
- `alpha/block_tx_rank/README.md`

Later, build a single operator-facing live trading parameters page in Asena. It
should show the active bankroll, buy size, signer/from address, vault address,
gas-rank policy, gas/fee caps, slippage/min-output policy, simulation freshness,
receipt finality settings, Kartal mode, and signer/Kartal policy caps. That page
is the right place to make duplicated live parameters visible and reviewable;
`bogaz.md` should only track it as a broad product/ops task.

## Limiting Factors Of Each Module

| Order | Issue | Owner | Latest Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Chain/source/simulator parity must be proven first** | `risk_atlas`, `eth_token`, `tx_processor::trade_simulation`, `alpha/backtest` | The V4 observed-flow eligibility leak is fixed, current open P0 parity blockers are none, and `Compass` helper-route V2 sells with meaningful same-block liquidity are now documented as a route-shape parity gap rather than a drained-pool case. Remaining P1 rechecks still affect evidence trust: V3 pool identity and amount-quality/dust sell parity. | Work the ranked promotion queue in `risk_atlas/investigations/README.md` before using affected cohorts for policy or PnL claims. Reproduce each old issue on current code, then either open a focused investigation or close it as stale. |
| 2 | **Backtests must cover the full Risk Atlas protocol surface** | `alpha/backtest`, `alpha/engine`, `risk_atlas`, `tx_processor::trade_simulation`, `eth_token` | Current executable strategy evidence is V2-centered. Risk Atlas corpus reports include `UNISWAP-V2`, `UNISWAP-V3`, and `UNISWAP-V4`; the older strategy report explicitly notes V3/V4 routing metadata was not yet persisted enough for chain-sim execution. A production policy cannot claim all-surface readiness while only executable on a subset. | Extend replay/backtest ingestion and execution metadata so each protocol is either executable with correct route simulation/valuation or explicitly counted as excluded with reason. Then rerun the selected policy over the exact target window with per-protocol PnL, skipped-pool counts, routeability counts, and all-protocol aggregate performance. |
| 3 | **Risk Atlas policy contract is not formalized enough for promotion** | `risk_atlas`, `alpha/strategies`, `alpha/lab`, `interface/asena` | Risk Atlas has durable DB/read-model docs, but the policy-facing contract is still implicit: thresholds such as init age, price-to-initial, LP approval timing, direct LP removal horizon, protocol filters, labels, and exclusion reasons are not yet one versioned artifact that Alpha and Asena both cite. | Create a versioned Risk Atlas policy contract that defines eligible cohort, protocol/denom routeability, feature names, target labels, threshold sources, exclusion reasons, calibration windows, and decision-question outputs. Alpha strategies should reference this contract version in backtest/live metadata. |
| 4 | **Live trading parameter page is missing** | `alpha/engine`, `kartal`, `interface/asena` | The validation bankroll is `0.555 ETH` and the gas policy is configured, but operators still need one page showing the active live-capital parameters and where each value came from. | Build an Asena live trading parameters page that pulls active values from Alpha/Kartal/signer status instead of maintaining another hidden duplicate list. |
| 5 | **Direct EOA allowance policy is unresolved** | `alpha/live/trading`, `tx_simulator::tx_builders`, `solidity/baygus-executor` | Mode A now has vault emergency-sell calldata and internal approve+sell semantics. Direct EOA sells still need a live allowance reader or explicit pre-approval deployment policy. | Prefer Mode A for scam exits; only enable direct EOA sells after documenting pre-approval, permit/multicall, or two-transaction approval behavior. |
| 6 | **Live strategy evidence still needs real-planner shadowing** | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | Chain-sim live-backtest evidence exists, but it does not include Kartal-shaped tx metadata, gas-rank rejects, or signer/RPC failures. | Run the real planner with Kartal `dry_run` and compare every planned priority exit against live chain-sim outcomes. |
| 7 | **Mempool signal latency attribution is incomplete** | `mempool_processor`, `eth_chain_server`, `alpha/engine` | The current 40020 API exposes `signal_created_at`, `signal_source`, and `mempool_first_seen_*`, and the mempool processor is publishing fresh signals on the rebuilt code. We still do not have a current timing ledger proving where any remaining store-to-trader delay occurs. | Let the current run collect fresh signal/decision pairs, then compare `mempool_first_seen_at`, `detection_timestamp`, `signal_events.created_at`, API `signal_created_at`, trader observation time, risk event time, and report completion before changing queue or trader architecture. |

## Third-Tier Implementation Items

These are planned improvements, but they are not current blockers for the next
validation step or the immediate second-tier cleanup.

| Order | Item | Owner | Current state | Next action |
| --- | --- | --- | --- | --- |
| 1 | **Private relay/builder execution** | `tx_executor`, `kartal` | `BroadcastMode` supports `dry_run` and public RPC/mempool routes; direct coinbase/private bundle bribes are explicitly outside the current `eth_unsigned_tx` wire contract. | Add a new protocol/version for relay or bundle submission when we are ready to route priority exits outside the public mempool. |
| 2 | **Silent simulator gap on session build failure** | `alpha/live/feed`, `eth_chain_server` | When `build_direct_live_block_session` fails (state diffs invalid or unavailable), the simulator is not updated for that block but `BlockApplied` still fires. Alpha receives the block frame and may attempt a buy; the pre-submit simulation then hard-errors and the order is correctly rejected, but the failure was invisible at the block-frame level. Logging fixed in `block_apply.rs`: both the build-error and the no-diffs case now emit `simulator_state_published=false` and `block_hash` so the gap is queryable. | Monitor for `simulator_state_published=false` log lines during live runs. If they appear repeatedly on the same blocks, investigate why state diffs are missing or invalid for those blocks. |
| 4 | **Frame source/hash in execution reports** | `alpha/engine/src/live_trader` | Implemented: `LiveRealInputResolver::source_metadata` now includes `input_frame_block` and `input_frame_hash` so every execution report carries the exact block frame that woke the decision loop. The Arcs are updated in the main loop on each advancing frame. | Monitor that `input_frame_block` matches `decision_block` for pool-update entries and that `input_frame_hash` is present. If `input_frame_hash` is null in a report, the loop iterated without a fresh frame before the signal fired. |
| 5 | **Operational alert for live block frame gap** | `alpha/engine/src/live_trader` | Implemented: the main loop now warns at `LIVE_BLOCK_FRAME_GAP_WARN_THRESHOLD = 3` skipped blocks with `prev_last_frame_block`, `new_frame_block`, and `gap` fields, referenced against `ring_buffer_cap = 16`. | If this warning fires repeatedly, investigate whether chain-server is failing to produce frames or Alpha's loop is stalling between polls. |
| 3 | **Inconsistent simulator state on reorg clear failure** | `alpha/live/feed`, `eth_chain_server` | On a parent-hash mismatch (reorg), `prune_stale_direct_live_state` clears the local session map then calls `provider().clear()` on the ring buffer. If `provider().clear()` fails, the local map is empty but the ring buffer retains stale entries from the old fork. A new session is then published on top, so the current block simulates correctly, but the stale entries for older blocks linger until naturally evicted. Logging fixed in `direct_live_state.rs`: the clear-failure case is now `error!` level with `simulator_state_consistent=false` instead of a plain `warn!`. | Monitor for `simulator_state_consistent=false` log lines. If observed, investigate what conditions cause `provider().clear()` to fail in the ring buffer implementation. |

## Mempool Signal Timing Bottleneck 2026-05-22

Current conceptual finding: signal production and signal consumption must be
measured separately before changing more code.

Latest state after the 2026-05-28 rebuild/restart:

- `mempool_processor` is running from the rebuilt release binary and publishing
  fresh `trading_enabled` and `lp_position_approval` signals.
- The 40020 API now exposes the current signal shape with `signal_source`,
  `signal_created_at`, and `mempool_first_seen_*`.
- The live backtester is running from the rebuilt release binary against
  chain-server-owned simulation state, with no pending/missing settlement.
- The old evidence of 7-70 second trader-side signal delay is not yet remeasured
  on this current process set, so it remains an attribution task rather than a
  proven current bottleneck.

What needs to be fixed or proven, in order:

1. Let the latest code run long enough to capture fresh live signals, then build
   a timing table with these stages: mempool detect, signal stored, API visible,
   trader received, risk event recorded, reports/decision finished.
2. If store-to-receive is high, inspect token-server/API polling, query limits,
   and whether the trader is reading an old process or stale endpoint.
3. If receive-to-decision is high, simplify the trader critical path: risk
   signals should not wait behind expensive chain-sim market/report work when a
   fast exit decision is needed.
4. Only after the latest timing table is clear, revisit the mempool simulation
   queue priority/drop behavior. There is a suspected priority-drop inversion,
   but it should not be patched blindly while the live evidence points at
   trader-side blocking.

Keep this bottleneck simple: the desired output is an explainable timing ledger,
not more queue layers. Every proposed fix should reduce one measured interval in
the timing table above.
