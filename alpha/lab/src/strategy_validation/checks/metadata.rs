use eyre::Result;
use serde_json::json;
use sqlx::PgPool;

use super::super::db::{ResultSetRecord, StrategySummary};
use super::super::report::{CheckResult, Verdict};
use super::common::{check, count_check};

pub(super) fn result_set_status_check(result_set: &ResultSetRecord) -> CheckResult {
    let expected_statuses = match result_set.mode.as_str() {
        "historical" => vec!["completed"],
        "live" => vec!["running", "stopped"],
        _ => Vec::new(),
    };
    let verdict = if expected_statuses.is_empty() {
        Verdict::Blocked
    } else if expected_statuses
        .iter()
        .any(|status| *status == result_set.status)
    {
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    let message = if expected_statuses.is_empty() {
        format!(
            "result set mode `{}` has no strict lab status expectation",
            result_set.mode
        )
    } else if verdict == Verdict::Pass {
        format!(
            "result set status `{}` is valid for mode `{}`",
            result_set.status, result_set.mode
        )
    } else {
        format!(
            "result set status is `{}`, expected one of `{}` for mode `{}`",
            result_set.status,
            expected_statuses.join("`, `"),
            result_set.mode
        )
    };
    check(
        "metadata",
        "result_set_status",
        verdict,
        message,
        json!({
            "mode": result_set.mode,
            "status": result_set.status,
            "expected_statuses": expected_statuses,
        }),
    )
}

pub(super) fn live_runtime_status_check(result_set: &ResultSetRecord) -> CheckResult {
    let live_status = result_set
        .metadata
        .get("live_status")
        .and_then(|value| value.as_str());
    let verdict = if result_set.mode != "live" {
        Verdict::Pass
    } else if result_set.status == "running" && live_status == Some("failed") {
        Verdict::Fail
    } else {
        Verdict::Pass
    };
    let message = if result_set.mode != "live" {
        "non-live result set has no live runtime status expectation".to_string()
    } else if verdict == Verdict::Pass {
        format!(
            "live runtime status `{}` is compatible with result-set status `{}`",
            live_status.unwrap_or("<unset>"),
            result_set.status
        )
    } else {
        format!(
            "result set is marked running but live runtime status is `{}`",
            live_status.unwrap_or("<unset>")
        )
    };
    check(
        "metadata",
        "live_runtime_status_matches_result_set",
        verdict,
        message,
        json!({
            "mode": result_set.mode,
            "result_set_status": result_set.status,
            "live_status": live_status,
            "live_last_error": result_set.metadata.get("live_last_error"),
            "poll_error": result_set.metadata.get("poll_error"),
        }),
    )
}

pub(super) async fn live_chain_sim_source_check(
    pool: &PgPool,
    result_set_id: &str,
) -> Result<CheckResult> {
    count_check(
        pool,
        "metadata",
        "live_chain_sim_uses_chain_server_simulator",
        Verdict::Fail,
        "live chain-sim result sets use the chain-server LiveTxSimulator state source",
        "live chain-sim result-set/run rows without chain-server simulator source metadata",
        r#"
        SELECT count(*)
        FROM alpha_trading.backtest_result_sets rs
        JOIN alpha_trading.backtest_result_set_runs rsr
          ON rsr.result_set_id = rs.result_set_id
        JOIN alpha_trading.trader_runs tr
          ON tr.run_id = rsr.run_id
        WHERE rs.result_set_id = $1
          AND ($2::text IS NULL OR $2::text IS NOT NULL)
          AND rs.mode = 'live'
          AND tr.mode = 'chain-sim'
          AND (
              COALESCE(rs.metadata#>>'{chain_sim_state,source}', '') <> 'chain_server_live_tx_simulator'
              OR COALESCE(tr.metadata#>>'{chain_sim_state,source}', '') <> 'chain_server_live_tx_simulator'
          )
        "#,
        result_set_id,
        None,
    )
    .await
}

pub(super) async fn live_block_frame_runtime_metadata_check(
    pool: &PgPool,
    result_set_id: &str,
) -> Result<CheckResult> {
    count_check(
        pool,
        "metadata",
        "live_block_frame_runtime_metadata",
        Verdict::Fail,
        "live chain-sim runs record block-frame input metadata",
        "live chain-sim result-set/run rows missing block-frame runtime metadata",
        r#"
        SELECT count(*)
        FROM alpha_trading.backtest_result_sets rs
        JOIN alpha_trading.backtest_result_set_runs rsr
          ON rsr.result_set_id = rs.result_set_id
        JOIN alpha_trading.trader_runs tr
          ON tr.run_id = rsr.run_id
        WHERE rs.result_set_id = $1
          AND ($2::text IS NULL OR $2::text IS NOT NULL)
          AND rs.mode = 'live'
          AND tr.mode = 'chain-sim'
          AND (
              COALESCE(rs.config->>'loop_wait', '') <> 'chain_server_live_updates'
              OR NOT (rs.metadata ? 'block_frame_pool_count')
              OR NOT (tr.metadata ? 'block_frame_pool_count')
          )
        "#,
        result_set_id,
        None,
    )
    .await
}

pub(super) async fn result_set_running_state_consistency_check(
    pool: &PgPool,
    result_set_id: &str,
) -> Result<CheckResult> {
    count_check(
        pool,
        "metadata",
        "running_result_set_has_no_stop_marker",
        Verdict::Fail,
        "running result sets and runs do not carry stopped markers",
        "running result sets or runs with stopped_at/shutdown metadata",
        r#"
        SELECT count(*)
        FROM alpha_trading.backtest_result_sets rs
        LEFT JOIN alpha_trading.backtest_result_set_runs rsr
          ON rsr.result_set_id = rs.result_set_id
        LEFT JOIN alpha_trading.trader_runs tr
          ON tr.run_id = rsr.run_id
        WHERE rs.result_set_id = $1
          AND ($2::text IS NULL OR $2::text IS NOT NULL)
          AND (
              (rs.status = 'running' AND rs.stopped_at IS NOT NULL)
              OR (tr.status = 'running' AND tr.stopped_at IS NOT NULL)
              OR (
                  rs.status = 'running'
                  AND rs.metadata->>'reason' IN ('shutdown_signal', 'stale_heartbeat')
              )
              OR (
                  tr.status = 'running'
                  AND tr.metadata->>'reason' IN ('shutdown_signal', 'stale_heartbeat')
              )
          )
        "#,
        result_set_id,
        None,
    )
    .await
}

pub(super) fn strategy_rows_check(
    strategy_summaries: &[StrategySummary],
    strategy: Option<&str>,
) -> CheckResult {
    let count = strategy_summaries.len();
    let trade_count: i64 = strategy_summaries
        .iter()
        .map(|summary| summary.trades)
        .sum();
    let verdict = if trade_count > 0 {
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    check(
        "metadata",
        "strategy_rows",
        verdict,
        format!("{trade_count} trades found across {count} strategy rows"),
        json!({
            "strategy_filter": strategy,
            "strategy_rows": count,
            "trades": trade_count,
        }),
    )
}

pub(super) async fn trade_id_format_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "identity",
        "public_trade_id_format",
        Verdict::Fail,
        "all public trade IDs use the `trd_` prefix",
        "trade IDs without `trd_` prefix",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
          AND trade_id NOT LIKE 'trd\_%' ESCAPE '\'
        "#,
        result_set_id,
        strategy,
    )
    .await
}
