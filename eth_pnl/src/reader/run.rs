use serde_json::{json, Value};
use sqlx::{postgres::PgRow, PgPool, Row};

use crate::Result;

// Run selection intentionally stays schema-level: callers get the latest
// complete token_pnl run when available, falling back to the freshest run.
pub async fn latest_token_pnl_run(pool: &PgPool) -> Result<Option<PgRow>> {
    Ok(sqlx::query(
        r#"
        SELECT run_id, mode, algorithm_version, start_block, end_block,
               status, metadata, created_at, updated_at
        FROM token_pnl.calculation_runs
        ORDER BY (status = 'complete') DESC, updated_at DESC, run_id DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?)
}

pub fn token_pnl_run_json(row: &PgRow, run_id: &str) -> Result<Value> {
    Ok(json!({
        "run_id": run_id,
        "mode": row.try_get::<String, _>("mode")?,
        "algorithm_version": row.try_get::<String, _>("algorithm_version")?,
        "start_block": row.try_get::<Option<i64>, _>("start_block")?,
        "end_block": row.try_get::<Option<i64>, _>("end_block")?,
        "status": row.try_get::<String, _>("status")?,
        "metadata": row.try_get::<Value, _>("metadata")?,
        "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")?,
        "updated_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")?,
    }))
}
