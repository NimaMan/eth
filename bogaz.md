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

## Tier One Issue - Live Backtest Settlement Still Uses Alpha-Local Simulator 2026-05-28

Status: active blocker. Do not treat live-backtest evidence after block
`25188709` in run
`alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-server-sim-20260526-201930Z`
as valid until this is fixed.

Problem:

- The current live-backtest run is heartbeating, but it is wedged on `10`
  submitted buys from block `25188709`.
- Those buys expected chain-sim settlement at block `25188710`; the run is now
  thousands of blocks ahead and repeatedly logs
  `chain-sim submitted execution is due but exact live state block is not
  available yet`.
- Strategy processing is deferred with
  `chain_sim.live_backtest.waiting_for_exact_settlement_state`, so the run is
  live but no longer producing trustworthy post-block evidence.

Evidence:

- Source audit:
  - real pre-submit simulation uses
    `ChainServerLivePreSubmitSimulator` and calls
    `/api/v1/eth/live/simulations/unsigned`;
  - live-backtest chain-sim settlement still constructs an Alpha-local
    `LiveTxSimulator` in `live_trader/runtime/execution_stack.rs`;
  - `ChainSimSettlement` calls
    `LiveChainSimExecutionAdapter.live_simulator().has_state_at(...)` and then
    `simulate_submitted_order(...)`.
- Chain-server owner-path audit:
  - `LiveTokenRuntime` already owns a server-side `LiveTxSimulator`;
  - `apply_loaded_block` builds and publishes the direct live block session
    before token/pool state is updated and before block-applied events are sent;
  - `build_direct_live_block_session` first tries to advance cached parent
    session `B-1`, but if that cached parent hash is stale after a reorg it
    returns an error instead of falling back to rebuild from prestate diffs;
  - `LiveChainRuntime::apply_processed_with_gap_fill` ignores incoming processed
    heads whose block number is `<= current`, so same-height replacement heads
    from a reorg can be skipped;
  - `LiveBlockProcessor` processes the block by hash but fetches prestate diffs
    by block number, so the hash-fetched block and number-fetched diffs can
    diverge around a reorg unless we add hash-based diff tracing or explicit
    hash validation.
- Running binary audit:
  - `target/release/eth_alpha_live_backtest_trader` was built at
    `2026-05-26 22:18:48+02`;
  - the process for the affected run started at
    `2026-05-26 22:19:29+02`, before the current simulator-boundary audit.
- Reorg evidence:
  - the canonical Reth block `25188709` hash is
    `0x0ff1d72aacd22fea261a38491c2cc433af478b0a55306cca3559791f58f6e763`;
  - canonical block `25188710` parents that hash;
  - Alpha had published a local live-state frame for block `25188709` with
    hash `0x83611fc00bb19d3f7b62deb6f16e43625c76c8ae1fc859a29b9788159a174cf9`;
  - the local simulator rejected block `25188710` with a parent-hash mismatch
    and never recovered the exact settlement block.

Root cause:

- We delegated real-live pre-submit simulation to chain-server, but not
  live-backtest settlement.
- Live-backtest settlement still depends on Alpha's in-process live state
  window. A reorg or stale frame can make one exact settlement block
  permanently unavailable, leaving submitted orders unresolved and blocking
  later strategy processing.
- Rebuilding/restarting the current source alone is not sufficient: the latest
  source still contains this Alpha-local live-backtest settlement path.

Required behavior:

- Chain-server owns live state and `LiveTxSimulator` sessions.
- Alpha owns strategy decisions, order intent, gas policy, and reporting.
- Chain-server must accept canonical replacement heads and either rebuild the
  affected exact sessions or return a structured unavailable/reorg result.
- For live backtests, Alpha should submit a small settlement simulation request
  to chain-server for the exact execution block. It should not maintain a
  separate live-state simulator for normal settlement.
- If chain-server cannot serve an exact settlement block because of a reorg,
  Alpha must record an infrastructure settlement result with the affected order,
  canonical block hash, local/stale hash, and retry/skip status. It must not
  wedge the whole run indefinitely.

Plan:

1. Stop the affected run and mark the result set as invalid after block
   `25188709` for review purposes.
2. Fix chain-server live-state ownership first:
   - do not skip same-height replacement heads;
   - prune/rebuild direct live sessions on parent-hash mismatch;
   - fetch prestate diffs by block hash when supported, or validate that
     number-fetched diffs match the processed block hash before publishing.
3. Add a chain-server live-backtest settlement endpoint that accepts the order
   intent, submitted block, expected execution block, and route context, then
   simulates against chain-server-owned exact state.
4. Replace `ChainSimSettlement`'s direct
   `LiveChainSimExecutionAdapter.live_simulator()` dependency with a
   chain-server settlement client.
5. Remove Alpha's normal live-backtest `LiveTxSimulator` dependency:
   - stop spawning the live-state stream publisher in the live-backtest stack;
   - remove local simulator wait gates from polling;
   - move position valuation and manual-close balance checks to chain-server
     simulation/view endpoints.
6. Add reorg/stale-frame handling: if the requested exact block is gone or its
   parent hash changed, settle with an explicit infrastructure state outcome
   instead of waiting forever.
7. Add a validation check for stale submitted reports older than a small block
   threshold with no terminal settlement report.
8. Rebuild and restart the live backtester only after the settlement path no
   longer depends on Alpha-local live simulator state.

## Tier One Issue - Exact Parent Live Simulation State 2026-05-25

Status: fixed in code and deployed to the current live-backtest run, but keep it
as the top active issue until the replacement run has enough confirmed buys and
no exact-parent state errors.

Problem:

- Previous live-backtest run
  `alpha11-univ2-lp30-pool-update-block-hold-sweep-all-pools-hold16-chain-sim-bankroll555-livetx-decimals-20260525-160501Z`
  produced `20` buy failures after regular market entries.
- The failures split into `10` buys at block `25173411` with
  `UniswapV2Library: INSUFFICIENT_LIQUIDITY` and `10` buys at block `25173474`
  with an empty revert from the Uniswap V2 router.
- These were not mempool `trading_enabled` entries. Chain-sim live backtest
  already skips those with reason code
  `chain_sim.live_backtest.skip_mempool_trading_enabled`.
- Direct historical simulation for the same pools and target blocks succeeded,
  which proves the failures were not genuine router outcomes for those two
  samples.

Root cause:

- The live simulator could construct block `N` from a historical base older than
  `N-1` and overlay only block `N` diffs.
- That dropped intervening state, including newly created pools and reserves,
  so the router simulation saw liquidity that was missing or inconsistent.
- The symptom looked like a strategy buy failure, but it was infrastructure
  state drift in live direct-state hydration.

Required behavior:

- Live trading must only use an exact live state frame for pre-submit
  simulation. The normal source is the in-memory parent session for `N-1`,
  advanced with block `N` prestate diffs. A bootstrap path may use Reth only
  when Reth can provide the exact parent state and matching parent header. It
  must not use an older historical base.
- If exact state is unavailable, live trading should fail readiness or skip
  submission rather than simulate from fabricated state. This can reduce entries
  after restarts or stream gaps, but it prevents live orders from being sent
  using a bad state model.
- Live backtest submits at observed block `N` and settles at the modeled mined
  block `N+1`. Settlement must use the exact `N+1` live block session. If that
  exact frame is unavailable, the position should wait or be marked with an
  infrastructure state reason, not become a router buy failure.

Fix:

- `tx_simulator` now rejects direct live state construction unless the exact
  parent state and parent hash are available.
- `eth_live_feed` advances direct live sessions from the exact in-memory parent
  when possible and no longer rebuilds from an older historical base after a
  parent-session error.
- `eth_alpha_engine` live state hydration checks parent hash continuity before
  publishing a frame to the live simulator provider.
- The related folder READMEs now document the event sequence and exact-parent
  state contract.

Current evidence:

- Current replacement run
  `alpha11-univ2-lp30-pool-update-block-hold-sweep-all-pools-hold16-chain-sim-bankroll555-livetx-decimals-20260525-163527Z`
  is running with `chain_sim_state_source="InMemoryLiveBlockSession"`.
- As of the latest check, it has `20` positions in `buy_confirmed`, `20`
  submitted buy reports, `20` confirmed buy reports, and `0` buy failures.
- No `INSUFFICIENT_LIQUIDITY`, empty router revert, or exact-parent state errors
  are present in the current run reports.

Residual risk and next action:

- Parent hash mismatch or missing exact parent state now stops live-frame
  construction instead of falling back. That is the correct safety behavior, but
  it needs monitoring because it can pause entries after restarts, stream gaps,
  or reorgs.
- Keep monitoring the replacement live backtest before reviewing strategy PnL.
  If a future buy fails, first classify it as genuine router outcome versus
  infrastructure state availability by checking exact target block, state source,
  and simulator error text.


## Current Limiting Factor

As of 2026-05-24, the live-capital bottleneck is production-readiness evidence
and policy/config ownership, not real transaction submission architecture. The
implemented real flow is documented in `alpha/live/trading/README.md` and
`alpha/engine/src/live_trader/real_execution/README.md`:

```text
Alpha strategy/engine
  -> planner
  -> exact deployed V2 vault simulation
  -> gas-rank policy
  -> Kartal direct-raw request
  -> tx_executor
  -> receipt reconciliation
```

The practical consequence is:

- The pipeline for tracking live real trades is implemented: Kartal direct-raw
  submission, receipt reconciliation, vault event parsing, live trade pages, and
  the basic receipt lifecycle are in place.
- Before promoting a policy, the evidence base must expand from the current
  V2-centered executable backtests to the full protocol surface Risk Atlas sees,
  or explicitly account for every excluded protocol/pool as not routeable.
- Risk Atlas must become a formal policy contract: versioned cohorts, features,
  thresholds, targets, exclusions, and decision questions that Alpha and Asena
  can both cite.
- The explicit Alpha11 hold16 validation service can public-broadcast through
  Kartal only with `--allow-public-mempool-live-validation`.
- Broader hold15 production remains blocked by public-mempool/private-relay
  policy and operator visibility, not by missing real-trade tracking or missing
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
  `/eth/tokens/api/live/status` reported block `25167605`, warmup
  `25160532..25167531`, `live_blocks_processed=74`, `tracked_pools=228`,
  `tracked_v2_pools=227`, `tracked_v3_pools=0`, and `tracked_v4_pools=1`.
- This is not a simple V3/V4 decoder or discovery gap. The same status payload
  reported cumulative discovery of `discovered_v3_pools_unique=83` and
  `discovered_v4_pools_unique=26`.
- `/eth/tokens/api/live/pools` is backed by the current in-memory registry, not
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
  `/eth/tokens/api/live/retention` has `min_weth_denom_reserve=0.1`,
  `min_stable_denom_reserve=1000.0`, `min_other_denom_reserve=0.0`, and
  `retain_liquidity_removal_pools_for_blocks=15000`.
- The same status payload reported `retention_dropped_v2_pools=144`. That
  counter name is misleading: the retention code evaluates `token.all_pool_bases()`
  in `eth_token/src/tracking/retention/live.rs:111`, and the removal loop deletes
  matching addresses from `v2_pools`, `v3_pools`, `v4_pools`, Curve, and Balancer
  maps at lines `175..180`.
- The retained current pool surface is mostly terminal/scam V2. At the same API
  snapshot, `/eth/tokens/api/live/pools` returned `228` pools:
  `227 UNISWAP-V2`, `1 UNISWAP-V4`, `0 UNISWAP-V3`; `207` of the V2 pools were
  marked scam, and only `17` pools were active. The single retained V4 pool had
  an unsupported/non-WETH currency and `denom_reserve=0.0`, which is retained by
  the current `min_other_denom_reserve=0.0` policy.

Likely cause:

- The V2-heavy live count is mainly a retention/read-model artifact, not proof
  that V3/V4 ingestion is absent. V3/V4 pools are discovered during warmup/live
  processing, but most are not present in the current retained registry by the
  time the `/live/pools` and `tracked_*` counters are rendered.
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
curl -fsS http://127.0.0.1:40019/eth/tokens/api/live/status |
  jq '.progress | {status,current_block,warmup_start_block,warmup_end_block,live_blocks_processed,discovered_v2_pools_unique,discovered_v3_pools_unique,discovered_v4_pools_unique,tracked_v2_pools,tracked_v3_pools,tracked_v4_pools,tracked_pools,retention_dropped_v2_pools,pool_simulation_failures}'

curl -fsS http://127.0.0.1:40019/eth/tokens/api/live/pools |
  jq '{count,total_count,protocols:(.pools|group_by(.protocol)|map({protocol:.[0].protocol,count:length}))}'

curl -fsS http://127.0.0.1:40019/eth/tokens/api/live/pools |
  jq '{total:.total_count, scam_count:.scam_count, active_count:.active_count, v2_scam:(.pools|map(select(.protocol=="UNISWAP-V2" and .is_scam==true))|length), v2_non_scam:(.pools|map(select(.protocol=="UNISWAP-V2" and .is_scam!=true))|length)}'

curl -fsS http://127.0.0.1:40019/eth/tokens/api/live/retention |
  jq '{policy:.policy, progress:{current_block:.progress.current_block,tracked_v2_pools:.progress.tracked_v2_pools,tracked_v3_pools:.progress.tracked_v3_pools,tracked_v4_pools:.progress.tracked_v4_pools}}'
```

Recommended next steps:

1. Rename retention fields that say `v2` but apply to all pools, or add
   protocol-specific fields alongside them.
2. Add live API counters for `discovered`, `retained`, and `dropped` by protocol
   and drop reason. This should include the latest dropped V3/V4 examples.
3. Decide whether `/live/pools` should remain a retained-registry view or also
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
| 7 | **Mempool signal latency attribution is incomplete** | `mempool_processor`, `eth_chain_server`, `alpha/engine` | Latest DB evidence shows `live_trading.signal_events` is usually written within sub-second latency, while the running trader can process some signals 7-70 seconds later. The committed receive-timing fields were not visible from the running chain-server/API yet, so the current process was not on the latest timing code. | Restart chain-server and the live-backtest trader on the latest commit, let them run, then compare `detection_timestamp`, `signal_events.created_at`, API `signal_created_at`, trader observation `first_seen_at`, risk event time, and report completion before changing queue or trader architecture. |

## Third-Tier Implementation Items

These are planned improvements, but they are not current blockers for the next
validation step or the immediate second-tier cleanup.

| Order | Item | Owner | Current state | Next action |
| --- | --- | --- | --- | --- |
| 1 | **Private relay/builder execution** | `tx_executor`, `kartal` | `BroadcastMode` supports `dry_run` and `public_mempool`; direct coinbase/private bundle bribes are explicitly outside `eth_direct_raw_v1`. | Add a new protocol/version for relay or bundle submission when we are ready to route priority exits outside the public mempool. |

## Mempool Signal Timing Bottleneck 2026-05-22

Current conceptual finding: signal production and signal consumption must be
measured separately before changing more code.

Observed latest state before restarting onto commit `1d9e16e3`:

- `mempool_processor` persisted recent signals quickly. Examples from
  `live_trading.signal_events`: detect-to-store ranged from about `6 ms` to
  `962 ms` for the latest sampled rows.
- The running chain-server/API at `40019` still returned the older signal shape;
  it did not expose `signal_source`, `signal_created_at`, or
  `mempool_first_seen_*`, so the process had not picked up the committed timing
  fields yet.
- The running trader had `poll_interval_ms=2000`, but some signals were still
  processed much later than polling alone explains. Example store-to-risk
  delays included about `7.9s`, `9.9s`, `33.9s`, `56.4s`, and `68.6s`.
- The worst sampled behavior looked like sequential trader-side blocking:
  while one signal/engine call generated reports, later signals waited behind
  it. That points first at trader event-loop/backpressure, not at mempool signal
  creation.

What needs to be fixed or proven, in order:

1. Run the latest committed code and prove the process is current. The signal
   API must expose `signal_created_at`, `signal_source`, and optional
   `mempool_first_seen_*`; strategy observations must show an initial
   `received` phase before engine handling.
2. Let the latest code run long enough to capture fresh live signals, then build
   a timing table with these stages: mempool detect, signal stored, API visible,
   trader received, risk event recorded, reports/decision finished.
3. If store-to-receive is high, inspect token-server/API polling, query limits,
   and whether the trader is reading an old process or stale endpoint.
4. If receive-to-decision is high, simplify the trader critical path: risk
   signals should not wait behind expensive chain-sim market/report work when a
   fast exit decision is needed.
5. Only after the latest timing table is clear, revisit the mempool simulation
   queue priority/drop behavior. There is a suspected priority-drop inversion,
   but it should not be patched blindly while the live evidence points at
   trader-side blocking.

Keep this bottleneck simple: the desired output is an explainable timing ledger,
not more queue layers. Every proposed fix should reduce one measured interval in
the timing table above.
