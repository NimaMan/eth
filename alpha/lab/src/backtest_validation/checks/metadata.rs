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
    let verdict = if expected_statuses.is_empty()
        || expected_statuses
            .iter()
            .any(|status| *status == result_set.status)
    {
        Verdict::Pass
    } else {
        Verdict::Warn
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
