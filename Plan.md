# Bogaz

`Plan.md` is the ETH bottleneck ledger. Keep only active bottlenecks here:
what is limiting the pipeline, the evidence, the owning subsystem, and the next
action. Completed work belongs in focused docs or commit history.

## Bogaz Destination Goal

Deploy a real ETH strategy whose policy has been validated by reproducible
recent backtests, live shadow/live-backtest evidence, and audited trade-level
decisions.

The non-capital path is the calibration layer: use it to estimate policy
behavior against current live data, compare it with historical backtests, and
catch signal/simulation drift before real execution.

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

## Current State Snapshot 2026-06-04

The ETH live-capital blocker is evidence quality, not transaction submission.
The pipeline must prove that backtests, live-backtests, real executions, pool
state, and address-level PnL agree on held inventory, scam detection, valuation,
and terminal lifecycle state.

Current canonical scam case: Session
`0xc3a640bd249381f8097f44c1b61c46172068cdff`, drain block `25202411`,
transaction `0xf0e8542a57c488121f18bdad4f148799e59ee21b83caa12bdc5b9aadac43d8ba`.
It proves why holder-balance confiscation must be tracked from actual holder
balance, not only ERC-20 `Transfer` logs or synthetic sell simulation.

Current naming target:

- Lifecycle state: `ClosedZeroValuation` / `closed_zero_valuation`.
- UI label: `Closed at zero value`.
- Old persisted `terminal_zero` should be read as a compatibility alias only.
- Scam mechanism/cause remains separate from lifecycle status.

Active background work:

| Workstream | Owner | Output expected |
| --- | --- | --- |
| Route/liquidity flag migration plus `ClosedZeroValuation` rename | `Poincare` (`019e93aa-1b13-7d61-ba8e-2d20c15d266d`) | Backend/API/frontend/docs rename sweep, compatibility notes, tests/builds. Log: `tmp/agents/route_flag_migration_codex_20260604_192756.log`. |
| 10K backtest/PnL review | `Hegel` (`019e93b1-8b4a-74d2-9f89-66125cfa2eaa`) | Fresh 10K result set plus positive-PnL/scammed-token review artifact and buyer-outcome summaries. Log: `tmp/agents/eth_10k_backtest_review_20260604_173300.log`. |

## Tier One - Backtest Validity

These block trusting strategy PnL, address-level token PnL, and therefore any
new real-capital run.

| Step | Evidence | Next action |
| --- | --- | --- |
| 1. Replay the known scam case | Single-tx replay verified the Session drain tx: no ordinary ERC-20 `Transfer` event for the confiscated balance and one successful internal `transferFrom` from the vault to burn/dead. | Rebuild/run the fixed stack over the Session window and require mined risk evidence, zero/near-zero valuation, and `closed_zero_valuation` at or after block `25202411`. |
| 2. Capture every transfer in a tx so the net-flow reserve/balance calc is complete | Trace-derived ERC-20 transfers — including event-less internal `transferFrom` (e.g. the Session `vault -> 0xdead` confiscation that emits no `Transfer` log) — are decoded from traces and folded into address/reserve balance changes (`e5721043`, `a4b55310`). The calc is event-sourced net-flow of reserves/held balances from captured transfers; a `balanceOf`-vs-ledger reconciliation is NOT the chosen approach (deprecate the existing `reconcile_custody_drain` fallback rather than wire it). | **DONE `deb2efa6`.** Audited: the backtest accounting path already traces every block (`BlockBatchOptions::default().include_traces==true` drives the chain-server range cache-fill + the live processor; the disk cache keys on the trace config so a traceless block is never served as traced) → capture is complete and a confiscation shows up as the tokens leaving the holder, no `balanceOf`. Locked: a regression test pins the trace default, the traceless branch logs a debug line (silent-miss surfaced), invariant README updated to "guaranteed on the accounting path". Residual (consumer-side, eth_token lane): a hard per-tx assert of trace presence at PnL/token-state ingestion. |
| 3. Prove pool-state scam detection drives zero valuation | Pool state covers direct reserve drains, LP-removal drains, external-holder reserve dumps, backdoored pair-balance drains, and holder-balance custody confiscations. Route/liquidity naming cleanup is in flight. | Require Session replay and the 10K review to show the first scam evidence block, pool labels, risk event, trade valuation, address-level valuation, and lifecycle status all agree. |
| 4. Prove address-level token PnL is Tier-One-valid | First version exists: `eth_pnl_store` persists calculation runs, pool states, address PnL, movement rows, actor roles, reconciliation state, and accounting context. Known 50K sample: `token-pnl-semantic-traders-50k-20260602-01`, blocks `25180546..=25230545`, `63,775` address-position rows, `20,146` addresses, `1,538` pools, `715,022` movement rows, `100,891` exact tx hashes. | Assess `closed_zero_valuation` vs closed, `115` open/no-mark rows, `2,240` partial-movement rows, actor roles, and positive-PnL rows on scammed pools. Use the active 10K run as the first case-review queue. |
| 5. Prove real and live-backtest lifecycle parity | Drain-close logic and lifecycle validation exist, but old hold16/hold50 evidence predates the latest scam/valuation path and cannot promote a policy. | Build/deploy the current parity stack and run fresh no-capital hold16 validation from healthy chain-server state. Compare against same-window chain-sim/backtest evidence only after old real positions are reconciled. |
| 6. Make validation gates mandatory | Snapshot checks and lifecycle checks can detect positive value after drain and positions that fail to reach a zero-value terminal close. | Fail promotion and PnL exports when `priced`, `no_mark`, open/closed status, reconciliation status, or value contradict mined scam/custody evidence. |
| 7. Re-run evidence on the fixed stack | Existing promotion evidence is stale relative to the current pool-state/PnL/lifecycle semantics. | After steps 1-6 pass, rerun the exact latest historical window plus fresh live-backtest/shadow with the same sizing, gas model, exit policy, valuation semantics, pool-state labels, and validation gates. |
| Prerequisite. Keep chain-server simulation health green | Chain-server live tracker and `chain_server_live_tx_simulator` were healthy after the `MemoryMax=24G` rebuild/restart, but every run needs a fresh health check. | Before every live-backtest/shadow run, verify tracker status, simulator block/hash, memory cap/current/peak, and address-index failures. |

## Tier One - Backtest Pipeline as Agent Research Infra

Objective: a solid, self-serve backtest pipeline an agent can run end to end and
then assess — the substrate for auto-research in Alpha (Karpathy-style): agents
propose strategies, run backtests, and judge performance from event- and
valuation-level evidence, without a human in the loop and without first risking
real capital. The hard rule: a money-making backtest position must be proven
trustworthy before any real deploy — we never deploy a real trade to discover the
backtest was wrong.

| Item | Why | Next action |
| --- | --- | --- |
| Clean up the backtest pipeline fully | Legacy cruft (removed `snipe-all` deployable identity still the CLI default; no-op compat constants; dead flags) makes the pipeline confusing to drive and hides assumptions. | Remove legacy defaults and compat shims (NO back-compat — old backtest results are disposable); one clear entry point with documented assumptions; review with codex + own subagents. |
| Agent-runnable + assessable interface | Auto-research agents must run a backtest and read back per-position, per-token-event, per-valuation results programmatically, not via ad-hoc SQL. | Define/confirm the run+assess surface: run a strategy over a block window, then enumerate every position, every token event it observed, the decision taken, and the valuation at each step — as a stable, agent-consumable contract. |
| 20K-block backtest + live-backtest suite | Validate the pipeline at scale and against current live data before trusting any money-making result. | Reset the backtest DB (keep only real trades), run a 20K historical backtest and start the live-backtest suite, persist clean result sets. |
| Audit every money-making position | Profitable exits on scammed/illiquid pools are the core failure mode; we must catch synthetic/phantom fills before they reach real capital. | Go over each profitable position: confirm the exit was genuinely realizable (sellable balance, real liquidity, no confiscation/drain at or after the exit), not a synthetic fill. |
| Backtest DB hygiene / disposability | Old backtest results are noise and block clean reruns; only real trades must be preserved for assessment. | Make the backtest result tables safe to wipe/rebuild; drop old backtest result sets, keep real-trade rows (`mode='real'`); document the reset procedure. |

## Tier Two - Evidence/Policy Promotion

These block claiming a production strategy after Backtest Validity code is
healthy.

| Issue | Evidence | Next action |
| --- | --- | --- |
| Hold16 deployed vs hold50 sweep evidence is mixed | The positive hold50 result is from a chain-sim sweep, while the real deployment was hold16. | Compare hold16-to-hold16 on the exact same block window, entry size, gas model, exit rules, and valuation semantics. |
| Latest two-week historical + live shadow evidence is missing | Current live-backtest evidence is stopped or invalidated by old valuation/scam gaps. | After Tier One passes, run the exact latest two-week historical backtest and a fresh live-backtest/shadow run. |
| Trade-level audit is incomplete | Top/worst contributors may include scams that old backtests valued incorrectly. | Audit top 10 and worst 10 by entry reason, skip reason, exit reason, decision block, simulation block, held balance, gas/slippage, and PnL snapshot. |
| Risk Atlas policy contract is still implicit | Protocol coverage, labels, thresholds, exclusion reasons, and routeability are spread across code/docs. | Create one versioned Risk Atlas policy contract and attach its version to Alpha backtest/live metadata. |
| Full-history PnL DB not yet built | Address-level PnL has only run over 50K samples (`token-pnl-semantic-traders-50k-20260602-01`, `token-pnl-tier1-50k-20260603-195033`, and the movement-retention A/B). | Deferred and gated: do not start full history until small-run results are reviewed and there is explicit go-ahead. Then run a historical sweep with movement retention, stable schema/API, and Tier-One accounting fixes applied. |

## Tier Three - Operator/Observability

These should not block code correctness, but they block confident operations.

| Issue | Evidence | Next action |
| --- | --- | --- |
| Live parameter surface is missing | Bankroll, buy size, signer/vault, gas-rank, route policy, kill switch, and freshness gates are not visible in one operator page. | Build the Asena live trading parameter page from Alpha/ETH tx executor/signer status. |
| Mempool timing attribution is incomplete | Signals are produced, but the rebuilt process set does not yet have a complete first-seen -> stored -> API visible -> trader received -> decision/report timing ledger. | Collect fresh timing pairs only after chain-server live state is healthy. |
| Build/deploy state is easy to misread | Working-tree fixes, committed UI fixes, and running service state are easy to conflate. | Record commit hash, binary build time, and process start time on every run/status page. |
| Block-hash pinning needs audit enforcement | ETH has block-frame and simulator hash plumbing, but promotion dashboards and audit exports still need consistent block number+hash visibility. | Surface and validate number+hash on Alpha reports, Asena detail pages, and promotion checks. |
| Terminal metadata regression guard is missing | Old terminal `backtest_result_sets` and `trader_runs` rows were backfilled, but future regressions need a gate. | Add validation that terminal rows cannot carry live metadata such as `live_status=live` or `trading_enabled=true`. |
| Module/file-size cleanup remains | Several Alpha files are still large enough to slow review. | Split only where it reduces current work risk: real execution wiring, simulated execution adapters, receipt reconciliation, and engine tests. |
| Live PnL not integrated into the live pipeline | Address-level token PnL (`eth_token::pnl` / `eth_pnl_store`, schema `token_pnl`) + the pool realizability/scam-state flags run only as a historical/backtest analytic; live real and live-backtest runs do not compute or persist per-position/per-address PnL, valuation, or realizability in real time. | Integrate the live PnL accounting into the live pipeline so live runs produce the same per-position/per-address realized/unrealized PnL, `ClosedZeroValuation` terminal valuation, and per-scam-type realizability the backtest produces — persisted and surfaced for operator review alongside the live trade pages (links to the run-linkage/`lineage_id` work). |

## Scammer Analytics - Tier-Two Consumer

Scammer Analytics consumes the Tier-One scam-detection and address-level PnL
work. Its objective is actionable cleanup: identify operators/funding roots,
follow stolen value to off-ramp, explain the scam mechanics, and produce packets
usable for exchange contact or law-enforcement escalation.

Spec: `risk_atlas/scammer_analytics/DESIGN.md`.

Active model decision: group cases by the funding tree, not by a disposable
operator address. Observed pattern is funding source -> fresh operator -> one
scam -> cash-out/forward -> abandon.

| Gap | Next action |
| --- | --- |
| Funder-tree rollup | Regroup `scammers[]` by funding source and link each fresh operator address through nearest pre-deploy funding. |
| Cluster/funder attribution | Link cases by shared funder, shared cash-out deposit address, shared forwarder/sink, and identical token bytecode; emit confidence-scored clusters. |
| Scam mechanics on page | Put `scam_mechanics.json` on the scammer page: pump curve, extraction, LP-add vs rug-removal, and lifecycle timeline. |
| High-impact showcase case | Pick a real pump-and-dump with large `inflation_x`, many victims, and CEX cash-out; generate the full artifact set. |
| Actionable output | Export LE packets and per-exchange contact blocks with exchange, deposit addresses, amount, and preservation/freeze request context. |
| Address first-degree screen | Build a per-address page with scam-trade ratio, scam pools, roles/mechanisms, and cases the address appears in. |
| Live detection fallback | Wire trace-independent `balanceOf` vs ledger reconciliation into live/backtest detection so drain evidence is not trace-dependent. |

## Pool-State / PnL Naming Contract

Keep one flag or label per column. Do not merge unrelated concepts into a
single liquidity/stage label.

Route simulation names:

- `buy_simulation_succeeded`
- `sell_simulation_succeeded`
- `buy_simulation_usable_with_current_liquidity`
- `sell_simulation_usable_with_current_liquidity`
- `sell_output_economic`

Liquidity/risk names:

- Remove `legacy_liquidity_removal_flag`.
- Remove `liquidity:legacy_terminal_flag`.
- Use specific mechanism labels for cause, such as reserve drain, LP removal,
  pair-balance backdoor drain, holder-balance confiscation, custody drain, dust,
  or no route.

Lifecycle/valuation names:

- Use `closed_zero_valuation` when the position/address-pool trade is final and
  valued at zero.
- Keep cause separate in pool labels, risk events, or a reason field such as
  `zero_valuation_reason`.
- Preserve a read alias for old `terminal_zero` persisted rows until existing
  runs are migrated or deprecated.

## Detailed Active Bottlenecks

### Held-Balance / Custody Confiscation Accounting

Required behavior:

1. `pool_viability_simulation` may use synthetic balances for route/tax checks.
2. `position_valuation` must use authoritative current held balance and must
   not inject inventory.
3. Held-token balance drains, holder-to-burn transfers, balance-changing control
   calls, sellability-state updates, and pool reserve updates must all trigger
   valuation.
4. Mined holder-balance confiscation must emit risk evidence, force zero
   valuation, and close the affected position/trade as `closed_zero_valuation`.
5. Validation must fail if value remains positive after mined balance-drain
   evidence, if no zero snapshot exists, or if movement/valuation evidence is
   materially incomplete.

Open gap: generic partial-balance divergence is still not proven for every
real/live-backtest path. The explicit drain path is covered by risk handling,
but ordinary or partial held-balance changes still need authoritative balance
reads/reconciliation before real-capital PnL can be trusted broadly.

### Real vs Live-Backtest Execution Parity

Problem: the first real hold16 run showed stuck `buy_confirmed` positions,
failed sells, and missing/zero valuation while chain-sim wrote per-block values.
The canonical Session case showed that synthetic valuation can stay positive
after the actual vault balance is confiscated.

Next actions:

1. Build/deploy the current real valuation delegate, per-block valuation
   trigger, stale sell-gate removal, and drain-close lifecycle logic.
2. Run fresh no-capital hold16 validation from healthy chain-server state.
3. Require per-block `current_value_eth` snapshots, exact sim sell planning, and
   zero-value close for drained inventory.
4. Reconcile old stuck real positions before any new real-capital run.

### Tail-Entry Production Parity

Tail-entry hardening is useful, but it is not promotion-ready until the same
block dependency path is measured end to end.

| Blocker | Required evidence |
| --- | --- |
| Non-vacuous coverage checks | Count trading-enabled signals, mempool-entry evidence, exact-vault eligible signals, tail-entry intents, and submitted/confirmed/deferred/failed/cancelled outcomes. |
| Config-bound exact-vault evidence | Prove configured production vault, chain id, buy amount, and route version, not just a generic route shape. |
| Real receipt ordering preservation | Preserve `tail_after_tx_hash`, dependency fee, selected fee, and ordering intent in final mined evidence. |
| Dependency-relative gas policy | Compare selected tail fee against the enabling transaction and validate behind-dependency ordering. |
| Same-block overlay proof | Simulate dependency transaction plus exact vault calldata in one overlay state, or label the run post-mine `N+1` only. |

### Module-Level Backlog

The Tier One/Tier Two tables are the authoritative promotion gates. This backlog
keeps module ownership for active cleanup and parity threads.

| Issue | Owner | Next action |
| --- | --- | --- |
| Chain/source/simulator parity must stay measurable | `risk_atlas`, `eth_token`, `tx_processor::trade_simulation`, `alpha/backtest` | Work the ranked promotion queue in `risk_atlas/investigations/README.md`; reproduce old issues on current code before using affected cohorts for policy or PnL claims. |
| Backtests must cover the Risk Atlas protocol surface | `alpha/backtest`, `alpha/engine`, `risk_atlas`, `tx_processor::trade_simulation`, `eth_token` | Make every protocol either executable with correct route simulation/valuation or explicitly excluded with reason; report per-protocol PnL and skipped-pool counts. |
| Risk Atlas policy contract is missing | `risk_atlas`, `alpha/strategies`, `alpha/lab`, `interface/new_asena` | Version eligible cohorts, protocol/denom routeability, feature names, target labels, thresholds, exclusion reasons, calibration windows, and decision-question outputs. |
| Direct EOA allowance policy is unresolved | `alpha/live/trading`, `tx_simulator::tx_builders`, `solidity/baygus-executor` | Prefer vault emergency-sell mode for scam exits; only enable direct EOA sells after documenting pre-approval, permit/multicall, or two-transaction approval behavior. |
| Real planner shadowing is missing | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | Run the real planner with ETH tx executor `dry_run` and compare every planned priority exit against live chain-sim outcomes. |
| Mempool signal latency attribution is incomplete | `mempool_processor`, `eth_chain_server`, `alpha/engine` | Build a fresh timing table from mempool detect to stored signal, API visibility, trader observation, decision, and report completion. |
| Live tracker protocol retention metrics are ambiguous | `eth_chain_server`, `alpha/live/feed`, `eth_token` | Separate discovered/retained/dropped pool counters by protocol and drop reason so V2/V3/V4 coverage claims are auditable. |
