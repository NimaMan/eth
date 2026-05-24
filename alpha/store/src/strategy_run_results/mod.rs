//! Trade-centric result-set read models for alpha backtests and live runs.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use eth_alpha_core::error::{AlphaCoreError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};

mod helpers;

use helpers::*;

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
    pub exposure_trades: i64,
    pub exposure_positions: i64,
    pub failed_exit_exposure_trades: i64,
    pub failed_exit_exposure_positions: i64,
    pub strategy_count: i64,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub exposure_entry_cost_eth: Option<f64>,
    pub exposure_current_value_eth: Option<f64>,
    pub exposure_unrealized_pnl_eth: Option<f64>,
    pub exposure_drawdown_eth: Option<f64>,
    pub exposure_roi_percent: Option<f64>,
    pub exposure_to_capital_percent: Option<f64>,
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
    pub exposure_positions: i64,
    pub failed_exit_exposure_positions: i64,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub exposure_entry_cost_eth: Option<f64>,
    pub exposure_current_value_eth: Option<f64>,
    pub exposure_unrealized_pnl_eth: Option<f64>,
    pub exposure_drawdown_eth: Option<f64>,
    pub exposure_roi_percent: Option<f64>,
    pub exposure_to_capital_percent: Option<f64>,
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
    pub protocol_summary: ResultSetProtocolPerformanceSummary,
    pub protocol_series: Vec<ResultSetProtocolPerformanceSeries>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetPerformancePoint {
    pub block_number: i64,
    pub elapsed_seconds: Option<i64>,
    pub time_axis_source: Option<String>,
    pub trade_count: i64,
    pub open_trades: i64,
    pub closed_trades: i64,
    pub exposure_trades: i64,
    pub failed_exit_exposure_trades: i64,
    pub entry_cost_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub portfolio_value_eth: Option<f64>,
    pub exposure_entry_cost_eth: Option<f64>,
    pub exposure_current_value_eth: Option<f64>,
    pub exposure_unrealized_pnl_eth: Option<f64>,
    pub exposure_drawdown_eth: Option<f64>,
    pub exposure_roi_percent: Option<f64>,
    pub exposure_to_entry_percent: Option<f64>,
    pub realized_pnl_eth: Option<f64>,
    pub unrealized_pnl_eth: Option<f64>,
    pub total_pnl_eth: Option<f64>,
    pub roi_percent: Option<f64>,
    pub recorded_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultSetProtocolPerformanceSeries {
    pub protocol: String,
    pub points: Vec<ResultSetPerformancePoint>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ResultSetProtocolPerformanceSummary {
    pub protocol_count: usize,
    pub latest_total_pnl_eth: Option<f64>,
    pub best_protocol: Option<String>,
    pub best_total_pnl_eth: Option<f64>,
    pub worst_protocol: Option<String>,
    pub worst_total_pnl_eth: Option<f64>,
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
                       WHERE lower(state) NOT IN ('sell_confirmed', 'buy_deferred', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
                   ) AS open_trades,
                   COUNT(*) FILTER (WHERE lower(state) = 'sell_confirmed') AS closed_trades,
                   COUNT(*) FILTER (WHERE lower(state) IN ('buy_failed', 'sell_failed', 'failed')) AS failed_trades,
                   COUNT(*) FILTER (
                       WHERE lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                   ) AS exposure_trades,
                   COUNT(*) FILTER (WHERE lower(state) IN ('sell_failed', 'sell_cancelled')) AS failed_exit_exposure_trades,
                   SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))::float8 AS buy_volume_eth,
                   SUM(COALESCE(NULLIF(exit_value_eth, '')::numeric, 0))::float8 AS sell_volume_eth,
                   SUM(COALESCE(NULLIF(current_value_eth, '')::numeric, 0))::float8 AS current_value_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                           ELSE 0
                       END
                   )::float8 AS exposure_entry_cost_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN COALESCE(NULLIF(current_value_eth, '')::numeric, 0)
                           ELSE 0
                       END
                   )::float8 AS exposure_current_value_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN COALESCE(NULLIF(unrealized_pnl_eth, '')::numeric, 0)
                           ELSE 0
                       END
                   )::float8 AS exposure_unrealized_pnl_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN GREATEST(
                               COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                               - COALESCE(NULLIF(current_value_eth, '')::numeric, 0),
                               0
                           )
                           ELSE 0
                       END
                   )::float8 AS exposure_drawdown_eth,
                   CASE
                       WHEN SUM(
                           CASE
                               WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                               THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                               ELSE 0
                           END
                       ) > 0
                       THEN (
                           SUM(
                               CASE
                                   WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                                   THEN COALESCE(NULLIF(unrealized_pnl_eth, '')::numeric, 0)
                                   ELSE 0
                               END
                           )
                           / SUM(
                               CASE
                                   WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                                   THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                                   ELSE 0
                               END
                           )
                           * 100
                       )::float8
                       ELSE NULL
                   END AS exposure_roi_percent,
                   CASE
                       WHEN SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)) > 0
                       THEN (
                           SUM(
                               CASE
                                   WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                                   THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                                   ELSE 0
                               END
                           )
                           / SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))
                           * 100
                       )::float8
                       ELSE NULL
                   END AS exposure_to_capital_percent,
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
               COALESCE(trade_totals.exposure_trades, 0) AS exposure_trades,
               COALESCE(trade_totals.failed_exit_exposure_trades, 0) AS failed_exit_exposure_trades,
               trade_totals.buy_volume_eth,
               trade_totals.sell_volume_eth,
               trade_totals.current_value_eth,
               trade_totals.exposure_entry_cost_eth,
               trade_totals.exposure_current_value_eth,
               trade_totals.exposure_unrealized_pnl_eth,
               trade_totals.exposure_drawdown_eth,
               trade_totals.exposure_roi_percent,
               trade_totals.exposure_to_capital_percent,
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
                       WHERE lower(state) NOT IN ('sell_confirmed', 'buy_deferred', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
                   ) AS open_trades,
                   COUNT(*) FILTER (WHERE lower(state) = 'sell_confirmed') AS closed_trades,
                   COUNT(*) FILTER (WHERE lower(state) IN ('buy_failed', 'sell_failed', 'failed')) AS failed_trades,
                   COUNT(*) FILTER (
                       WHERE lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                   ) AS exposure_trades,
                   COUNT(*) FILTER (WHERE lower(state) IN ('sell_failed', 'sell_cancelled')) AS failed_exit_exposure_trades,
                   SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))::float8 AS buy_volume_eth,
                   SUM(COALESCE(NULLIF(exit_value_eth, '')::numeric, 0))::float8 AS sell_volume_eth,
                   SUM(COALESCE(NULLIF(current_value_eth, '')::numeric, 0))::float8 AS current_value_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                           ELSE 0
                       END
                   )::float8 AS exposure_entry_cost_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN COALESCE(NULLIF(current_value_eth, '')::numeric, 0)
                           ELSE 0
                       END
                   )::float8 AS exposure_current_value_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN COALESCE(NULLIF(unrealized_pnl_eth, '')::numeric, 0)
                           ELSE 0
                       END
                   )::float8 AS exposure_unrealized_pnl_eth,
                   SUM(
                       CASE
                           WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN GREATEST(
                               COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                               - COALESCE(NULLIF(current_value_eth, '')::numeric, 0),
                               0
                           )
                           ELSE 0
                       END
                   )::float8 AS exposure_drawdown_eth,
                   CASE
                       WHEN SUM(
                           CASE
                               WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                               THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                               ELSE 0
                           END
                       ) > 0
                       THEN (
                           SUM(
                               CASE
                                   WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                                   THEN COALESCE(NULLIF(unrealized_pnl_eth, '')::numeric, 0)
                                   ELSE 0
                               END
                           )
                           / SUM(
                               CASE
                                   WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                                   THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                                   ELSE 0
                               END
                           )
                           * 100
                       )::float8
                       ELSE NULL
                   END AS exposure_roi_percent,
                   CASE
                       WHEN SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)) > 0
                       THEN (
                           SUM(
                               CASE
                                   WHEN lower(state) IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                                   THEN COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0)
                                   ELSE 0
                               END
                           )
                           / SUM(COALESCE(NULLIF(entry_cost_eth, '')::numeric, 0))
                           * 100
                       )::float8
                       ELSE NULL
                   END AS exposure_to_capital_percent,
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
               COALESCE(totals.exposure_trades, 0) AS exposure_trades,
               COALESCE(totals.failed_exit_exposure_trades, 0) AS failed_exit_exposure_trades,
               totals.buy_volume_eth,
               totals.sell_volume_eth,
               totals.current_value_eth,
               totals.exposure_entry_cost_eth,
               totals.exposure_current_value_eth,
               totals.exposure_unrealized_pnl_eth,
               totals.exposure_drawdown_eth,
               totals.exposure_roi_percent,
               totals.exposure_to_capital_percent,
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
                   COALESCE(ts.observed_block_number, ts.block_number, ts.valuation_block_number) AS block_number
            FROM alpha_trading.trade_snapshots ts
            JOIN filtered_trades t ON t.trade_id = ts.trade_id
            WHERE COALESCE(ts.observed_block_number, ts.block_number, ts.valuation_block_number) IS NOT NULL
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
    let timeline_start = TimeAxisStart {
        block_number: baseline
            .as_ref()
            .map(|baseline| baseline.point.block_number)
            .or_else(|| blocks.first().copied()),
        epoch_seconds: baseline
            .as_ref()
            .filter(|baseline| baseline.use_recorded_time_axis)
            .and_then(|baseline| baseline.recorded_epoch_seconds),
    };
    if blocks.is_empty() {
        let mut baseline = baseline;
        if let Some(baseline) = baseline.as_mut() {
            apply_elapsed_time(
                &mut baseline.point,
                timeline_start,
                baseline.recorded_epoch_seconds,
            );
        }
        return Ok(ResultSetPerformanceResponse {
            points: baseline
                .into_iter()
                .map(|baseline| baseline.point)
                .collect(),
            protocol_summary: ResultSetProtocolPerformanceSummary::default(),
            protocol_series: Vec::new(),
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
               COALESCE(ts.observed_block_number, ts.block_number, ts.valuation_block_number) AS snapshot_block,
               ts.state,
               COALESCE(NULLIF(ts.current_value_eth, '')::numeric, 0)::float8 AS current_value_eth,
               COALESCE(NULLIF(ts.realized_pnl_eth, '')::numeric, 0)::float8 AS realized_pnl_eth,
               COALESCE(NULLIF(ts.unrealized_pnl_eth, '')::numeric, 0)::float8 AS unrealized_pnl_eth,
               COALESCE(NULLIF(ts.total_pnl_eth, '')::numeric, 0)::float8 AS total_pnl_eth,
               FLOOR(EXTRACT(EPOCH FROM ts.created_at))::BIGINT AS created_epoch_seconds,
               ts.created_at::text AS created_at
        FROM alpha_trading.trade_snapshots ts
        JOIN filtered_trades ON filtered_trades.trade_id = ts.trade_id
        WHERE COALESCE(ts.observed_block_number, ts.block_number, ts.valuation_block_number) IS NOT NULL
          AND COALESCE(ts.observed_block_number, ts.block_number, ts.valuation_block_number) <= $3
        ORDER BY snapshot_block ASC, ts.id ASC
        "#,
    )
    .bind(result_set_id)
    .bind(query.strategy_name.as_deref())
    .bind(max_block)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    let protocols_by_trade =
        load_result_set_trade_protocols(pool, result_set_id, query.strategy_name.as_deref())
            .await?;
    let snapshots = snapshot_rows
        .iter()
        .map(|row| row_to_carried_snapshot(row, &protocols_by_trade))
        .collect::<Result<Vec<_>>>()?;
    let mut points = build_performance_points(&blocks, &snapshots, timeline_start);
    let mut protocol_series =
        build_protocol_performance_series(&blocks, &snapshots, timeline_start);
    if let Some(baseline) = baseline {
        let mut baseline_point = baseline.point;
        apply_elapsed_time(
            &mut baseline_point,
            timeline_start,
            baseline.recorded_epoch_seconds,
        );
        let should_insert = points.first().map_or(true, |point| {
            point.block_number > baseline_point.block_number
                || (point.block_number == baseline_point.block_number
                    && point.total_pnl_eth.unwrap_or(0.0).abs() > f64::EPSILON)
        });
        if should_insert {
            points.insert(0, baseline_point.clone());
        }
        insert_protocol_baseline_points(&mut protocol_series, &baseline_point);
    }
    let protocol_summary = build_protocol_performance_summary(&protocol_series);
    Ok(ResultSetPerformanceResponse {
        points,
        protocol_summary,
        protocol_series,
    })
}

#[derive(Debug, Clone)]
struct CarriedSnapshot {
    trade_id: String,
    protocol: String,
    entry_cost_eth: f64,
    snapshot_block: i64,
    state: String,
    current_value_eth: f64,
    realized_pnl_eth: f64,
    unrealized_pnl_eth: f64,
    total_pnl_eth: f64,
    created_epoch_seconds: Option<i64>,
    created_at: Option<String>,
}

#[derive(Debug, Clone)]
struct BaselinePerformancePoint {
    point: ResultSetPerformancePoint,
    recorded_epoch_seconds: Option<i64>,
    use_recorded_time_axis: bool,
}

#[derive(Debug, Clone, Copy)]
struct TimeAxisStart {
    block_number: Option<i64>,
    epoch_seconds: Option<i64>,
}

async fn load_result_set_trade_protocols(
    pool: &PgPool,
    result_set_id: &str,
    strategy_name: Option<&str>,
) -> Result<HashMap<String, String>> {
    let has_risk_atlas: bool =
        sqlx::query_scalar("SELECT to_regclass('public.risk_atlas_pool_eligibility') IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(store_error)?;

    let rows = if has_risk_atlas {
        sqlx::query(
            r#"
            WITH protocol_lookup AS (
                SELECT DISTINCT ON (lower(token_address), lower(pool_address))
                       lower(token_address) AS token_key,
                       lower(pool_address) AS pool_key,
                       protocol
                FROM public.risk_atlas_pool_eligibility
                WHERE protocol IS NOT NULL
                ORDER BY
                    lower(token_address),
                    lower(pool_address),
                    last_observed_block DESC NULLS LAST,
                    first_observed_block DESC NULLS LAST
            )
            SELECT t.trade_id,
                   COALESCE(
                       NULLIF(t.protocol, ''),
                       NULLIF(t.payload->>'protocol', ''),
                       NULLIF(t.payload #>> '{pool,protocol}', ''),
                       NULLIF(t.payload #>> '{key,protocol}', ''),
                       protocol_lookup.protocol,
                       'unknown'
                   ) AS protocol
            FROM alpha_trading.trades t
            LEFT JOIN protocol_lookup
              ON protocol_lookup.token_key = lower(t.token_address)
             AND protocol_lookup.pool_key = lower(
                CASE
                    WHEN strpos(t.pool_address, ':') > 0 THEN split_part(t.pool_address, ':', 2)
                    ELSE t.pool_address
                END
             )
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
            "#,
        )
        .bind(result_set_id)
        .bind(strategy_name)
        .fetch_all(pool)
        .await
        .map_err(store_error)?
    } else {
        sqlx::query(
            r#"
            SELECT t.trade_id,
                   COALESCE(
                       NULLIF(t.protocol, ''),
                       NULLIF(t.payload->>'protocol', ''),
                       NULLIF(t.payload #>> '{pool,protocol}', ''),
                       NULLIF(t.payload #>> '{key,protocol}', ''),
                       'unknown'
                   ) AS protocol
            FROM alpha_trading.trades t
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
            "#,
        )
        .bind(result_set_id)
        .bind(strategy_name)
        .fetch_all(pool)
        .await
        .map_err(store_error)?
    };

    rows.iter()
        .map(|row| {
            Ok((
                text(row, "trade_id")?,
                normalize_protocol_label(&text(row, "protocol")?),
            ))
        })
        .collect()
}

async fn load_result_set_performance_baseline(
    pool: &PgPool,
    result_set_id: &str,
    strategy_name: Option<&str>,
) -> Result<Option<BaselinePerformancePoint>> {
    let row = sqlx::query(
        r#"
        WITH baseline AS (
            SELECT COALESCE(
                       rs.start_block,
                       LEAST(
                           COALESCE(
                               MIN(t.entry_block),
                               MIN(COALESCE(
                                   ts.observed_block_number,
                                   ts.block_number,
                                   ts.valuation_block_number
                               ))
                           ),
                           COALESCE(
                               MIN(COALESCE(
                                   ts.observed_block_number,
                                   ts.block_number,
                                   ts.valuation_block_number
                               )),
                               MIN(t.entry_block)
                           )
                       )
                   ) AS baseline_block,
                   CASE
                       WHEN rs.start_block IS NOT NULL THEN rs.created_at
                       ELSE COALESCE(MIN(t.created_at), MIN(ts.created_at), rs.created_at)
                   END AS baseline_at,
                   LOWER(rs.mode) = 'live' AS use_recorded_time_axis
            FROM alpha_trading.backtest_result_sets rs
            LEFT JOIN alpha_trading.trades t
              ON t.result_set_id = rs.result_set_id
             AND ($2::text IS NULL OR t.strategy_name = $2)
            LEFT JOIN alpha_trading.trade_snapshots ts ON ts.trade_id = t.trade_id
            WHERE rs.result_set_id = $1
            GROUP BY rs.result_set_id, rs.start_block, rs.created_at
        )
        SELECT baseline_block,
               baseline_at::text AS baseline_at,
               FLOOR(EXTRACT(EPOCH FROM baseline_at))::BIGINT AS baseline_epoch_seconds,
               use_recorded_time_axis
        FROM baseline
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
    Ok(Some(BaselinePerformancePoint {
        point: ResultSetPerformancePoint {
            block_number,
            elapsed_seconds: None,
            time_axis_source: None,
            trade_count: 0,
            open_trades: 0,
            closed_trades: 0,
            exposure_trades: 0,
            failed_exit_exposure_trades: 0,
            entry_cost_eth: Some(0.0),
            current_value_eth: Some(0.0),
            portfolio_value_eth: Some(0.0),
            exposure_entry_cost_eth: Some(0.0),
            exposure_current_value_eth: Some(0.0),
            exposure_unrealized_pnl_eth: Some(0.0),
            exposure_drawdown_eth: Some(0.0),
            exposure_roi_percent: Some(0.0),
            exposure_to_entry_percent: Some(0.0),
            realized_pnl_eth: Some(0.0),
            unrealized_pnl_eth: Some(0.0),
            total_pnl_eth: Some(0.0),
            roi_percent: Some(0.0),
            recorded_at: optional_text(&row, "baseline_at")?,
        },
        recorded_epoch_seconds: optional_int(&row, "baseline_epoch_seconds")?,
        use_recorded_time_axis: bool_value(&row, "use_recorded_time_axis")?,
    }))
}
