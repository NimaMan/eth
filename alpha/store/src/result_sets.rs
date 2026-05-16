//! Trade-centric result-set read models for alpha backtests and live runs.

use eth_alpha_core::error::{AlphaCoreError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};

const DEFAULT_PERFORMANCE_LIMIT: i64 = 500;
const MAX_PERFORMANCE_LIMIT: i64 = 5_000;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResultSetDetailQuery {
    pub strategy_name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResultSetListQuery {
    pub mode: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResultSetPerformanceQuery {
    pub strategy_name: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetDetailResponse {
    pub result_set: ResultSetView,
    pub run: ResultSetView,
    pub summary: ResultSetSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetListResponse {
    pub result_sets: Vec<ResultSetView>,
    pub live: Vec<ResultSetView>,
    pub historical: Vec<ResultSetView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetView {
    pub result_set_id: String,
    pub run_id: Option<String>,
    pub run_ids: Vec<String>,
    pub mode: String,
    pub status: String,
    pub strategy_suite: Option<String>,
    pub strategy_name: Option<String>,
    pub strategy_id: Option<String>,
    pub strategy_label: Option<String>,
    pub start_block: Option<i64>,
    pub end_block: Option<i64>,
    pub from_block: Option<i64>,
    pub to_block: Option<i64>,
    pub block_count: Option<i64>,
    pub config: Value,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
    pub stopped_at: Option<String>,
    pub trade_count: i64,
    pub positions: i64,
    pub open_trades: i64,
    pub open_positions: i64,
    pub closed_trades: i64,
    pub closed_positions: i64,
    pub failed_trades: i64,
    pub failed_positions: i64,
    pub strategy_count: i64,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub realized_pnl_eth: Option<f64>,
    pub unrealized_pnl_eth: Option<f64>,
    pub total_pnl_eth: Option<f64>,
    pub roi_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetSummary {
    pub positions: i64,
    pub open_positions: i64,
    pub closed_positions: i64,
    pub failed_positions: i64,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub realized_pnl_eth: Option<f64>,
    pub unrealized_pnl_eth: Option<f64>,
    pub total_pnl_eth: Option<f64>,
    pub roi_percent: Option<f64>,
    pub reports: i64,
    pub failed: i64,
    pub risks: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetPerformanceResponse {
    pub points: Vec<ResultSetPerformancePoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetPerformancePoint {
    pub block_number: i64,
    pub trade_count: i64,
    pub open_trades: i64,
    pub closed_trades: i64,
    pub entry_cost_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub realized_pnl_eth: Option<f64>,
    pub unrealized_pnl_eth: Option<f64>,
    pub total_pnl_eth: Option<f64>,
    pub roi_percent: Option<f64>,
    pub recorded_at: Option<String>,
}

pub async fn load_result_sets(
    pool: &PgPool,
    query: ResultSetListQuery,
) -> Result<ResultSetListResponse> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PERFORMANCE_LIMIT)
        .clamp(1, MAX_PERFORMANCE_LIMIT);
    let rows = sqlx::query(
        r#"
        WITH selected AS (
            SELECT result_set_id, mode, status, strategy_suite, start_block, end_block,
                   config::text AS config, metadata::text AS metadata,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   stopped_at::text AS stopped_at
            FROM alpha_trading.backtest_result_sets
            WHERE ($2::text IS NULL OR mode = $2)
            ORDER BY
                CASE WHEN status = 'running' THEN 0 ELSE 1 END,
                updated_at DESC
            LIMIT $1
        ),
        trade_totals AS (
            SELECT result_set_id,
                   COUNT(*) AS trades,
                   COUNT(DISTINCT strategy_name) AS strategies,
                   COUNT(*) FILTER (
                       WHERE lower(state) NOT IN ('sell_confirmed', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
                   ) AS open_trades,
                   COUNT(*) FILTER (WHERE lower(state) = 'sell_confirmed') AS closed_trades,
                   COUNT(*) FILTER (WHERE lower(state) IN ('buy_failed', 'sell_failed', 'failed')) AS failed_trades,
                   SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))::float8 AS buy_volume_eth,
                   SUM(COALESCE(NULLIF(exit_value_eth, '')::numeric, 0))::float8 AS sell_volume_eth,
                   SUM(COALESCE(NULLIF(current_value_eth, '')::numeric, 0))::float8 AS current_value_eth,
                   SUM(COALESCE(NULLIF(realized_pnl_eth, '')::numeric, 0))::float8 AS realized_pnl_eth,
                   SUM(COALESCE(NULLIF(unrealized_pnl_eth, '')::numeric, 0))::float8 AS unrealized_pnl_eth,
                   SUM(COALESCE(NULLIF(total_pnl_eth, '')::numeric, 0))::float8 AS total_pnl_eth,
                   CASE
                       WHEN SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)) > 0
                       THEN (
                           SUM(COALESCE(NULLIF(total_pnl_eth, '')::numeric, 0))
                           / SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))
                           * 100
                       )::float8
                       ELSE NULL
                   END AS roi_percent
            FROM alpha_trading.trades
            WHERE result_set_id IN (SELECT result_set_id FROM selected)
            GROUP BY result_set_id
        ),
        run_ids AS (
            SELECT result_set_id,
                   ARRAY_AGG(run_id ORDER BY created_at DESC) AS run_ids
            FROM alpha_trading.backtest_result_set_runs
            WHERE result_set_id IN (SELECT result_set_id FROM selected)
            GROUP BY result_set_id
        )
        SELECT selected.*,
               COALESCE(trade_totals.trades, 0) AS trades,
               COALESCE(trade_totals.strategies, 0) AS strategies,
               COALESCE(trade_totals.open_trades, 0) AS open_trades,
               COALESCE(trade_totals.closed_trades, 0) AS closed_trades,
               COALESCE(trade_totals.failed_trades, 0) AS failed_trades,
               trade_totals.buy_volume_eth,
               trade_totals.sell_volume_eth,
               trade_totals.current_value_eth,
               trade_totals.realized_pnl_eth,
               trade_totals.unrealized_pnl_eth,
               trade_totals.total_pnl_eth,
               trade_totals.roi_percent,
               COALESCE(run_ids.run_ids, ARRAY[]::text[]) AS run_ids
        FROM selected
        LEFT JOIN trade_totals USING (result_set_id)
        LEFT JOIN run_ids USING (result_set_id)
        ORDER BY
            CASE WHEN selected.status = 'running' THEN 0 ELSE 1 END,
            selected.updated_at DESC
        "#,
    )
    .bind(limit)
    .bind(query.mode.as_deref())
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    let result_sets = rows
        .iter()
        .map(|row| row_to_result_set(row, None))
        .collect::<Result<Vec<_>>>()?;
    let live = result_sets
        .iter()
        .filter(|row| row.mode == "live")
        .cloned()
        .collect();
    let historical = result_sets
        .iter()
        .filter(|row| row.mode == "historical")
        .cloned()
        .collect();
    Ok(ResultSetListResponse {
        result_sets,
        live,
        historical,
    })
}

pub async fn load_result_set(
    pool: &PgPool,
    result_set_id: &str,
    query: ResultSetDetailQuery,
) -> Result<Option<ResultSetDetailResponse>> {
    let row = sqlx::query(
        r#"
        WITH rs AS (
            SELECT result_set_id, mode, status, strategy_suite, start_block, end_block,
                   config::text AS config, metadata::text AS metadata,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   stopped_at::text AS stopped_at
            FROM alpha_trading.backtest_result_sets
            WHERE result_set_id = $1
        ),
        filtered_trades AS (
            SELECT *
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        totals AS (
            SELECT COUNT(*) AS trades,
                   COUNT(DISTINCT strategy_name) AS strategies,
                   COUNT(*) FILTER (
                       WHERE lower(state) NOT IN ('sell_confirmed', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
                   ) AS open_trades,
                   COUNT(*) FILTER (WHERE lower(state) = 'sell_confirmed') AS closed_trades,
                   COUNT(*) FILTER (WHERE lower(state) IN ('buy_failed', 'sell_failed', 'failed')) AS failed_trades,
                   SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))::float8 AS buy_volume_eth,
                   SUM(COALESCE(NULLIF(exit_value_eth, '')::numeric, 0))::float8 AS sell_volume_eth,
                   SUM(COALESCE(NULLIF(current_value_eth, '')::numeric, 0))::float8 AS current_value_eth,
                   SUM(COALESCE(NULLIF(realized_pnl_eth, '')::numeric, 0))::float8 AS realized_pnl_eth,
                   SUM(COALESCE(NULLIF(unrealized_pnl_eth, '')::numeric, 0))::float8 AS unrealized_pnl_eth,
                   SUM(COALESCE(NULLIF(total_pnl_eth, '')::numeric, 0))::float8 AS total_pnl_eth,
                   CASE
                       WHEN SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)) > 0
                       THEN (
                           SUM(COALESCE(NULLIF(total_pnl_eth, '')::numeric, 0))
                           / SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))
                           * 100
                       )::float8
                       ELSE NULL
                   END AS roi_percent
            FROM filtered_trades
        ),
        event_totals AS (
            SELECT COUNT(*) AS reports,
                   COUNT(*) FILTER (WHERE status = 'failed') AS failed
            FROM alpha_trading.trade_events te
            JOIN filtered_trades t ON t.trade_id = te.trade_id
        ),
        risk_totals AS (
            SELECT COUNT(*) AS risks
            FROM alpha_trading.risk_events re
            JOIN (SELECT DISTINCT run_id FROM filtered_trades) runs USING (run_id)
        ),
        run_ids AS (
            SELECT ARRAY_AGG(run_id ORDER BY created_at DESC) AS run_ids
            FROM alpha_trading.backtest_result_set_runs
            WHERE result_set_id = $1
        )
        SELECT rs.*,
               COALESCE(totals.trades, 0) AS trades,
               COALESCE(totals.strategies, 0) AS strategies,
               COALESCE(totals.open_trades, 0) AS open_trades,
               COALESCE(totals.closed_trades, 0) AS closed_trades,
               COALESCE(totals.failed_trades, 0) AS failed_trades,
               totals.buy_volume_eth,
               totals.sell_volume_eth,
               totals.current_value_eth,
               totals.realized_pnl_eth,
               totals.unrealized_pnl_eth,
               totals.total_pnl_eth,
               totals.roi_percent,
               COALESCE(event_totals.reports, 0) AS reports,
               COALESCE(event_totals.failed, 0) AS failed,
               COALESCE(risk_totals.risks, 0) AS risks,
               COALESCE(run_ids.run_ids, ARRAY[]::text[]) AS run_ids
        FROM rs
        CROSS JOIN totals
        CROSS JOIN event_totals
        CROSS JOIN risk_totals
        CROSS JOIN run_ids
        "#,
    )
    .bind(result_set_id)
    .bind(query.strategy_name.as_deref())
    .fetch_optional(pool)
    .await
    .map_err(store_error)?;

    let Some(row) = row else {
        return Ok(None);
    };
    let result_set = row_to_result_set(&row, query.strategy_name.as_deref())?;
    let summary = row_to_result_set_summary(&row)?;
    Ok(Some(ResultSetDetailResponse {
        run: result_set.clone(),
        result_set,
        summary,
    }))
}

pub async fn load_result_set_performance(
    pool: &PgPool,
    result_set_id: &str,
    query: ResultSetPerformanceQuery,
) -> Result<ResultSetPerformanceResponse> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PERFORMANCE_LIMIT)
        .clamp(1, MAX_PERFORMANCE_LIMIT);
    let rows = sqlx::query(
        r#"
        WITH filtered_trades AS (
            SELECT *
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        snapshot_rows AS (
            SELECT ts.*,
                   COALESCE(ts.valuation_block_number, ts.observed_block_number, ts.block_number) AS snapshot_block
            FROM alpha_trading.trade_snapshots ts
            JOIN filtered_trades t ON t.trade_id = ts.trade_id
            WHERE COALESCE(ts.valuation_block_number, ts.observed_block_number, ts.block_number) IS NOT NULL
        ),
        selected_blocks AS (
            SELECT block_number
            FROM (
                SELECT DISTINCT snapshot_block AS block_number
                FROM snapshot_rows
                ORDER BY snapshot_block DESC
                LIMIT $3
            ) recent
        ),
        carried AS (
            SELECT selected_blocks.block_number,
                   filtered_trades.trade_id,
                   COALESCE(NULLIF(filtered_trades.entry_cost_eth, '')::numeric, 0) AS entry_cost_eth,
                   latest.state,
                   COALESCE(NULLIF(latest.current_value_eth, '')::numeric, 0) AS current_value_eth,
                   COALESCE(NULLIF(latest.realized_pnl_eth, '')::numeric, 0) AS realized_pnl_eth,
                   COALESCE(NULLIF(latest.unrealized_pnl_eth, '')::numeric, 0) AS unrealized_pnl_eth,
                   COALESCE(NULLIF(latest.total_pnl_eth, '')::numeric, 0) AS total_pnl_eth,
                   latest.created_at
            FROM selected_blocks
            JOIN filtered_trades ON true
            JOIN LATERAL (
                SELECT snapshot_rows.state,
                       snapshot_rows.current_value_eth,
                       snapshot_rows.realized_pnl_eth,
                       snapshot_rows.unrealized_pnl_eth,
                       snapshot_rows.total_pnl_eth,
                       snapshot_rows.created_at
                FROM snapshot_rows
                WHERE snapshot_rows.trade_id = filtered_trades.trade_id
                  AND snapshot_rows.snapshot_block <= selected_blocks.block_number
                ORDER BY snapshot_rows.snapshot_block DESC, snapshot_rows.id DESC
                LIMIT 1
            ) latest ON true
        )
        SELECT block_number,
               COUNT(*) AS trade_count,
               COUNT(*) FILTER (
                   WHERE lower(state) NOT IN ('sell_confirmed', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
               ) AS open_trades,
               COUNT(*) FILTER (WHERE lower(state) = 'sell_confirmed') AS closed_trades,
               SUM(entry_cost_eth)::float8 AS entry_cost_eth,
               SUM(current_value_eth)::float8 AS current_value_eth,
               SUM(realized_pnl_eth)::float8 AS realized_pnl_eth,
               SUM(unrealized_pnl_eth)::float8 AS unrealized_pnl_eth,
               SUM(total_pnl_eth)::float8 AS total_pnl_eth,
               CASE
                   WHEN SUM(entry_cost_eth) > 0
                   THEN (SUM(total_pnl_eth) / SUM(entry_cost_eth) * 100)::float8
                   ELSE NULL
               END AS roi_percent,
               MAX(created_at)::text AS recorded_at
        FROM carried
        GROUP BY block_number
        ORDER BY block_number ASC
        "#,
    )
    .bind(result_set_id)
    .bind(query.strategy_name.as_deref())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    let points = rows
        .iter()
        .map(row_to_performance_point)
        .collect::<Result<Vec<_>>>()?;
    Ok(ResultSetPerformanceResponse { points })
}

fn row_to_result_set(
    row: &sqlx::postgres::PgRow,
    strategy_name: Option<&str>,
) -> Result<ResultSetView> {
    let run_ids = string_vec(row, "run_ids")?;
    let start_block = optional_int(row, "start_block")?;
    let end_block = optional_int(row, "end_block")?;
    let block_count = match (start_block, end_block) {
        (Some(start), Some(end)) if end >= start => Some(end - start + 1),
        _ => None,
    };
    let config = json_value(row, "config")?;
    let strategy_name = strategy_name
        .map(ToOwned::to_owned)
        .or_else(|| config_string(&config, "strategy_name"));
    let strategy_suite = optional_text(row, "strategy_suite")?;
    Ok(ResultSetView {
        result_set_id: text(row, "result_set_id")?,
        run_id: run_ids.first().cloned(),
        run_ids,
        mode: text(row, "mode")?,
        status: text(row, "status")?,
        strategy_suite: strategy_suite.clone(),
        strategy_id: strategy_name.clone().or(strategy_suite.clone()),
        strategy_label: config_string(&config, "strategy_label").or(strategy_name.clone()),
        strategy_name,
        start_block,
        end_block,
        from_block: start_block,
        to_block: end_block,
        block_count,
        metadata: json_value(row, "metadata")?,
        config,
        created_at: text(row, "created_at")?,
        updated_at: text(row, "updated_at")?,
        stopped_at: optional_text(row, "stopped_at")?,
        trade_count: int(row, "trades")?,
        positions: int(row, "trades")?,
        open_trades: int(row, "open_trades")?,
        open_positions: int(row, "open_trades")?,
        closed_trades: int(row, "closed_trades")?,
        closed_positions: int(row, "closed_trades")?,
        failed_trades: int(row, "failed_trades")?,
        failed_positions: int(row, "failed_trades")?,
        strategy_count: int(row, "strategies")?,
        buy_volume_eth: optional_float(row, "buy_volume_eth")?,
        sell_volume_eth: optional_float(row, "sell_volume_eth")?,
        current_value_eth: optional_float(row, "current_value_eth")?,
        realized_pnl_eth: optional_float(row, "realized_pnl_eth")?,
        unrealized_pnl_eth: optional_float(row, "unrealized_pnl_eth")?,
        total_pnl_eth: optional_float(row, "total_pnl_eth")?,
        roi_percent: optional_float(row, "roi_percent")?,
    })
}

fn row_to_result_set_summary(row: &sqlx::postgres::PgRow) -> Result<ResultSetSummary> {
    Ok(ResultSetSummary {
        positions: int(row, "trades")?,
        open_positions: int(row, "open_trades")?,
        closed_positions: int(row, "closed_trades")?,
        failed_positions: int(row, "failed_trades")?,
        buy_volume_eth: optional_float(row, "buy_volume_eth")?,
        sell_volume_eth: optional_float(row, "sell_volume_eth")?,
        current_value_eth: optional_float(row, "current_value_eth")?,
        realized_pnl_eth: optional_float(row, "realized_pnl_eth")?,
        unrealized_pnl_eth: optional_float(row, "unrealized_pnl_eth")?,
        total_pnl_eth: optional_float(row, "total_pnl_eth")?,
        roi_percent: optional_float(row, "roi_percent")?,
        reports: int(row, "reports")?,
        failed: int(row, "failed")?,
        risks: int(row, "risks")?,
    })
}

fn row_to_performance_point(row: &sqlx::postgres::PgRow) -> Result<ResultSetPerformancePoint> {
    Ok(ResultSetPerformancePoint {
        block_number: int(row, "block_number")?,
        trade_count: int(row, "trade_count")?,
        open_trades: int(row, "open_trades")?,
        closed_trades: int(row, "closed_trades")?,
        entry_cost_eth: optional_float(row, "entry_cost_eth")?,
        current_value_eth: optional_float(row, "current_value_eth")?,
        realized_pnl_eth: optional_float(row, "realized_pnl_eth")?,
        unrealized_pnl_eth: optional_float(row, "unrealized_pnl_eth")?,
        total_pnl_eth: optional_float(row, "total_pnl_eth")?,
        roi_percent: optional_float(row, "roi_percent")?,
        recorded_at: optional_text(row, "recorded_at")?,
    })
}

fn config_string(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_value(row: &sqlx::postgres::PgRow, column: &str) -> Result<Value> {
    serde_json::from_str(&text(row, column)?)
        .map_err(|error| AlphaCoreError::Store(format!("failed to parse {column} json: {error}")))
}

fn text(row: &sqlx::postgres::PgRow, column: &str) -> Result<String> {
    row.try_get::<String, _>(column).map_err(store_error)
}

fn optional_text(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<String>> {
    row.try_get::<Option<String>, _>(column)
        .map_err(store_error)
}

fn int(row: &sqlx::postgres::PgRow, column: &str) -> Result<i64> {
    row.try_get::<i64, _>(column).map_err(store_error)
}

fn optional_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<i64>> {
    row.try_get::<Option<i64>, _>(column).map_err(store_error)
}

fn optional_float(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<f64>> {
    row.try_get::<Option<f64>, _>(column).map_err(store_error)
}

fn string_vec(row: &sqlx::postgres::PgRow, column: &str) -> Result<Vec<String>> {
    row.try_get::<Vec<String>, _>(column).map_err(store_error)
}

fn store_error(error: impl std::fmt::Display) -> AlphaCoreError {
    AlphaCoreError::Store(error.to_string())
}
