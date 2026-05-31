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

## Current State Snapshot 2026-05-31

Do not mix the live-capital run and the live-backtest sweep:

- The deployed real-capital strategy was
  `alpha11-univ2-lp30-pool-update-block-hold16`.
- `hold50` came from the live-backtest sweep result set
  `live-alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-block-frame-20260529-152040017864493Z`.
  It is one strategy member inside the sweep, not the live-capital strategy.
- The hold50 row showing `21` positions and `+0.4323682439662406 ETH` is
  chain-sim evidence only. It cannot be used as deployed-real evidence, and it
  cannot be trusted for promotion until the held-balance/valuation scam gap is
  closed and re-run.
- The real hold16 result set
  `live-alpha11-univ2-lp30-pool-update-block-hold16-live-real-broadcast-20260529-151921026902336Z`
  is stopped at `2026-05-29 17:10:05.783021+00`, with `16` positions,
  `11` open, `1` closed, `8` failed, and `-0.043955014340477175 ETH` total
  PnL. The old persisted metadata bug has been backfilled on `2026-05-31`;
  the result set and matching trader run now both carry
  `live_status=stopped` and `trading_enabled=false`.
- The real-executions API/page still shows stopped hold16 runs as `open`
  because `run_status_label()` derives display status from open-position counts
  after reading `trader_runs.status`. Result-set detail says `stopped`; the
  real-live page says `open`. This is a frontend/API contract bug: expose
  separate `lifecycle_status` (`running/stopped/stale/failed`) and
  `position_status` (`open/closed/failed`) instead of overloading `status`.
- Current processes on `40020`: `eth_chain_server`, `mempool_signal_detector`,
  and `asena-static` are running. No live trader or live backtester process is
  running.
- The live token tracker inside chain-server is currently `failed` with
  `last_error="environment map size limit reached"`, `live_blocks_processed=0`,
  and `/api/v1/eth/live-tx-simulator/status` returns `503`. This means there is
  no healthy current live simulation surface to use for new evidence until the
  storage failure path is fixed and the chain-server live state is rebuilt.
- `reth_index/mdbx.dat` is 160 GiB on disk, but diagnostics show only about
  6.04 GiB of live MDBX pages: `address_to_blocks` is about 5.06 GiB with
  `222,333,079` rows and `mempool_tx_arrival_times` is about 0.82 GiB with
  `22,381,852` rows. The scary file size is MDBX high-water allocation/free
  pages, not 160 GiB of current address-index payload. The size driver that
  remains is broad address extraction, not duplicate `(address, block)` rows.
  The map-size fix should be bounded: a new index gets a 64 GiB ceiling; this
  existing 160 GiB high-water file gets 32 GiB of headroom, so 192 GiB, not
  1 TiB.

Latest relevant commits:

- `31ed35f5` (`blockchains/eth`): future stopped/stale trader runs write
  terminal metadata (`live_status=<terminal>`, `trading_enabled=false`).
- `9b12556` (`new_asena`): committed a duration cap for stopped live rows, but
  it has not been rebuilt/restarted on the running `asena-static`, and it still
  needs the lifecycle/position status split above to cover stopped runs that
  still have open positions.
- `e5721043`, `1fd16868`, `23a375e1`, and adjacent ETH commits improved
  transfer accounting, transaction inspection, and live processing ownership.
  They do not by themselves validate live-capital strategy PnL.

## Current Three-Tier Limiting Factors 2026-05-31

### Tier One - Capital/Accounting Correctness

These block any new real-capital run.

| Issue | Evidence | Next action |
| --- | --- | --- |
| Real and live-backtest position lifecycle are not equivalent | Real hold16 bought positions, then showed missing/zero valuation, stuck open positions, and failed sells while chain-sim marked positions every block. | Finish, commit, build, deploy, and verify real-mode valuation delegation plus exact-sim sell planning. Then re-run a small hold16 validation only after existing rows are reconciled. |
| Backtest PnL can be inflated by held-balance scams | Session-style `holder_balance_backdoor_drain` can remove vault inventory without a pool update. Previous chain-sim valuation could keep selling synthetic entry inventory. | Re-run backtests/live-backtests only after holder-balance drain detection, token-affecting valuation triggers, and zero-value validation checks are committed, built, and deployed. |
| Chain-server live tracker uptime is broken | `eth_chain_server` is still running, but live tracker failed during warmup at block `25203730` after `3072/7000` warmup blocks. The optional `reth_index` address-participation MDBX write hit `environment map size limit reached`; `/api/v1/eth/live-tx-simulator/status` is unavailable because no live block state was published. RethIndex stats show the 160 GiB file is mostly MDBX high-water/free pages, while live payload is about 6.04 GiB. | Make live tracker resilient to optional replay/index storage failure, configure explicit `reth_index` MDBX geometry, add stats/diagnostics, rebuild/restart chain-server live state, then verify live tracker and live tx simulator expose the same healthy block/hash. Separately narrow the address-participation filter and compact/rebuild the MDBX env during maintenance. |
| Real/live frontend state contract is inconsistent | Result-set API says stopped; real-executions API/page says `open` for stopped runs with open positions. | Split `lifecycle_status` from `position_status`, backfill stale terminal metadata on old rows, and make duration use lifecycle stop time. |

### Tier Two - Evidence/Policy Promotion

These block claiming a production strategy even after Tier One code is healthy.

| Issue | Evidence | Next action |
| --- | --- | --- |
| Hold16 deployed vs hold50 sweep evidence is mixed | The positive hold50 result is from a chain-sim sweep, while the real deployment was hold16. | Compare hold16-to-hold16 on the exact same block window, with the same entry size, gas model, exit rules, and valuation semantics. |
| Latest two-week historical + live shadow evidence is missing | The current live-backtest runs are stopped or invalidated by the valuation/scam gap and current live-state failure. | After Tier One fixes, run the exact latest two-week historical backtest and a fresh live-backtest/shadow run from healthy chain-server state. |
| Trade-level audit is incomplete | Top/worst contributors may include scams that the old backtest valued incorrectly. | Audit top 10 and worst 10 by entry reason, skip reason, exit reason, decision block, simulation block, held balance, gas/slippage, and PnL snapshot. |
| Risk Atlas policy contract is still implicit | Protocol coverage, labels, thresholds, exclusion reasons, and routeability are spread across code/docs. | Create one versioned Risk Atlas policy contract and attach its version to Alpha backtest/live metadata. |

### Tier Three - Operator/Observability

These should not block code correctness, but they block confident operations.

| Issue | Evidence | Next action |
| --- | --- | --- |
| Live parameter surface is missing | Bankroll, buy size, signer/vault, gas-rank, route policy, kill switch, and freshness gates are not visible in one operator page. | Build the Asena live trading parameter page from Alpha/Kartal/signer status. |
| Mempool timing attribution is incomplete | Signals are being produced, but the latest rebuilt process set has not produced a complete timing ledger from first seen -> stored -> API visible -> trader received -> decision/report. | Collect fresh timing pairs only after chain-server live state is healthy. |
| Build/deploy state is easy to misread | Working-tree fixes exist, committed UI fixes exist, but the running `asena-static` and chain-server state do not yet prove those fixes are live. | Record commit hash, binary build time, and process start time on every run/status page. |
| Old terminal metadata pollutes dashboards | Fixed on `2026-05-31`: terminal `backtest_result_sets` and `trader_runs` rows were backfilled so old stopped/completed/stale rows no longer carry `metadata.live_status=live` or `trading_enabled=true`. | Add validation that terminal rows cannot carry live metadata, so future regressions fail before reaching dashboards. |

## Tier One - Real/Backtest Execution Parity 2026-05-29

Status: active P0 blocker for real capital. Partially fixed in the working tree,
but not committed/built/deployed as evidence. Discovered from the first real
broadcast run
`alpha11-univ2-lp30-pool-update-block-hold16-live-real-broadcast-20260529-151921026902336Z`.

Problem: the real (Kartal) execution path manages an open position's lifecycle
differently from the live-backtest (chain-sim) path. The strategy is the same,
but after a confirmed buy the real path neither values nor reliably exits the
position. Concrete evidence from the run above: the Session token
(`0xc3a640...68cdff`) pumped before its holder-balance backdoor drained the
vault. The live token tracker initially showed the vault holding ~9.87M tokens
at ~+0.068 ETH unrealized, but the trade stayed `buy_confirmed`, never sold
automatically, and showed "Current Value -" with zero valuation snapshots. Six
other positions sat `sell_failed`. Real run wrote 7 position snapshots (all
`current_value_eth=0`); the parallel backtest run wrote 2924 per-block values,
but the Session case proves those values can be synthetic and wrong unless they
are tied to real held balance.

Verified divergences (each is a parity gap real must close against chain-sim):

1. No per-block position valuation in real mode.
   - Chain-sim: `LiveChainSimExecutionAdapter::simulate_position_value`
     (`execution/simulated/mod.rs:407`) builds a sell intent and calls
     chain-server alpha-order simulation to mark the held position each pool
     update; `valuation/snapshot_flow.rs` persists `position_snapshots` with
     `current_value_eth`.
   - Baseline real path: `TxExecutorAdapter` only implemented `execute`; it
     inherited the default `simulate_position_value` that returns `Ok(None)`.
     So open real positions were never valued. The dashboard "Current Value"
     was blank/zero and the strategy had no mark-to-market.
   - Working tree state: `TxExecutorAdapter` now has an optional
     `LiveChainSimExecutionAdapter` valuation delegate and
     `real_execution/mod.rs` wires it from the same chain-server state. This is
     the right direction, but it is not yet committed/deployed evidence.

2. Sell exits blocked by stale `can_sell` snapshot gate.
   - `validate_sell_intent` (`alpha/live/trading/src/planner/route_builder.rs`)
     rejected sells when the snapshot said `can_sell=false`, before the
     authoritative exact pre-submit simulation. Fixed in working tree (gate
     removed; exact sim is authoritative) but NOT yet built/deployed. Chain-sim
     never hit this because it sells via exact simulation.

3. Pool-update cadence divergence starves real of exit/valuation triggers.
   - Real uses `LIVE_REAL_FRAME_POLL_TIMEOUT_MS=1` so `block_frame_pool_count`
     and `market_events` are ~0; held tokens that fall out of the tracker's
     retained set stop producing `PoolUpdated` events. In chain-sim those events
     drive both valuation and pool-driven exits. Real currently relies only on
     the per-block `BlockCompleted` position monitor plus mempool risk signals.
     The position monitor does fire (552 `pool_update/checked` observations in
     the run), so max-hold can be evaluated, but without valuation or a working
     sell the exit does not complete.

Not a gap (intentional): settlement evidence shape differs by design — real
reconciles mined receipts (`receipt_reconciliation/`), chain-sim re-simulates at
the settlement block (`execution_lifecycle/chain_sim_settlement.rs`). Keep this.

Next action, in order:

1. Commit, build, and deploy the real valuation delegate and verify real open
   positions get per-block `current_value_eth` snapshots.
2. Build and deploy the `can_sell` gate removal so real exits reach the exact
   simulation instead of dying on a stale flag.
3. Confirm max-hold and risk exits fire for real positions once valuation and
   the sell path work; re-audit against the same-window backtest trades for the
   same tokens (e.g. `0xed5475` the backtest sold +20..+51%).
4. Recover/settle the currently stuck open positions before they round-trip.

## Tier One - Position Valuation Must Use Real Held Balance 2026-05-29

Status: active P0 blocker for trusting live-backtest or real-capital PnL. The
holder-balance drain special case is partially implemented in the working tree,
but the evidence must be regenerated from a rebuilt chain-server/Alpha stack.
Discovered from the Session case:
`risk_atlas/scammer_analytics/cases/eth_0x02467dd0_session_vault_balance_drain_25202411/`.

Scam mechanism label:

- machine label: `holder_balance_backdoor_drain`
- human label: `Backdoored Holder-Balance Drain`
- event/detail label: `control_transfer_from_holder_to_burn_without_allowance`

Reason for a new label: existing Risk Atlas vocabulary already uses
`pair_balance_backdoor_drain` / `Backdoored Pair-Balance Drain` for creator or
control logic that drains the pool/pair balance. Session is different: the
creator/control path drained a holder's balance, specifically our trading vault,
without ordinary allowance evidence. The control tx called
`transferFrom(vault, dead, amount)` while vault allowance to the control
contract was zero and emitted no normal `Transfer` log for the burned amount.

Issue 1: live-backtest valuation is not triggered on every token-affecting
block.

- Intended behavior: after every block that affects a held token, pool, vault
  balance, or sellability state, Alpha should revalue open positions from the
  authoritative current held balance.
- Current behavior: `snapshot_open_positions_for_pool()` only runs on
  `MarketEvent::PoolUpdated`. A holder-balance drain can be token/control
  activity with no pool reserve update, so block `25202411` did not create a
  valuation snapshot for the Session position.
- Consequence: the backtest skipped the exact block where the vault balance
  dropped from `9,871,580.343970612` Session to `93` Session.

Issue 2: position valuation simulates a synthetic sell of the original entry
amount instead of selling the position's current held balance.

- Current valuation builds a sell intent from
  `position.entry_token_raw_amount`.
- The router sell simulator then calls `prepare_seller_token_balance(...)`,
  which injects that requested amount into the simulated seller before selling.
- That is correct for generic pool viability or tax probing, but wrong for
  position PnL. A position valuation must not mint/inject inventory. It must
  query or track current `balanceOf(holder)` at the valuation block and only
  value that balance.
- Consequence: after the Session drain, live-backtest snapshots kept marking a
  profitable synthetic inventory even though the actual vault inventory had
  already been taken.

Important nuance: the token/PnL analytics appeared closer to reality because
the token-network/PnL view is address-balance based. It saw that the vault's
actual Session balance collapsed and that the manual sell could only sell `93`
tokens. That does not make Alpha's trade valuation correct; it means the
address-level PnL layer is the evidence source we should use to validate and
repair Alpha's position valuation.

Required fix:

1. Split valuation semantics explicitly:
   - `pool_viability_simulation`: may use synthetic balances for can-buy,
     can-sell, and tax checks.
   - `position_valuation`: must use authoritative current held balance and
     must not inject synthetic token inventory.
2. Add held-balance state for Alpha positions:
   - Real mode: query/simulate `balanceOf(vault)` at the exact block before
     valuation and exits.
   - Live-backtest mode: maintain a portfolio overlay or authoritative
     simulated holder balance that can be reduced by token-control drains and
     other holder-affecting token activity.
3. Trigger valuation from token-affecting events, not only pool reserve updates.
   A held-token balance drain, holder-to-burn transfer, balance-changing control
   call, or sellability-state update must cause an open-position snapshot even
   when the pool reserves did not change.
4. Emit a Tier One risk event for
   `holder_balance_backdoor_drain` / `Backdoored Holder-Balance Drain`, and make
   the strategy exit/zero-value the position immediately when the current held
   balance is materially below expected inventory.
5. Add backtest validation:
   - fail if an open position has post-entry holder-balance-drain evidence but
     no matching zero/near-zero valuation snapshot;
   - fail if a position valuation report used a synthetic injected balance
     instead of current held balance;
   - fail if latest trade value is positive after the held balance is zero or
     dust.

Implementation status 2026-05-31 (working tree, pending commit/build/deploy):

- DONE (3) Token-affecting valuation trigger: `value_open_positions_for_tokens`
  in `alpha/engine/src/valuation/snapshot_flow.rs`; `updated_tokens` threaded
  through `LiveInputBatch` and called for ChainSim in `live_trader/mod.rs`.
- DONE (4) Detection + risk + exit: `holder_balance_backdoor_drain` detected in
  `eth_token` (`pools/scam_mechanism.rs` label;
  `erc20/token/activity.rs::mark_holder_balance_backdoor_drains_from_processed_transaction`
  reusing control-transferFrom-without-log evidence). The drain marks the pool
  scam, sets `reserve_tracker.is_scam`, and bumps `latest_block_number` so the
  scammed pool enters the block frame at the drain block. It surfaces as a
  `LiquidityRemoval` critical risk (tagged `risk_kind=holder_balance_backdoor_drain`
  in `mined_pool_risks.rs`), which marks the position drained → zero-value
  snapshot → existing `exit.liquidity_removal` rule fires. This is the
  zero-on-drain held-balance model for the hypothetical backtest position.
- DONE (5) Three validation checks in
  `alpha/lab/src/strategy_validation/checks/snapshots.rs`:
  `open_position_balance_drain_has_zero_snapshot`,
  `no_positive_open_snapshot_after_drain`, `no_positive_value_after_zero_balance`.
- DEFERRED (1/2 full held-balance semantics) Authoritative on-chain
  `balanceOf(vault)` read for generic REAL/live-backtest position valuation
  (chain-server sell-sim `UseOnchain` mode replacing injected balances). The
  drain case is zeroed by explicit risk handling, but non-drain partial-balance
  divergence is still not solved. Track before relying on real-capital PnL for
  non-drain partial-balance cases.

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

Current deployed evidence at the time of the 2026-05-28 fix:

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

2026-05-31 caveat:

- The simulator ownership design is still the right architecture, but the
  current running chain-server live state is not healthy evidence: live token
  tracker status is `failed` with `environment map size limit reached`, and the
  live tx simulator status endpoint returns `503`.
- Before using any new live-backtest result, first restore chain-server live
  state, verify the live token tracker and live tx simulator expose the same
  current block/hash, then start a fresh live-backtest from that healthy state.

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

As of 2026-05-31, the live-capital bottleneck is not raw transaction submission.
It is the Tier One accounting/parity set above: real valuation/exits, held
balance scam handling, current chain-server live-state health, and frontend/API
lifecycle truth. The intended real flow is documented in
`alpha/live/trading/README.md` and
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
- The pipeline cannot be promoted until real and live-backtest accounting agree
  on held inventory, valuation, terminal state, and sell outcomes.
- Before promoting a policy, the evidence base must expand from the current
  V2-centered executable backtests to the full protocol surface Risk Atlas sees,
  or explicitly account for every excluded protocol/pool as not routeable.
- Risk Atlas must become a formal policy contract: versioned cohorts, features,
  thresholds, targets, exclusions, and decision questions that Alpha and Asena
  can both cite.
- Real-live validation can public-broadcast through Kartal only when the live
  service is started with explicit public-broadcast validation enabled.
- Broader production remains blocked by evidence quality, healthy block-pinned
  Alpha inputs, and operator visibility, not by missing real-trade tracking or
  missing gas outcome fields.
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

## Module-Level Backlog

The current three-tier limiting factors are the authoritative promotion gates.
This table keeps module ownership for the broader backlog so we do not lose the
older cleanup threads.

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
| 1 | **Private relay/builder execution** | `tx_executor`, `kartal` | Flashbots/relay/bundle submission was attempted and removed. `TxSubmissionRoute` now has exactly two variants: `PublicRpcBroadcast` (default) and `PublicMempoolTail` (tail after enabling tx using its priority fee minus `ALPHA_LIVE_TAIL_ENTRY_PRIORITY_UNDERCUT_WEI=500 wei`). No relay URL, no `sign_flashbots_auth`, no bundle path exists in the current codebase. | If private relay submission is needed in future, add a new `TxSubmissionRoute` variant and a new Kartal wire protocol version. Do not re-enable Flashbots in the current path. |
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
