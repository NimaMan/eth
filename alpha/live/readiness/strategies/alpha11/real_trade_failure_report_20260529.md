# Alpha11 Real Trade Failure Report — 2026-05-29

Every distinct failure observed across the real (ETH tx executor broadcast) runs of
`alpha11-univ2-lp30-pool-update-block-hold16`, with root cause, status, and the
action required for each. Data source: `alpha_trading.execution_reports` for all
`%live-real%` runs, cross-checked against on-chain receipts.

Outcome tally across all real runs:

| Side | Outcome | Class | Count |
| --- | --- | --- | --- |
| buy | submitted | (interim) | 30 |
| buy | confirmed | success | 20 |
| buy | cancelled | pre_submit_would_revert | 9 |
| buy | deferred | pre_submit_state_not_ready | 5 |
| buy | deferred | pre_submit_state_stale | 1 |
| buy | failed | eth_tx_executor_backrun_not_found | 2 |
| buy | failed | eth_tx_executor_flashbots_sig | 2 |
| buy | failed | eth_tx_executor_min_priority_fee | 1 |
| buy | failed | eth_tx_executor_signer_flashbots_variant | 1 |
| buy | failed | eth_tx_executor_wire_protocol | 1 |
| buy | failed | onchain_revert_status0 | 1 |
| sell | submitted | (interim) | 9 |
| sell | confirmed | success | 1 |
| sell | failed | sell_can_sell_gate | 7 |
| sell | failed | onchain_revert_status0 | 1 |
| sell | cancelled | provisional_sim_would_revert | 5 |
| sell | cancelled | gas_rank_exceeds_value_cap | 1 |

Legend for Status: **Correct** = working as designed, no fix needed.
**Fixed-deployed** = fix already in a deployed binary. **Fixed-pending** = fix in
the working tree, not yet built/deployed. **Active** = no fix yet.

---

## BUY failures

### B1. Pre-submit simulation would revert — `buy_cancelled` (9)

- Evidence: `exact live transaction simulation would revert at block N`.
- What happened: the exact pre-submit calldata simulation against chain-server
  live state predicted the buy would revert, so the order was cancelled before
  signing. No tx, no gas, no capital spent.
- Root cause: not a bug — these are pools the exact simulator correctly refused
  (anti-snipe windows, trading-not-enabled-yet, transient state).
- Status: **Correct.**
- Action: none. Monitor the rate; a high cancel rate is an entry-signal quality
  signal, not an execution bug.

### B2. Pre-submit simulation state not ready / stale — `buy_deferred` (5 + 1)

- Evidence: `pre-submit simulation state not ready: selected_block=N
  required_block=N+1 ...`; `exact V2 vault simulation state is stale:
  selected_block=N current_block=N+2 max_state_lag_blocks=2`.
- What happened: the decision targeted block N+1 but the chain-server simulator
  had only committed block N at the moment of the request. The order is deferred
  and retried; the entry is usually missed by one block.
- Root cause: 1-block lag between the decision block (mempool signal
  `detected_at_head_block` = N+1) and the simulator's committed state (N). Old
  form used `LocalHistoricalContext` (fixed; chain-server now owns state); the
  remaining form is the inherent poll/commit race.
- Status: **Active** (already tracked as a tier-3 follow-up).
- Action: tighten the decision→simulation handoff so the request waits for the
  required block to commit (bounded wait) instead of deferring, or pin the
  decision to the committed block. Re-measure deferral rate after.

### B3. ETH tx executor Flashbots path failures — `buy_failed` (2 + 2 + 1)

- Evidence: `backrun not found`; `Flashbots relay returned HTTP 403: invalid
  flashbots signature`; `signer error: unknown variant 'sign_flashbots_auth'`.
- What happened: orders routed through a Flashbots/backrun/bundle path that the
  relay and signer were not configured for; all rejected before mining.
- Root cause: an abandoned private-relay execution path.
- Status: **Fixed-deployed.** Flashbots/relay/bundle submission was removed.
  `TxSubmissionRoute` is now only `PublicRpcBroadcast` and `PublicMempoolTail`;
  no `sign_flashbots_auth`, no relay, no bundle path exists in the codebase.
- Action: none. Do not re-introduce Flashbots in the current `eth_unsigned_tx`
  path; if private relay is ever needed, add a new route + wire version.

### B4. Priority fee below ETH tx executor minimum — `buy_failed` (1)

- Evidence: `max_priority_fee_per_gas 549862982 is below configured minimum
  1000000000`.
- What happened: Alpha selected ~0.55 gwei priority; the old ETH tx executor build
  enforced a 1 gwei minimum and rejected it.
- Root cause: two minimums (Alpha gas policy vs ETH tx executor) were not synchronized,
  and ETH tx executor should not own a floor at all.
- Status: **Fixed-deployed.** ETH tx executor no longer enforces a min priority fee
  (only max caps); Alpha owns the floor. `ALPHA_LIVE_GAS_MIN_PRIORITY_FEE_GWEI`
  was removed from config and internalized as a `1` gwei default in the gas
  policy.
- Action: none.

### B5. Wrong ETH tx executor wire protocol — `buy_failed` (1)

- Evidence: `ETH tx policy rejected ... metadata wire_protocol must be
  eth_direct_raw_v1`.
- What happened: the request carried the wrong `wire_protocol` metadata variant.
- Root cause: a transient mismatch in the first broadcast run
  (`...084916Z`); later runs do not show it.
- Status: **Fixed-deployed** (only appears in the earliest run).
- Action: none beyond confirming the request builder always emits
  `eth_direct_raw_v1`.

### B6. Buy mined but reverted on-chain — `buy_failed` (1) — CAPITAL/GAS LOSS

- Evidence: tx `0xfdcc82a2b5d2ae864feb96a6d123daa5b141daeb4d658f24ab158298ec0d7127`,
  decision block 25202277/25202278, mined block 25202342, `status=0x0`,
  gas_used 199,477, to vault `0x28474c...`.
- What happened: the buy passed the exact pre-submit simulation, was broadcast,
  mined, and then reverted. Real ETH was spent on gas for a failed buy.
- Root cause: state moved between the simulation block and the mining block —
  the pool/token state at mine time no longer matched the simulated state
  (front-run, anti-snipe activation, reserve move). Pre-submit sim freshness is
  necessary but not sufficient when there is a multi-block gap to inclusion.
- Status: **Active.**
- Action: (a) minimize sim→broadcast→inclusion gap; (b) carry a strict
  min-output / slippage bound in the buy calldata so a moved pool reverts
  cheaply or fills within tolerance rather than reverting after full gas; (c)
  treat repeated same-token sim-pass/mine-revert as an entry-exclusion signal.

---

## SELL failures

### S1. Sell blocked by stale `can_sell` snapshot gate — `sell_failed` (7) — MISSED EXITS

- Evidence: `priority sell planner failed: invalid planner input: pool snapshot
  says can_sell=false`. No tx submitted; tokens left stuck in the vault.
- What happened: `validate_sell_intent` rejected the exit on the snapshot's
  stale `can_sell=false` flag before reaching the authoritative exact
  simulation. Proven false-negative: the same-window backtest sold one of these
  tokens (`0xed5475`) eight times for +20% to +51%.
- Root cause: the real sell path gated on a block-lagged heuristic snapshot
  (pools can even be dropped from the tracker before the exit fires) instead of
  the exact pre-submit simulation.
- Status: **Fixed-pending.** Gate removed in `route_builder.rs`
  (`validate_sell_intent`); the exact pre-submit simulation is now authoritative.
  Not yet built/deployed.
- Action: build and deploy. After deploy, confirm these stuck positions exit.

### S2. Provisional sell simulation would revert — `sell_cancelled` (5)

- Evidence: `priority sell planner rejected before broadcast: pre-submit
  simulation failed: cannot derive min-output from failed provisional
  simulation: pre-submit simulation indicates the sell would revert`.
- What happened: the exact provisional sell simulation itself reverted, so no
  min-output could be derived and the sell was cancelled before broadcast. No
  capital spent.
- Root cause: genuinely non-sellable at that block (honeypot mechanics or
  drained pool). This is the exact-sim protection working — distinct from S1,
  which never reached the simulation.
- Status: **Correct.**
- Action: none for execution. These should feed the entry filter (a token that
  buys but cannot sell at exit time is an entry-quality problem). Note: with S1
  fixed, all exits route through this authoritative path, so genuine
  non-sellables cancel here cleanly.

### S3. Gas-rank exceeds value cap — `sell_cancelled` (1)

- Evidence: `priority sell tx prep rejected: gas_rank_exceeds_value_cap ...`.
- What happened: the cheapest viable priority fee would have cost more than the
  ETH protected by exiting, so the sell was cancelled (hold instead).
- Root cause: not a bug — the value-cap protection refusing to sell at a net
  loss to gas.
- Status: **Correct.**
- Action: none.

### S4. Sell mined but reverted on-chain — `sell_failed` (1) — GAS LOSS

- Evidence: tx `0x8b90b5a48e6bd158723406844c759bc56c9d2b65464b814b0e6729262c54f53e`,
  mined block 25158999 `status=0x0`. The LP-removal tx (`0x0ea03ef6...`, 3.0
  gwei priority, tx index 19) landed before our sell (2.1456 gwei, index 24) and
  drained the pool; our emergency sell then reverted.
- Root cause: our `mempool_race` exit bid a fixed gas-rank percentile (p95)
  instead of bidding relative to the enabling/removal transaction, so we were
  outbid and landed after the drain.
- Status: **Fixed-deployed.** `mempool_race` is now dependency-relative: it reads
  the triggering pending tx's effective priority fee and bids above it with a
  deterministic buffer.
- Action: none. Confirm on the next mempool-race exit that the selected priority
  fee exceeds the dependency's.

---

## Cross-cutting issues (not a single execution row)

### X1. Receipt reconciliation orphaned mined buys across restart — RECOVERY GAP

- Evidence: `trd_mpmxrb0o` and `trd_mpmxoqfn` mined `status=0x1` to the vault but
  stayed `buy_submitted`; tokens sat unmanaged in the vault.
- Root cause: `load_submitted_executions` filtered by the current `run_id`, so a
  process restart with a new run id orphaned the prior run's submitted txs.
- Status: **Fixed-pending** (working tree). Query is now cross-run: it reconciles
  any `buy_submitted`/`sell_submitted` position with a pending tx hash from any
  run.
- Action: build and deploy; confirm the orphaned positions reconcile on start.

### X2. No real-mode position valuation (dashboard "Current Value -") — PARITY GAP

- Evidence: real run wrote 7 position snapshots (all `current_value_eth=0`) vs
  2924 in the parallel backtest; a 13.88x winner showed no current value.
- Root cause: `TxExecutorAdapter` did not implement `simulate_position_value`
  (inherited default `Ok(None)`), and valuation only triggered on `PoolUpdated`
  which real mode barely receives.
- Status: **Fixed-pending** (working tree). Real adapter now delegates valuation
  to a chain-server-backed `LiveChainSimExecutionAdapter`; the loop values open
  positions every block; asena API + frontend expose an Open Positions table
  with current value.
- Action: build and deploy; verify open real positions get per-block values.
  See the dedicated tier-one parity entries in `bogaz.md`.

---

## Action summary (by priority)

| Pri | Item | Status | Action |
| --- | --- | --- | --- |
| P0 | S1 stale can_sell gate | Fixed-pending | Build + deploy |
| P0 | X1 receipt reconciliation cross-run | Fixed-pending | Build + deploy |
| P0 | X2 real valuation parity | Fixed-pending | Build + deploy |
| P1 | B6 buy mined-revert | Active | Min-output bound on buy + shrink sim→inclusion gap |
| P1 | B2 pre-submit 1-block lag | Active | Bounded wait for required block instead of defer |
| — | B1 sim-would-revert cancels | Correct | Monitor rate; feed entry filter |
| — | B3/B4/B5 ETH tx executor path | Fixed-deployed | None |
| — | S2 provisional-revert cancels | Correct | Feed entry filter |
| — | S3 value-cap cancels | Correct | None |
| — | S4 mempool-race outbid | Fixed-deployed | Confirm dependency-relative bid |
