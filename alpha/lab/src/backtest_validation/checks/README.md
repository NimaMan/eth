# Validation Checks

This directory contains the individual checks used by the comprehensive
backtest validator. Each check asks one trust question about a persisted
strategy result and returns a backend-owned `CheckResult`.

## Module Map

- `metadata.rs`: result-set shape and public trade id sanity.
- `signal_scope.rs`: historical/live evidence boundaries and block range scope.
- `decision_timing.rs`: whether orders can be traced to strategy decisions and
  risk evidence.
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
- Numeric accounting uses `0.000000000000001` ETH tolerance.
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

### Decision Timing

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `buy_submitted_has_decision` | Can every buy submission be explained by a strategy decision? | Every buy order should trace to a same-block `submit_buy` decision for auditability. | `decision_timing.rs` |
| `sell_submitted_has_decision` | Can every sell submission be explained by a strategy decision? | Every sell order should trace to a same-block `submit_sell` decision for auditability. | `decision_timing.rs` |
| `risk_sell_has_available_signal` | Was each risk-triggered sell based on already-available evidence? | A risk exit should not depend on evidence observed after the decision block. | `decision_timing.rs` |
| `risk_sell_signal_block_immediate` | Did risk-triggered exits submit immediately on the signal block? | For risk-driven exits, delayed submission can materially change loss and should be explicit. | `decision_timing.rs` |

### Execution Replay

| Code | Question | Why We Ask | File |
| --- | --- | --- | --- |
| `terminal_report_matches_execution_delay` | Do fills land at the configured execution delay? | Backtests should honor the configured delay from submission to terminal execution result. | `execution_replay.rs` |
| `confirmed_reports_have_simulation_outputs` | Are confirmed fills backed by persisted EVM simulation output? | A confirmed fill must carry fill amount, gas, gas cost, and buy token output so PnL can be reconstructed. | `execution_replay.rs` |
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
| `total_pnl_equals_realized_plus_unrealized` | Does total PnL reconcile with realized and unrealized PnL? | This catches inconsistent aggregate math before charts or summaries use total PnL. | `accounting.rs` |
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
| `zero_value_snapshots_do_not_reuse_stale_pool_metrics` | Do zero-value exposure snapshots avoid stale pool metrics? | A zero-value exposure after a drain should not continue to display old pool liquidity or price data. | `snapshots.rs` |
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
