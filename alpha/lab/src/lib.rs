pub mod backtest_validation;
pub mod position_lab;
pub mod render;
pub mod strategy_assessment;
#[path = "strategy_lab/event_trace/mod.rs"]
pub mod strategy_event_trace;
pub mod strategy_lab;

use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .wrap_err("failed to connect to alpha database")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunMetadata {
    pub run_id: String,
    pub mode: String,
    pub status: String,
    pub config: serde_json::Value,
    pub metadata: serde_json::Value,
    pub replay_run_id: Option<String>,
}

pub async fn load_run_metadata(pool: &PgPool, run_id: &str) -> Result<RunMetadata> {
    let row = sqlx::query(
        r#"
        SELECT run_id, mode, status, config, metadata, config->>'replay_run_id' AS replay_run_id
        FROM alpha_trading.trader_runs
        WHERE run_id = $1
        "#,
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await
    .wrap_err("failed to load trader run metadata")?
    .ok_or_else(|| eyre::eyre!("run_id not found in alpha_trading.trader_runs: {run_id}"))?;

    Ok(RunMetadata {
        run_id: row.try_get("run_id")?,
        mode: row.try_get("mode")?,
        status: row.try_get("status")?,
        config: row.try_get("config")?,
        metadata: row.try_get("metadata")?,
        replay_run_id: row.try_get("replay_run_id")?,
    })
}

pub fn parse_f64(value: &Option<String>) -> Option<f64> {
    value.as_deref()?.parse::<f64>().ok()
}

pub fn is_nonzero_hex(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let normalized = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed)
        .trim_start_matches('0');
    !normalized.is_empty()
}
