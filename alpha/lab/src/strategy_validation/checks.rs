use eyre::Result;
use sqlx::PgPool;

use super::db::{ResultSetRecord, StrategySummary};
use super::report::CheckResult;

mod accounting;
mod common;
mod decision_timing;
mod execution_replay;
mod lifecycle;
mod metadata;
mod risk_policy;
mod signal_scope;
mod snapshots;

pub async fn run_checks(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
    strategy_summaries: &[StrategySummary],
) -> Result<Vec<CheckResult>> {
    let result_set_id = result_set.result_set_id.as_str();
    let mut checks = Vec::new();
    checks.push(metadata::result_set_status_check(result_set));
    checks.push(metadata::live_runtime_status_check(result_set));
    checks.push(metadata::live_chain_sim_source_check(pool, result_set_id).await?);
    checks.push(metadata::live_block_frame_runtime_metadata_check(pool, result_set_id).await?);
    checks.push(metadata::result_set_running_state_consistency_check(pool, result_set_id).await?);
    checks.push(metadata::strategy_rows_check(strategy_summaries, strategy));
    checks.push(metadata::trade_id_format_check(pool, result_set_id, strategy).await?);
    checks.push(signal_scope::historical_mempool_scope_check(pool, result_set, strategy).await?);
    checks.push(
        signal_scope::historical_mempool_observations_check(pool, result_set, strategy).await?,
    );
    checks.push(
        decision_timing::submitted_events_have_decisions_check(
            pool,
            result_set_id,
            strategy,
            "buy",
        )
        .await?,
    );
    checks.push(
        decision_timing::submitted_events_have_decisions_check(
            pool,
            result_set_id,
            strategy,
            "sell",
        )
        .await?,
    );
    checks.push(
        decision_timing::risk_sell_decisions_have_prior_risk_events_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(
        decision_timing::risk_sell_decisions_submit_on_signal_block_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(
        risk_policy::configured_critical_risks_have_strategy_response_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(
        risk_policy::lp_approval_deferrals_have_age_evidence_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        risk_policy::liquidity_removal_risk_kind_source_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        signal_scope::mempool_risk_events_use_detector_head_block_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(
        risk_policy::mempool_liquidity_removal_does_not_zero_snapshot_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(
        signal_scope::historical_market_buy_decisions_have_observations_check(
            pool, result_set, strategy,
        )
        .await?,
    );
    checks.push(
        signal_scope::deferred_mempool_signals_have_reason_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        signal_scope::chain_sim_trading_enabled_mempool_skip_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        signal_scope::no_settlement_wait_mempool_deferrals_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(signal_scope::submit_decisions_in_range_check(pool, result_set, strategy).await?);
    checks.push(
        signal_scope::trades_match_allowed_protocols_check(pool, result_set_id, strategy).await?,
    );
    checks.push(execution_replay::execution_delay_check(pool, result_set, strategy).await?);
    checks
        .push(execution_replay::terminal_report_presence_check(pool, result_set, strategy).await?);
    checks.push(
        execution_replay::live_chain_sim_block_alignment_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        execution_replay::live_chain_sim_block_hash_evidence_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        execution_replay::chain_sim_real_execution_artifacts_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        execution_replay::pre_submit_simulation_state_ready_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        execution_replay::terminal_gas_policy_fee_evidence_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        execution_replay::confirmed_reports_have_simulated_outputs_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(execution_replay::tail_entry_coverage_check(pool, result_set, strategy).await?);
    checks.push(
        execution_replay::chain_sim_tail_entry_absence_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        execution_replay::tail_entry_intents_have_exact_vault_evidence_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(
        execution_replay::tail_entry_ordering_evidence_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        execution_replay::tail_entry_priority_undercut_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        execution_replay::tail_entry_live_backtest_n_plus_1_validation_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks.push(
        lifecycle::entry_block_matches_buy_confirmation_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        lifecycle::exit_block_matches_sell_confirmation_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        lifecycle::no_exit_block_before_terminal_sell_check(pool, result_set_id, strategy).await?,
    );
    checks.push(lifecycle::trade_position_rollup_check(pool, result_set_id, strategy).await?);
    checks.push(
        lifecycle::execution_report_trade_event_mirror_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        lifecycle::single_submitted_event_per_order_check(pool, result_set_id, strategy).await?,
    );
    checks
        .push(lifecycle::no_duplicate_terminal_events_check(pool, result_set_id, strategy).await?);
    checks.push(lifecycle::lifecycle_order_check(pool, result_set_id, strategy).await?);
    checks.push(lifecycle::active_hold_limit_exit_check(pool, result_set_id, strategy).await?);
    checks.push(
        lifecycle::drained_position_reaches_closed_zero_valuation_check(
            pool,
            result_set_id,
            strategy,
        )
        .await?,
    );
    checks
        .push(accounting::entry_cost_matches_buy_fill_check(pool, result_set_id, strategy).await?);
    checks
        .push(accounting::exit_value_matches_sell_fill_check(pool, result_set_id, strategy).await?);
    checks.push(accounting::gas_cost_matches_events_check(pool, result_set_id, strategy).await?);
    checks
        .push(accounting::failed_sell_gas_has_snapshot_check(pool, result_set_id, strategy).await?);
    checks.push(accounting::pnl_sum_check(pool, result_set_id, strategy).await?);
    checks.push(accounting::open_trade_pnl_formula_check(pool, result_set_id, strategy).await?);
    checks.push(accounting::open_snapshot_pnl_formula_check(pool, result_set_id, strategy).await?);
    checks.push(accounting::realized_sell_pnl_check(pool, result_set_id, strategy).await?);
    checks
        .push(accounting::closed_trade_zero_unrealized_check(pool, result_set_id, strategy).await?);
    checks.push(
        accounting::closed_trade_snapshot_zero_unrealized_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        snapshots::no_snapshots_after_sell_confirmed_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        snapshots::no_future_valued_open_snapshots_after_sell_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        snapshots::no_snapshots_before_buy_confirmation_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(snapshots::latest_snapshot_block_check(pool, result_set_id, strategy).await?);
    checks.push(snapshots::latest_snapshot_values_check(pool, result_set_id, strategy).await?);
    checks.push(
        snapshots::observed_block_not_after_valuation_check(pool, result_set_id, strategy).await?,
    );
    checks
        .push(snapshots::duplicate_snapshot_coordinate_check(pool, result_set_id, strategy).await?);
    checks.push(
        snapshots::duplicate_position_snapshot_coordinate_check(pool, result_set_id, strategy)
            .await?,
    );
    checks.push(
        snapshots::trade_position_snapshot_mirror_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        snapshots::zero_value_snapshot_pool_metrics_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        snapshots::terminal_snapshot_no_pool_metrics_check(pool, result_set_id, strategy).await?,
    );
    checks.push(snapshots::closed_trade_final_snapshot_check(pool, result_set_id, strategy).await?);
    checks
        .push(snapshots::closed_trade_latest_snapshot_check(pool, result_set_id, strategy).await?);
    checks
        .push(snapshots::drain_block_has_zero_snapshot_check(pool, result_set_id, strategy).await?);
    checks.push(
        snapshots::synthetic_balance_used_after_drain_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        snapshots::positive_value_after_zero_balance_check(pool, result_set_id, strategy).await?,
    );
    checks.push(
        execution_replay::execution_replay_inputs_check(pool, result_set_id, strategy).await?,
    );
    Ok(checks)
}
