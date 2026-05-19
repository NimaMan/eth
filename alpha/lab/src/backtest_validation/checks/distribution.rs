use eyre::Result;
use serde_json::json;
use sqlx::PgPool;

use super::super::db::scalar_i64;
use super::super::report::{CheckResult, Verdict};
use super::common::check;

pub(super) async fn pnl_concentration_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    let row_count = scalar_i64(
        pool,
        r#"
        WITH ranked AS (
            SELECT coalesce(nullif(total_pnl_eth, '')::numeric, 0) AS pnl,
                   row_number() OVER (ORDER BY coalesce(nullif(total_pnl_eth, '')::numeric, 0) DESC) AS rn
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        totals AS (
            SELECT coalesce(sum(pnl), 0) AS total_pnl,
                   coalesce(sum(pnl) FILTER (WHERE rn <= 5), 0) AS top5_pnl
            FROM ranked
        )
        SELECT CASE
            WHEN abs(total_pnl) > 0.000000000000001 AND abs(top5_pnl / total_pnl) > 1.0 THEN 1
            ELSE 0
        END
        FROM totals
        "#,
        result_set_id,
        strategy,
    )
    .await?;
    let verdict = if row_count == 0 {
        Verdict::Pass
    } else {
        Verdict::Warn
    };
    Ok(check(
        "distribution",
        "top5_pnl_concentration",
        verdict,
        if row_count == 0 {
            "top five winners do not exceed 100% of aggregate PnL".to_string()
        } else {
            "top five winners exceed 100% of aggregate PnL; strategy conclusion is concentration-sensitive".to_string()
        },
        json!({
            "threshold": "abs(top5_pnl / total_pnl) <= 1.0",
            "violations": row_count,
        }),
    ))
}
