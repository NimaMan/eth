use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};

use super::report::TradeSample;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultSetRecord {
    pub result_set_id: String,
    pub mode: String,
    pub status: String,
    pub strategy_suite: Option<String>,
    pub start_block: Option<i64>,
    pub end_block: Option<i64>,
    pub config: Value,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
    pub stopped_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategySummary {
    pub strategy_name: String,
    pub trades: i64,
    pub closed: i64,
    pub open: i64,
    pub failed: i64,
    pub realized_pnl_eth: String,
    pub unrealized_pnl_eth: String,
    pub total_pnl_eth: String,
}

pub async fn load_result_set(pool: &PgPool, result_set_id: &str) -> Result<ResultSetRecord> {
    let row = sqlx::query(
        r#"
        SELECT result_set_id,
               mode,
               status,
               strategy_suite,
               start_block,
               end_block,
               config,
               metadata,
               created_at::text AS created_at,
               updated_at::text AS updated_at,
               stopped_at::text AS stopped_at
        FROM alpha_trading.backtest_result_sets
        WHERE result_set_id = $1
        "#,
    )
    .bind(result_set_id)
    .fetch_optional(pool)
    .await
    .wrap_err("failed to load backtest result set")?
    .ok_or_else(|| eyre::eyre!("result_set_id not found: {result_set_id}"))?;

    Ok(ResultSetRecord {
        result_set_id: row.try_get("result_set_id")?,
        mode: row.try_get("mode")?,
        status: row.try_get("status")?,
        strategy_suite: row.try_get("strategy_suite")?,
        start_block: row.try_get("start_block")?,
        end_block: row.try_get("end_block")?,
        config: row.try_get("config")?,
        metadata: row.try_get("metadata")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        stopped_at: row.try_get("stopped_at")?,
    })
}

pub async fn load_strategy_summaries(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<Vec<StrategySummary>> {
    let rows = sqlx::query(
        r#"
        SELECT strategy_name,
               count(*) AS trades,
               count(*) FILTER (WHERE state = 'sell_confirmed') AS closed,
               count(*) FILTER (WHERE state NOT IN ('sell_confirmed', 'sell_failed', 'buy_failed', 'failed', 'cancelled')) AS open,
               count(*) FILTER (WHERE state IN ('sell_failed', 'buy_failed', 'failed', 'cancelled')) AS failed,
               coalesce(sum(coalesce(nullif(realized_pnl_eth, '')::numeric, 0)), 0)::text AS realized_pnl_eth,
               coalesce(sum(coalesce(nullif(unrealized_pnl_eth, '')::numeric, 0)), 0)::text AS unrealized_pnl_eth,
               coalesce(sum(coalesce(nullif(total_pnl_eth, '')::numeric, 0)), 0)::text AS total_pnl_eth
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
        GROUP BY strategy_name
        ORDER BY sum(coalesce(nullif(total_pnl_eth, '')::numeric, 0)) DESC, strategy_name ASC
        "#,
    )
    .bind(result_set_id)
    .bind(strategy)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load validation strategy summaries")?;

    rows.into_iter()
        .map(|row| {
            Ok(StrategySummary {
                strategy_name: row.try_get("strategy_name")?,
                trades: row.try_get("trades")?,
                closed: row.try_get("closed")?,
                open: row.try_get("open")?,
                failed: row.try_get("failed")?,
                realized_pnl_eth: row.try_get("realized_pnl_eth")?,
                unrealized_pnl_eth: row.try_get("unrealized_pnl_eth")?,
                total_pnl_eth: row.try_get("total_pnl_eth")?,
            })
        })
        .collect()
}

pub async fn load_trade_samples(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
    limit: i64,
) -> Result<Vec<TradeSample>> {
    let limit = limit.max(1);
    let rows = sqlx::query(
        r#"
        WITH scoped AS (
            SELECT trade_id,
                   strategy_name,
                   state,
                   token_address,
                   pool_address,
                   entry_block,
                   exit_block,
                   entry_cost_eth,
                   exit_value_eth,
                   realized_pnl_eth,
                   unrealized_pnl_eth,
                   total_pnl_eth,
                   roi,
                   coalesce(nullif(total_pnl_eth, '')::numeric, 0) AS total_pnl_numeric
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        top_winners AS (
            SELECT 'top_winner' AS sample_kind, *
            FROM scoped
            ORDER BY total_pnl_numeric DESC, trade_id ASC
            LIMIT $3
        ),
        worst_losers AS (
            SELECT 'worst_loser' AS sample_kind, *
            FROM scoped
            ORDER BY total_pnl_numeric ASC, trade_id ASC
            LIMIT $3
        )
        SELECT *
        FROM top_winners
        UNION ALL
        SELECT *
        FROM worst_losers
        ORDER BY sample_kind ASC, total_pnl_numeric DESC, trade_id ASC
        "#,
    )
    .bind(result_set_id)
    .bind(strategy)
    .bind(limit)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load validation trade samples")?;

    rows.into_iter()
        .map(|row| {
            Ok(TradeSample {
                sample_kind: row.try_get("sample_kind")?,
                trade_id: row.try_get("trade_id")?,
                strategy_name: row.try_get("strategy_name")?,
                state: row.try_get("state")?,
                token_address: row.try_get("token_address")?,
                pool_address: row.try_get("pool_address")?,
                entry_block: row.try_get("entry_block")?,
                exit_block: row.try_get("exit_block")?,
                entry_cost_eth: row.try_get("entry_cost_eth")?,
                exit_value_eth: row.try_get("exit_value_eth")?,
                realized_pnl_eth: row.try_get("realized_pnl_eth")?,
                unrealized_pnl_eth: row.try_get("unrealized_pnl_eth")?,
                total_pnl_eth: row.try_get("total_pnl_eth")?,
                roi: row.try_get("roi")?,
            })
        })
        .collect()
}

pub async fn scalar_i64(
    pool: &PgPool,
    sql: &str,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<i64> {
    let row = sqlx::query(sql)
        .bind(result_set_id)
        .bind(strategy)
        .fetch_one(pool)
        .await
        .wrap_err_with(|| format!("failed validation scalar query: {sql}"))?;
    if let Ok(value) = row.try_get::<i64, _>(0) {
        return Ok(value);
    }
    if let Ok(value) = row.try_get::<i32, _>(0) {
        return Ok(i64::from(value));
    }
    row.try_get::<i16, _>(0)
        .map(i64::from)
        .wrap_err("failed to decode validation scalar")
}
