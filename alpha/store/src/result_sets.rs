//! Trade-centric result-set read models for alpha backtests and live runs.

use std::collections::HashMap;

use eth_alpha_core::error::{AlphaCoreError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};

const DEFAULT_PERFORMANCE_LIMIT: i64 = 500;
const MAX_PERFORMANCE_LIMIT: i64 = 5_000;
const ESTIMATED_ETH_BLOCK_SECONDS: i64 = 12;

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
    pub elapsed_seconds: Option<i64>,
    pub time_axis_source: Option<String>,
    pub trade_count: i64,
    pub open_trades: i64,
    pub closed_trades: i64,
    pub entry_cost_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub portfolio_value_eth: Option<f64>,
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
    let baseline =
        load_result_set_performance_baseline(pool, result_set_id, query.strategy_name.as_deref())
            .await?;
    let block_rows = sqlx::query(
        r#"
        WITH filtered_trades AS (
            SELECT trade_id
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        selected_blocks AS (
            SELECT DISTINCT
                   COALESCE(ts.valuation_block_number, ts.observed_block_number, ts.block_number) AS block_number
            FROM alpha_trading.trade_snapshots ts
            JOIN filtered_trades t ON t.trade_id = ts.trade_id
            WHERE COALESCE(ts.valuation_block_number, ts.observed_block_number, ts.block_number) IS NOT NULL
        ),
        ranked_blocks AS (
            SELECT block_number,
                   ROW_NUMBER() OVER (ORDER BY block_number ASC) AS rn,
                   COUNT(*) OVER () AS total_blocks
            FROM selected_blocks
        ),
        sampled_blocks AS (
            SELECT block_number
            FROM ranked_blocks
            WHERE total_blocks <= $3
               OR rn = 1
               OR rn = total_blocks
               OR (
                   (rn - 1)
                   % GREATEST(1::bigint, CEIL(total_blocks::numeric / $3::numeric)::bigint)
               ) = 0
        )
        SELECT block_number
        FROM sampled_blocks
        ORDER BY block_number ASC
        "#,
    )
    .bind(result_set_id)
    .bind(query.strategy_name.as_deref())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    let blocks = block_rows
        .iter()
        .map(|row| int(row, "block_number"))
        .collect::<Result<Vec<_>>>()?;
    let timeline_start_block = baseline
        .as_ref()
        .map(|point| point.block_number)
        .or_else(|| blocks.first().copied());
    if blocks.is_empty() {
        let mut baseline = baseline;
        if let Some(point) = baseline.as_mut() {
            apply_elapsed_time(point, timeline_start_block);
        }
        return Ok(ResultSetPerformanceResponse {
            points: baseline.into_iter().collect(),
        });
    }
    let max_block = *blocks.last().unwrap_or(&0);

    let snapshot_rows = sqlx::query(
        r#"
        WITH filtered_trades AS (
            SELECT trade_id,
                   COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)::float8 AS entry_cost_eth
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        )
        SELECT ts.trade_id,
               filtered_trades.entry_cost_eth,
               COALESCE(ts.valuation_block_number, ts.observed_block_number, ts.block_number) AS snapshot_block,
               ts.state,
               COALESCE(NULLIF(ts.current_value_eth, '')::numeric, 0)::float8 AS current_value_eth,
               COALESCE(NULLIF(ts.realized_pnl_eth, '')::numeric, 0)::float8 AS realized_pnl_eth,
               COALESCE(NULLIF(ts.unrealized_pnl_eth, '')::numeric, 0)::float8 AS unrealized_pnl_eth,
               COALESCE(NULLIF(ts.total_pnl_eth, '')::numeric, 0)::float8 AS total_pnl_eth,
               ts.created_at::text AS created_at
        FROM alpha_trading.trade_snapshots ts
        JOIN filtered_trades ON filtered_trades.trade_id = ts.trade_id
        WHERE COALESCE(ts.valuation_block_number, ts.observed_block_number, ts.block_number) IS NOT NULL
          AND COALESCE(ts.valuation_block_number, ts.observed_block_number, ts.block_number) <= $3
        ORDER BY snapshot_block ASC, ts.id ASC
        "#,
    )
    .bind(result_set_id)
    .bind(query.strategy_name.as_deref())
    .bind(max_block)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    let snapshots = snapshot_rows
        .iter()
        .map(row_to_carried_snapshot)
        .collect::<Result<Vec<_>>>()?;
    let mut points = build_performance_points(&blocks, &snapshots, timeline_start_block);
    if let Some(baseline) = baseline {
        let mut baseline = baseline;
        apply_elapsed_time(&mut baseline, timeline_start_block);
        let should_insert = points.first().map_or(true, |point| {
            point.block_number > baseline.block_number
                || (point.block_number == baseline.block_number
                    && point.total_pnl_eth.unwrap_or(0.0).abs() > f64::EPSILON)
        });
        if should_insert {
            points.insert(0, baseline);
        }
    }
    Ok(ResultSetPerformanceResponse { points })
}

#[derive(Debug, Clone)]
struct CarriedSnapshot {
    trade_id: String,
    entry_cost_eth: f64,
    snapshot_block: i64,
    state: String,
    current_value_eth: f64,
    realized_pnl_eth: f64,
    unrealized_pnl_eth: f64,
    total_pnl_eth: f64,
    created_at: Option<String>,
}

async fn load_result_set_performance_baseline(
    pool: &PgPool,
    result_set_id: &str,
    strategy_name: Option<&str>,
) -> Result<Option<ResultSetPerformancePoint>> {
    let row = sqlx::query(
        r#"
        SELECT COALESCE(
                   rs.start_block,
                   MIN(t.entry_block),
                   MIN(COALESCE(
                       ts.valuation_block_number,
                       ts.observed_block_number,
                       ts.block_number
                   ))
               ) AS baseline_block,
               CASE
                   WHEN rs.start_block IS NOT NULL THEN rs.created_at
                   ELSE COALESCE(MIN(t.created_at), MIN(ts.created_at), rs.created_at)
               END::text AS baseline_at
        FROM alpha_trading.backtest_result_sets rs
        LEFT JOIN alpha_trading.trades t
          ON t.result_set_id = rs.result_set_id
         AND ($2::text IS NULL OR t.strategy_name = $2)
        LEFT JOIN alpha_trading.trade_snapshots ts ON ts.trade_id = t.trade_id
        WHERE rs.result_set_id = $1
        GROUP BY rs.result_set_id, rs.start_block, rs.created_at
        "#,
    )
    .bind(result_set_id)
    .bind(strategy_name)
    .fetch_optional(pool)
    .await
    .map_err(store_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let Some(block_number) = optional_int(&row, "baseline_block")? else {
        return Ok(None);
    };
    Ok(Some(ResultSetPerformancePoint {
        block_number,
        elapsed_seconds: None,
        time_axis_source: None,
        trade_count: 0,
        open_trades: 0,
        closed_trades: 0,
        entry_cost_eth: Some(0.0),
        current_value_eth: Some(0.0),
        portfolio_value_eth: Some(0.0),
        realized_pnl_eth: Some(0.0),
        unrealized_pnl_eth: Some(0.0),
        total_pnl_eth: Some(0.0),
        roi_percent: Some(0.0),
        recorded_at: optional_text(&row, "baseline_at")?,
    }))
}

fn apply_elapsed_time(point: &mut ResultSetPerformancePoint, start_block: Option<i64>) {
    if let Some((elapsed_seconds, source)) = elapsed_time_fields(start_block, point.block_number) {
        point.elapsed_seconds = Some(elapsed_seconds);
        point.time_axis_source = Some(source);
    }
}

fn elapsed_time_fields(start_block: Option<i64>, block_number: i64) -> Option<(i64, String)> {
    let start_block = start_block?;
    let elapsed_blocks = block_number.saturating_sub(start_block);
    Some((
        elapsed_blocks * ESTIMATED_ETH_BLOCK_SECONDS,
        "estimated_elapsed_from_blocks".to_string(),
    ))
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

fn row_to_carried_snapshot(row: &sqlx::postgres::PgRow) -> Result<CarriedSnapshot> {
    Ok(CarriedSnapshot {
        trade_id: text(row, "trade_id")?,
        entry_cost_eth: float(row, "entry_cost_eth")?,
        snapshot_block: int(row, "snapshot_block")?,
        state: text(row, "state")?,
        current_value_eth: float(row, "current_value_eth")?,
        realized_pnl_eth: float(row, "realized_pnl_eth")?,
        unrealized_pnl_eth: float(row, "unrealized_pnl_eth")?,
        total_pnl_eth: float(row, "total_pnl_eth")?,
        created_at: optional_text(row, "created_at")?,
    })
}

fn build_performance_points(
    blocks: &[i64],
    snapshots: &[CarriedSnapshot],
    timeline_start_block: Option<i64>,
) -> Vec<ResultSetPerformancePoint> {
    let mut latest_by_trade: HashMap<String, CarriedSnapshot> = HashMap::new();
    let mut next_snapshot = 0_usize;
    let mut points = Vec::with_capacity(blocks.len());

    for block_number in blocks {
        while let Some(snapshot) = snapshots.get(next_snapshot) {
            if snapshot.snapshot_block > *block_number {
                break;
            }
            latest_by_trade.insert(snapshot.trade_id.clone(), snapshot.clone());
            next_snapshot += 1;
        }

        let mut trade_count = 0_i64;
        let mut open_trades = 0_i64;
        let mut closed_trades = 0_i64;
        let mut entry_cost_eth = 0.0_f64;
        let mut current_value_eth = 0.0_f64;
        let mut realized_pnl_eth = 0.0_f64;
        let mut unrealized_pnl_eth = 0.0_f64;
        let mut total_pnl_eth = 0.0_f64;
        let mut recorded_at: Option<String> = None;

        for snapshot in latest_by_trade.values() {
            trade_count += 1;
            if is_closed_trade_state(&snapshot.state) {
                closed_trades += 1;
            } else if is_open_trade_state(&snapshot.state) {
                open_trades += 1;
            }
            entry_cost_eth += snapshot.entry_cost_eth;
            current_value_eth += snapshot.current_value_eth;
            realized_pnl_eth += snapshot.realized_pnl_eth;
            unrealized_pnl_eth += snapshot.unrealized_pnl_eth;
            total_pnl_eth += snapshot.total_pnl_eth;
            if let Some(created_at) = snapshot.created_at.as_deref() {
                if recorded_at
                    .as_deref()
                    .map(|current| created_at > current)
                    .unwrap_or(true)
                {
                    recorded_at = Some(created_at.to_owned());
                }
            }
        }

        let portfolio_value_eth = entry_cost_eth + total_pnl_eth;
        let roi_percent = if entry_cost_eth > 0.0 {
            Some(total_pnl_eth / entry_cost_eth * 100.0)
        } else {
            None
        };
        let (elapsed_seconds, time_axis_source) =
            elapsed_time_fields(timeline_start_block, *block_number)
                .map(|(elapsed_seconds, source)| (Some(elapsed_seconds), Some(source)))
                .unwrap_or((None, None));
        points.push(ResultSetPerformancePoint {
            block_number: *block_number,
            elapsed_seconds,
            time_axis_source,
            trade_count,
            open_trades,
            closed_trades,
            entry_cost_eth: some_if_any(trade_count, entry_cost_eth),
            current_value_eth: some_if_any(trade_count, current_value_eth),
            portfolio_value_eth: some_if_any(trade_count, portfolio_value_eth),
            realized_pnl_eth: some_if_any(trade_count, realized_pnl_eth),
            unrealized_pnl_eth: some_if_any(trade_count, unrealized_pnl_eth),
            total_pnl_eth: some_if_any(trade_count, total_pnl_eth),
            roi_percent,
            recorded_at,
        });
    }

    points
}

fn some_if_any(count: i64, value: f64) -> Option<f64> {
    if count > 0 {
        Some(value)
    } else {
        None
    }
}

fn is_open_trade_state(state: &str) -> bool {
    !matches!(
        state.to_ascii_lowercase().as_str(),
        "sell_confirmed" | "buy_failed" | "buy_cancelled" | "cancelled" | "scammed" | "failed"
    )
}

fn is_closed_trade_state(state: &str) -> bool {
    state.eq_ignore_ascii_case("sell_confirmed")
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

fn float(row: &sqlx::postgres::PgRow, column: &str) -> Result<f64> {
    row.try_get::<f64, _>(column).map_err(store_error)
}

fn string_vec(row: &sqlx::postgres::PgRow, column: &str) -> Result<Vec<String>> {
    row.try_get::<Vec<String>, _>(column).map_err(store_error)
}

fn store_error(error: impl std::fmt::Display) -> AlphaCoreError {
    AlphaCoreError::Store(error.to_string())
}
