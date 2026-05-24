# Validation Checks

This directory contains the individual checks used by the comprehensive
strategy validator. Each check asks one trust question about a persisted
strategy result and returns a backend-owned `CheckResult`.

## Module Map

- `metadata.rs`: result-set shape and public trade id sanity.
- `signal_scope.rs`: historical/live evidence boundaries and block range scope.
- `decision_timing.rs`: whether orders can be traced to strategy decisions and
  risk evidence.
- `risk_policy.rs`: whether persisted risk rows have coherent semantics and
  configured strategy responses.
- `execution_replay.rs`: execution-delay and replay-input completeness.
- `lifecycle.rs`: trade event ordering and trade/position rollup consistency.
- `accounting.rs`: fill, gas, realized/unrealized/total PnL consistency.
- `snapshots.rs`: mark-to-market timeline and latest snapshot rollup checks.
- `common.rs`: shared result builders, tolerance, and user-facing check copy.

## Shared Semantics

- `Fail` means the result should not be trusted until fixed or explained.
- `Warn` is retained for older persisted reports; current validation checks
  should produce only `Pass`, `Fail`, or `Blocked`.
- `Pass` means the scoped invariant had zero violations.
- `Blocked` means validation could not answer because the result-set shape is
  unsupported or prerequisite data is missing.
- Numeric accounting uses `0.000001` ETH tolerance.
- The frontend renders these results and must not recalculate membership,
  accounting, or trust verdicts.
- Distribution, concentration, and profitability robustness questions belong in
  `strategy_assessment`.

## Checks

### Metadata

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `result_set_status` | Is this result set in the expected state? | Historical results should be completed; live results can be running or stopped. A wrong status means the data may be partial or from the wrong mode. | `metadata.rs` |
| `running_result_set_has_no_stop_marker` | Is a running live result free of stopped-run markers? | A running result set should not carry stopped timestamps or stale shutdown metadata from a prior process lifetime. | `metadata.rs` |
| `strategy_rows` | Did this strategy produce trades in this result set? | A validation report without scoped trades is not meaningful for PnL or strategy behavior. | `metadata.rs` |
| `public_trade_id_format` | Are public trade identifiers opaque and stable? | UI and reports should expose stable `trd_` ids instead of leaking legacy position identifiers. | `metadata.rs` |

### Signal Scope

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `historical_mempool_rows` | Did a historical backtest avoid pending mempool evidence? | Historical backtests must use mined/local evidence only. Pending mempool rows would leak live-only information into historical evaluation. | `signal_scope.rs` |
| `market_buy_has_historical_observation` | Can every historical market buy be traced to an input observation? | A buy decision in historical mode should come from the replayed Risk Atlas observation for the same token, pool, and block. | `signal_scope.rs` |
| `submit_decisions_within_result_range` | Were submitted decisions made inside the replayed input range? | Decisions outside the result-set block window would contaminate the backtest window. | `signal_scope.rs` |
| `trades_match_allowed_protocols` | Did trades respect configured protocol filters? | A V2-only candidate should not accidentally persist V3/V4 trades because entry filtering or read-model wiring drifted. | `signal_scope.rs` |

### Decision Timing

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `buy_submitted_has_decision` | Can every buy submission be explained by a strategy decision? | Every buy order should trace to a same-block `submit_buy` decision for auditability. | `decision_timing.rs` |
| `sell_submitted_has_decision` | Can every sell submission be explained by a strategy decision? | Every sell order should trace to a same-block `submit_sell` decision for auditability. | `decision_timing.rs` |
| `risk_sell_has_available_signal` | Was each risk-triggered sell based on already-available evidence? | A risk exit should not depend on evidence observed after the decision block. | `decision_timing.rs` |
| `risk_sell_signal_block_immediate` | Did risk-triggered exits submit immediately on the signal block? | For risk-driven exits, delayed submission can materially change loss and should be explicit. | `decision_timing.rs` |

### Risk Policy

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `configured_critical_risk_has_strategy_response` | Did configured critical risks receive the strategy response they require? | Enabled critical risk exits should submit a sell, record an explicit same-block deferral, or, in live backtests, show a same-signal mempool-removal sell cancelled by the gas/value cap. | `risk_policy.rs` |
| `lp_approval_deferral_has_age_evidence` | Do LP-approval deferrals carry explicit active-block evidence? | Deferring instead of selling is only valid inside the configured launch window, so the signal id, age basis, and active-block age must be persisted. | `risk_policy.rs` |
| `liquidity_removal_risk_kind_matches_source` | Does liquidity-removal risk kind match its evidence source? | Pending mempool removal signals and mined-chain removals have different accounting and valuation semantics. | `risk_policy.rs` |
| `mempool_liquidity_removal_does_not_zero_exposure_snapshot` | Do mempool liquidity-removal signals avoid marking exposure as drained? | A pending mempool tx can justify an exit, but it should not zero confirmed exposure before mined evidence exists. | `risk_policy.rs` |

### Execution Replay

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `terminal_report_matches_execution_delay` | Do fills land at the configured execution delay? | Backtests should honor the configured delay from submission to terminal execution result. | `execution_replay.rs` |
| `submitted_orders_have_terminal_report_after_delay` | Does every elapsed submitted order have a terminal execution report? | A submitted order stuck past the execution delay means the execution adapter or persistence pipeline dropped the terminal outcome. | `execution_replay.rs` |
| `confirmed_reports_have_simulation_outputs` | Are confirmed fills backed by persisted EVM simulation output? | A confirmed fill must carry fill amount, gas, gas cost, and buy token output so PnL can be reconstructed. | `execution_replay.rs` |
| `tail_entry_intent_has_exact_vault_buy_evidence` | Are tail-entry buys backed by exact deployed-vault evidence? | Tail-entry buys must prove the deployed vault route works, not only the generic pool buy/sell probe. | `execution_replay.rs` |
| `tail_entry_buy_has_ordering_evidence` | Do tail-entry buys retain dependency ordering evidence? | Same-block tail entry only makes sense if the enabling tx hash and fee evidence used for behind-the-tx placement are persisted. | `execution_replay.rs` |
| `tail_entry_buy_uses_live_backtest_n_plus_1_validation` | Do tail-entry buys use the live-backtest N+1 execution model? | Live backtest models mempool tail-entry as submit at signal block N, then simulate the fill against post-block N+1 state; same-block overlay proof belongs to real-live deployment gates. | `execution_replay.rs` |
| `closed_trade_replay_inputs_present` | Can this closed trade be independently replayed? | Closed trades need buy token amount, sell order amount, and sell fill data for independent chain-sim replay. | `execution_replay.rs` |

### Lifecycle And Rollups

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `entry_block_matches_buy_confirmed` | Does `entry_block` mean the buy-confirmed block? | Entry timing drives hold duration, risk timing, and performance charts. | `lifecycle.rs` |
| `exit_block_matches_sell_confirmed` | Does `exit_block` mean the sell-confirmed block? | Exit timing drives realized PnL, hold duration, and closed-trade chart points. | `lifecycle.rs` |
| `no_exit_block_before_sell_confirmed` | Do open or failed trades avoid fake exit blocks? | Non-closed trades should not appear closed because of inferred or stale snapshot data. | `lifecycle.rs` |
| `trade_rollup_matches_position` | Does each trade row still match its source position? | `trades` is a read model over `positions`; state, order ids, blocks, and protocol must not drift. | `lifecycle.rs` |
| `single_terminal_event_per_trade` | Does each trade have only one terminal buy and sell confirmation? | Duplicate terminal events corrupt lifecycle state and accounting. | `lifecycle.rs` |
| `event_block_order` | Are lifecycle event blocks ordered correctly? | Buy submit, buy terminal, sell submit, and sell terminal blocks must form a possible timeline. | `lifecycle.rs` |
| `active_hold_limit_submits_exit` | Did max-hold positions actually submit exits? | A position that reached the active hold limit should have a sell submission, otherwise the strategy lifecycle is stuck. | `lifecycle.rs` |

### Accounting

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `entry_cost_matches_buy_fill` | Does entry cost come from the buy simulation fill? | Entry cost is the cost basis for ROI and realized/unrealized PnL. | `accounting.rs` |
| `exit_value_matches_sell_fill` | Does exit value come from the sell simulation fill? | Exit value is the realized proceeds for closed-trade PnL. | `accounting.rs` |
| `gas_cost_matches_trade_events` | Does gas cost come from the execution event stream? | Realized PnL must subtract all persisted buy/sell gas, not a stale or partial aggregate. | `accounting.rs` |
| `failed_sell_gas_has_same_block_snapshot` | Does failed sell gas have a matching PnL snapshot? | Failed sell gas changes realized PnL immediately, so the failure block needs its own sell_failed snapshot. | `accounting.rs` |
| `total_pnl_equals_realized_plus_unrealized` | Does total PnL reconcile with realized and unrealized PnL? | This catches inconsistent aggregate math before charts or summaries use total PnL. | `accounting.rs` |
| `open_trade_pnl_formula` | Does open-trade PnL reconcile with entry, current value, and gas? | Open trades should realize only gas while unrealized PnL equals current value minus entry cost. | `accounting.rs` |
| `open_snapshot_pnl_formula` | Do open-state snapshots reconcile with entry, current value, and gas? | Intermediate open snapshots should use gas accumulated through the valuation block and current value at that snapshot. | `accounting.rs` |
| `realized_sell_pnl_formula` | Does realized PnL reconcile with entry, exit, and gas? | Closed-trade realized PnL should equal `exit_value - entry_cost - gas_cost`. | `accounting.rs` |
| `closed_trade_has_no_unrealized_value` | Are closed trades fully realized? | A sell-confirmed trade should have zero current value and zero unrealized PnL. | `accounting.rs` |
| `closed_trade_snapshots_have_no_unrealized_value` | Do closed-trade snapshots stay fully realized? | The terminal snapshot should not keep current value or unrealized PnL after sell confirmation. | `accounting.rs` |

### Snapshots

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `no_snapshots_after_sell_confirmed` | Are closed trades no longer receiving snapshots? | Any snapshot appended after a sell-confirmed snapshot can corrupt latest value and charts. | `snapshots.rs` |
| `no_open_snapshot_valued_after_sell_confirmed` | Were open-state snapshots valued only before the sell block? | An open-state valuation after sell confirmation creates impossible post-exit exposure. | `snapshots.rs` |
| `latest_snapshot_block_matches_snapshots` | Does the trade latest snapshot pointer match persisted snapshots? | `trades.latest_snapshot_block` must point at the max persisted snapshot block for the trade. | `snapshots.rs` |
| `latest_snapshot_values_match_trade` | Do trade latest fields match the latest snapshot? | Latest block coordinates, current value, PnL, and ROI in `trades` must match the latest `trade_snapshots` row. | `snapshots.rs` |
| `snapshot_observed_not_after_valuation` | Are snapshot pool observations available at valuation time? | A snapshot valued at block N must not display pool observations from a later block. | `snapshots.rs` |
| `no_duplicate_snapshot_coordinates` | Does each trade have one snapshot per block, state, and valuation block? | Duplicate trade snapshot coordinates make timeline APIs and latest-row selection ambiguous. | `snapshots.rs` |
| `no_duplicate_position_snapshot_coordinates` | Does each raw position snapshot coordinate have only one row? | Position snapshots are the source rows, so the same duplicate invariant must hold before deriving trade snapshots. | `snapshots.rs` |
| `trade_position_snapshots_match` | Do trade snapshots mirror their source position snapshots? | The read model should not drift from the raw position snapshot timeline. | `snapshots.rs` |
| `zero_value_snapshots_do_not_reuse_stale_pool_metrics` | Do zero-value exposure snapshots avoid stale pool metrics? | A display-near-zero exposure may retain same-block pool metrics such as `0x` price/init, but it must not reuse old positive pre-drain metrics. | `snapshots.rs` |
| `terminal_snapshots_have_no_pool_metrics` | Do terminal closed snapshots omit pool metrics? | A sell-confirmed row is terminal accounting state, not an open pool valuation. | `snapshots.rs` |
| `closed_trade_final_snapshot` | Does each closed trade have a final closed snapshot? | UI and validators need a clean sell-confirmed snapshot at `exit_block` for terminal valuation. | `snapshots.rs` |
| `closed_trade_latest_snapshot_is_terminal` | Is the latest closed-trade snapshot terminal? | For a closed trade, the latest snapshot by block/id must be the sell-confirmed exit snapshot, not a stale open valuation. | `snapshots.rs` |

## Adding A Check

1. Choose the module by concern. If no module fits, add a new focused module and
   wire it from `checks.rs`.
2. Prefer `common::count_check` for SQL checks that count violations.
3. Use `common::check` for checks with custom evidence or non-count behavior.
4. Add user-facing question/description copy in `common.rs`.
5. Add the check to this README.
6. Run:

```bash
cargo fmt -p eth_alpha_lab --check
cargo check -p eth_alpha_lab
```

For behavior changes, run one known-good and one known-bad result set so the
validator proves both paths. Do not add PnL concentration, return distribution,
or strategy attractiveness questions here; those are assessment questions.
