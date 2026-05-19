//! Strategy performance rollups for the alpha trading dashboard.

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use eth_alpha_core::error::{AlphaCoreError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

mod rollups;

use rollups::*;

const DEFAULT_BUCKET_SECS: u64 = 300;
const MIN_BUCKET_SECS: u64 = 60;
const MAX_BUCKET_SECS: u64 = 3_600;
const DEFAULT_POOL_LIMIT: u64 = 100;
const MAX_POOL_LIMIT: u64 = 500;
const DEFAULT_BUCKET_LIMIT: u64 = 180;
const MAX_BUCKET_LIMIT: u64 = 500;
const POSITION_ROW_LIMIT: i64 = 2_000;
const EVENT_ROW_LIMIT: i64 = 5_000;
const SNAPSHOT_ROW_LIMIT: i64 = 5_000;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct StrategyPerformanceQuery {
    pub bucket_secs: Option<u64>,
    pub pool_limit: Option<u64>,
    pub bucket_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyPerformanceReport {
    pub strategy_id: String,
    pub run_id: String,
    pub generated_at_unix_secs: u64,
    pub bucket_secs: u64,
    pub caveats: Vec<String>,
    pub totals: StrategyPerformanceTotals,
    pub timeline: Vec<StrategyPerformancePoint>,
    pub pools: Vec<StrategyPoolPerformance>,
    pub state_breakdown: Vec<StrategyBreakdown>,
    pub risk_breakdown: Vec<StrategyBreakdown>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyPerformanceTotals {
    pub positions: u64,
    pub open_positions: u64,
    pub closed_positions: u64,
    pub buy_orders: u64,
    pub sell_orders: u64,
    pub execution_reports: u64,
    pub confirmed_reports: u64,
    pub failed_reports: u64,
    pub confirmed_buys: u64,
    pub confirmed_sells: u64,
    pub risk_events: u64,
    pub critical_risk_events: u64,
    pub exposure_positions: u64,
    pub failed_exit_exposure_positions: u64,
    pub capital_deployed_eth: Option<f64>,
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
    pub pools_with_snapshots: u64,
    pub pools_with_pnl: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyPerformancePoint {
    pub bucket_start_unix_secs: u64,
    pub buy_orders: u64,
    pub sell_orders: u64,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub execution_reports: u64,
    pub confirmed_reports: u64,
    pub failed_reports: u64,
    pub confirmed_buys: u64,
    pub confirmed_sells: u64,
    pub opened_positions: u64,
    pub closed_positions: u64,
    pub risk_events: u64,
    pub critical_risk_events: u64,
    pub realized_pnl_eth: Option<f64>,
    pub unrealized_pnl_eth: Option<f64>,
    pub total_pnl_eth: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyPoolPerformance {
    pub token_address: String,
    pub pool_address: String,
    pub protocol: Option<String>,
    pub state: String,
    pub entry_order_id: Option<String>,
    pub exit_order_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub duration_seconds: Option<u64>,
    pub buy_order_count: u64,
    pub sell_order_count: u64,
    pub execution_report_count: u64,
    pub confirmed_report_count: u64,
    pub failed_report_count: u64,
    pub confirmed_buy_count: u64,
    pub confirmed_sell_count: u64,
    pub risk_event_count: u64,
    pub critical_risk_count: u64,
    pub latest_risk_kind: Option<String>,
    pub latest_risk_at: Option<String>,
    pub has_exposure: bool,
    pub capital_deployed_eth: Option<f64>,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
    pub exposure_entry_cost_eth: Option<f64>,
    pub exposure_current_value_eth: Option<f64>,
    pub exposure_unrealized_pnl_eth: Option<f64>,
    pub exposure_drawdown_eth: Option<f64>,
    pub exposure_roi_percent: Option<f64>,
    pub realized_pnl_eth: Option<f64>,
    pub unrealized_pnl_eth: Option<f64>,
    pub total_pnl_eth: Option<f64>,
    pub roi_percent: Option<f64>,
    pub has_snapshot: bool,
    pub has_pnl: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyBreakdown {
    pub key: String,
    pub count: u64,
    pub share_percent: f64,
}

pub async fn load_strategy_performance(
    pool: &PgPool,
    strategy_id: &str,
    run_id: &str,
    query: StrategyPerformanceQuery,
) -> Result<StrategyPerformanceReport> {
    let generated_at_unix_secs = unix_now_secs();
    let bucket_secs = query
        .bucket_secs
        .unwrap_or(DEFAULT_BUCKET_SECS)
        .clamp(MIN_BUCKET_SECS, MAX_BUCKET_SECS);
    let pool_limit = query
        .pool_limit
        .unwrap_or(DEFAULT_POOL_LIMIT)
        .clamp(1, MAX_POOL_LIMIT);
    let bucket_limit = query
        .bucket_limit
        .unwrap_or(DEFAULT_BUCKET_LIMIT)
        .clamp(1, MAX_BUCKET_LIMIT);

    let positions = load_positions(pool, run_id, strategy_id).await?;
    let orders = load_orders(pool, run_id, strategy_id).await?;
    let reports = load_execution_reports(pool, run_id, strategy_id).await?;
    let risks = load_risk_events(pool, run_id).await?;
    let snapshots = load_latest_snapshots(pool, run_id, strategy_id).await?;

    let mut caveats = Vec::new();
    if positions.len() as i64 == POSITION_ROW_LIMIT {
        caveats.push(format!(
            "Per-pool rollup is capped at {POSITION_ROW_LIMIT} recent positions."
        ));
    }
    if orders.len() as i64 == EVENT_ROW_LIMIT {
        caveats.push(format!(
            "Order timeline and volume rollups are capped at {EVENT_ROW_LIMIT} recent orders."
        ));
    }
    if reports.len() as i64 == EVENT_ROW_LIMIT {
        caveats.push(format!(
            "Execution-report rollups are capped at {EVENT_ROW_LIMIT} recent reports."
        ));
    }
    if risks.len() as i64 == EVENT_ROW_LIMIT {
        caveats.push(format!(
            "Risk rollups are capped at {EVENT_ROW_LIMIT} recent risk events."
        ));
    }
    if snapshots.len() as i64 == SNAPSHOT_ROW_LIMIT {
        caveats.push(format!(
            "PnL rollup is capped at {SNAPSHOT_ROW_LIMIT} latest position snapshots."
        ));
    }

    let report_by_order = reports_by_order(&reports);
    let snapshot_by_position = snapshots_by_position(snapshots);
    let mut pools = build_pool_rollups(
        &positions,
        &orders,
        &risks,
        &report_by_order,
        &snapshot_by_position,
        generated_at_unix_secs,
    );
    pools.sort_by(compare_pool_rollups);

    let state_breakdown = breakdown(positions.iter().map(|position| position.state.as_str()));
    let risk_breakdown = breakdown(risks.iter().map(|risk| risk.kind.as_str()));
    let totals = build_totals(&positions, &orders, &reports, &risks, &pools);
    let timeline = build_timeline(
        &positions,
        &orders,
        &reports,
        &risks,
        &report_by_order,
        bucket_secs,
        bucket_limit,
    );

    let pools = pools
        .into_iter()
        .take(pool_limit as usize)
        .map(PoolRollup::into_view)
        .collect();

    Ok(StrategyPerformanceReport {
        strategy_id: strategy_id.to_string(),
        run_id: run_id.to_string(),
        generated_at_unix_secs,
        bucket_secs,
        caveats,
        totals,
        timeline,
        pools,
        state_breakdown,
        risk_breakdown,
    })
}

#[derive(Debug, Clone)]
struct PositionRecord {
    position_id: String,
    token_address: String,
    pool_address: String,
    protocol: Option<String>,
    state: String,
    entry_order_id: Option<String>,
    exit_order_id: Option<String>,
    created_at: String,
    updated_at: String,
    created_epoch: i64,
    updated_epoch: i64,
}

#[derive(Debug, Clone)]
struct OrderRecord {
    side: String,
    token_address: String,
    pool_address: String,
    protocol: Option<String>,
    created_epoch: i64,
}

#[derive(Debug, Clone)]
struct ExecutionReportRecord {
    order_id: String,
    status: String,
    filled_amount_raw: Option<String>,
    filled_amount_decimals: Option<i16>,
    created_epoch: i64,
}

#[derive(Debug, Clone)]
struct RiskRecord {
    kind: String,
    severity: String,
    token_address: String,
    pool_address: Option<String>,
    created_at: String,
    created_epoch: i64,
}

#[derive(Debug, Clone)]
struct SnapshotRecord {
    position_id: String,
    current_value_eth: Option<f64>,
    realized_profit_eth: Option<f64>,
    unrealized_profit_eth: Option<f64>,
}

#[derive(Debug, Default)]
struct PoolRollup {
    token_address: String,
    pool_address: String,
    protocol: Option<String>,
    state: String,
    entry_order_id: Option<String>,
    exit_order_id: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    created_epoch: Option<i64>,
    updated_epoch: Option<i64>,
    buy_order_count: u64,
    sell_order_count: u64,
    execution_report_count: u64,
    confirmed_report_count: u64,
    failed_report_count: u64,
    confirmed_buy_count: u64,
    confirmed_sell_count: u64,
    risk_event_count: u64,
    critical_risk_count: u64,
    latest_risk_kind: Option<String>,
    latest_risk_at: Option<String>,
    latest_risk_epoch: Option<i64>,
    has_exposure: bool,
    capital_deployed_eth: f64,
    buy_volume_eth: f64,
    sell_volume_eth: f64,
    current_value_eth: f64,
    exposure_entry_cost_eth: f64,
    exposure_current_value_eth: f64,
    exposure_unrealized_pnl_eth: f64,
    realized_pnl_eth: f64,
    unrealized_pnl_eth: f64,
    pnl_observation_count: u64,
    snapshot_count: u64,
}

impl PoolRollup {
    fn position(position: &PositionRecord) -> Self {
        Self {
            token_address: position.token_address.clone(),
            pool_address: position.pool_address.clone(),
            protocol: position.protocol.clone(),
            state: position.state.clone(),
            has_exposure: is_exposure_state(&position.state),
            entry_order_id: position.entry_order_id.clone(),
            exit_order_id: position.exit_order_id.clone(),
            created_at: Some(position.created_at.clone()),
            updated_at: Some(position.updated_at.clone()),
            created_epoch: Some(position.created_epoch),
            updated_epoch: Some(position.updated_epoch),
            ..Self::default()
        }
    }

    fn order_only(order: &OrderRecord) -> Self {
        Self {
            token_address: order.token_address.clone(),
            pool_address: order.pool_address.clone(),
            protocol: order.protocol.clone(),
            state: "order_only".to_string(),
            created_epoch: Some(order.created_epoch),
            updated_epoch: Some(order.created_epoch),
            ..Self::default()
        }
    }

    fn risk_only(risk: &RiskRecord) -> Self {
        Self {
            token_address: risk.token_address.clone(),
            pool_address: risk.pool_address.clone().unwrap_or_default(),
            state: "risk_only".to_string(),
            created_at: Some(risk.created_at.clone()),
            updated_at: Some(risk.created_at.clone()),
            created_epoch: Some(risk.created_epoch),
            updated_epoch: Some(risk.created_epoch),
            ..Self::default()
        }
    }

    fn add_order(&mut self, order: &OrderRecord) {
        match order.side.as_str() {
            "buy" => {
                self.buy_order_count += 1;
            }
            "sell" => {
                self.sell_order_count += 1;
            }
            _ => {}
        }
        self.bump_epochs(order.created_epoch, None);
    }

    fn add_risk(&mut self, risk: &RiskRecord) {
        self.risk_event_count += 1;
        if risk.severity == "critical" {
            self.critical_risk_count += 1;
        }
        if self
            .latest_risk_epoch
            .map(|epoch| risk.created_epoch >= epoch)
            .unwrap_or(true)
        {
            self.latest_risk_kind = Some(risk.kind.clone());
            self.latest_risk_at = Some(risk.created_at.clone());
            self.latest_risk_epoch = Some(risk.created_epoch);
        }
        self.bump_epochs(risk.created_epoch, None);
    }

    fn add_reports(&mut self, reports: &[ExecutionReportRecord], side: ReportSide) -> f64 {
        let mut confirmed_filled_eth = 0.0;
        for report in reports {
            self.execution_report_count += 1;
            if report.status == "confirmed" {
                self.confirmed_report_count += 1;
                let filled_eth = report.filled_amount_eth().unwrap_or(0.0);
                confirmed_filled_eth += filled_eth;
                match side {
                    ReportSide::Buy => {
                        self.confirmed_buy_count += 1;
                        self.buy_volume_eth += filled_eth;
                        self.capital_deployed_eth += filled_eth;
                    }
                    ReportSide::Sell => {
                        self.confirmed_sell_count += 1;
                        self.sell_volume_eth += filled_eth;
                    }
                }
            } else if is_failed_report_status(&report.status) {
                self.failed_report_count += 1;
            }
            self.bump_epochs(report.created_epoch, None);
        }
        confirmed_filled_eth
    }

    fn add_snapshot(&mut self, snapshot: &SnapshotRecord) {
        self.snapshot_count += 1;
        self.current_value_eth += snapshot.current_value_eth.unwrap_or(0.0);
        self.realized_pnl_eth += snapshot.realized_profit_eth.unwrap_or(0.0);
        self.unrealized_pnl_eth += snapshot.unrealized_profit_eth.unwrap_or(0.0);
        self.pnl_observation_count += 1;
    }

    fn add_realized_pnl(&mut self, entry_value_eth: f64, exit_value_eth: f64) {
        self.realized_pnl_eth += exit_value_eth - entry_value_eth;
        self.current_value_eth = 0.0;
        self.pnl_observation_count += 1;
    }

    fn bump_epochs(&mut self, epoch: i64, created_label: Option<String>) {
        if self
            .created_epoch
            .map(|value| epoch < value)
            .unwrap_or(true)
        {
            self.created_epoch = Some(epoch);
            if created_label.is_some() {
                self.created_at = created_label;
            }
        }
        if self
            .updated_epoch
            .map(|value| epoch > value)
            .unwrap_or(true)
        {
            self.updated_epoch = Some(epoch);
        }
    }

    fn total_pnl_eth(&self) -> Option<f64> {
        if self.pnl_observation_count == 0 {
            None
        } else {
            Some(self.realized_pnl_eth + self.unrealized_pnl_eth)
        }
    }

    fn roi_percent(&self) -> Option<f64> {
        let total_pnl = self.total_pnl_eth()?;
        if self.capital_deployed_eth > 0.0 {
            Some((total_pnl / self.capital_deployed_eth) * 100.0)
        } else {
            None
        }
    }

    fn exposure_drawdown_eth(&self) -> Option<f64> {
        if self.has_exposure {
            Some((self.exposure_entry_cost_eth - self.exposure_current_value_eth).max(0.0))
        } else {
            None
        }
    }

    fn exposure_roi_percent(&self) -> Option<f64> {
        if self.has_exposure && self.exposure_entry_cost_eth > 0.0 {
            Some((self.exposure_unrealized_pnl_eth / self.exposure_entry_cost_eth) * 100.0)
        } else {
            None
        }
    }

    fn into_view(self) -> StrategyPoolPerformance {
        let duration_seconds = match (self.created_epoch, self.updated_epoch) {
            (Some(created), Some(updated)) if updated >= created => {
                Some((updated - created) as u64)
            }
            _ => None,
        };
        let total_pnl_eth = self.total_pnl_eth();
        let roi_percent = self.roi_percent();
        let exposure_drawdown_eth = self.exposure_drawdown_eth();
        let exposure_roi_percent = self.exposure_roi_percent();
        StrategyPoolPerformance {
            token_address: self.token_address,
            pool_address: self.pool_address,
            protocol: self.protocol,
            state: self.state,
            entry_order_id: self.entry_order_id,
            exit_order_id: self.exit_order_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            duration_seconds,
            buy_order_count: self.buy_order_count,
            sell_order_count: self.sell_order_count,
            execution_report_count: self.execution_report_count,
            confirmed_report_count: self.confirmed_report_count,
            failed_report_count: self.failed_report_count,
            confirmed_buy_count: self.confirmed_buy_count,
            confirmed_sell_count: self.confirmed_sell_count,
            risk_event_count: self.risk_event_count,
            critical_risk_count: self.critical_risk_count,
            latest_risk_kind: self.latest_risk_kind,
            latest_risk_at: self.latest_risk_at,
            has_exposure: self.has_exposure,
            capital_deployed_eth: positive_or_none(self.capital_deployed_eth),
            buy_volume_eth: positive_or_none(self.buy_volume_eth),
            sell_volume_eth: positive_or_none(self.sell_volume_eth),
            current_value_eth: if self.snapshot_count == 0 {
                None
            } else {
                Some(self.current_value_eth)
            },
            exposure_entry_cost_eth: if self.has_exposure {
                Some(self.exposure_entry_cost_eth)
            } else {
                None
            },
            exposure_current_value_eth: if self.has_exposure {
                Some(self.exposure_current_value_eth)
            } else {
                None
            },
            exposure_unrealized_pnl_eth: if self.has_exposure {
                Some(self.exposure_unrealized_pnl_eth)
            } else {
                None
            },
            exposure_drawdown_eth,
            exposure_roi_percent,
            realized_pnl_eth: if self.pnl_observation_count == 0 {
                None
            } else {
                Some(self.realized_pnl_eth)
            },
            unrealized_pnl_eth: if self.snapshot_count == 0 {
                None
            } else {
                Some(self.unrealized_pnl_eth)
            },
            total_pnl_eth,
            roi_percent,
            has_snapshot: self.snapshot_count > 0,
            has_pnl: self.pnl_observation_count > 0,
        }
    }
}

impl ExecutionReportRecord {
    fn filled_amount_eth(&self) -> Option<f64> {
        amount_to_f64(
            self.filled_amount_raw.as_deref()?,
            self.filled_amount_decimals?,
        )
    }
}

#[derive(Clone, Copy)]
enum ReportSide {
    Buy,
    Sell,
}

async fn load_positions(
    pool: &PgPool,
    run_id: &str,
    strategy_id: &str,
) -> Result<Vec<PositionRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT position_id, token_address, pool_address, protocol, state, entry_order_id,
               exit_order_id, created_at::text AS created_at, updated_at::text AS updated_at,
               EXTRACT(EPOCH FROM created_at)::BIGINT AS created_epoch,
               EXTRACT(EPOCH FROM updated_at)::BIGINT AS updated_epoch
        FROM alpha_trading.positions
        WHERE run_id = $1 AND strategy_name = $2
        ORDER BY updated_at DESC
        LIMIT $3
        "#,
    )
    .bind(run_id)
    .bind(strategy_id)
    .bind(POSITION_ROW_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    rows.iter()
        .map(|row| {
            Ok(PositionRecord {
                position_id: row.try_get("position_id").map_err(store_error)?,
                token_address: row.try_get("token_address").map_err(store_error)?,
                pool_address: row.try_get("pool_address").map_err(store_error)?,
                protocol: row.try_get("protocol").map_err(store_error)?,
                state: row.try_get("state").map_err(store_error)?,
                entry_order_id: row.try_get("entry_order_id").map_err(store_error)?,
                exit_order_id: row.try_get("exit_order_id").map_err(store_error)?,
                created_at: row.try_get("created_at").map_err(store_error)?,
                updated_at: row.try_get("updated_at").map_err(store_error)?,
                created_epoch: row.try_get("created_epoch").map_err(store_error)?,
                updated_epoch: row.try_get("updated_epoch").map_err(store_error)?,
            })
        })
        .collect()
}

async fn load_orders(pool: &PgPool, run_id: &str, strategy_id: &str) -> Result<Vec<OrderRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT side, token_address, pool_address, protocol,
               EXTRACT(EPOCH FROM created_at)::BIGINT AS created_epoch
        FROM alpha_trading.order_intents
        WHERE run_id = $1 AND strategy_name = $2
        ORDER BY created_at DESC
        LIMIT $3
        "#,
    )
    .bind(run_id)
    .bind(strategy_id)
    .bind(EVENT_ROW_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    rows.iter()
        .map(|row| {
            Ok(OrderRecord {
                side: row.try_get("side").map_err(store_error)?,
                token_address: row.try_get("token_address").map_err(store_error)?,
                pool_address: row.try_get("pool_address").map_err(store_error)?,
                protocol: row.try_get("protocol").map_err(store_error)?,
                created_epoch: row.try_get("created_epoch").map_err(store_error)?,
            })
        })
        .collect()
}

async fn load_execution_reports(
    pool: &PgPool,
    run_id: &str,
    strategy_id: &str,
) -> Result<Vec<ExecutionReportRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT order_id, status, filled_amount_raw, filled_amount_decimals,
               EXTRACT(EPOCH FROM created_at)::BIGINT AS created_epoch
        FROM alpha_trading.execution_reports
        WHERE run_id = $1
          AND EXISTS (
              SELECT 1
              FROM alpha_trading.positions positions
              WHERE positions.run_id = $1
                AND positions.strategy_name = $2
                AND (
                    positions.position_id = execution_reports.position_id
                    OR
                    positions.entry_order_id = execution_reports.order_id
                    OR positions.exit_order_id = execution_reports.order_id
                )
          )
        ORDER BY created_at DESC
        LIMIT $3
        "#,
    )
    .bind(run_id)
    .bind(strategy_id)
    .bind(EVENT_ROW_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    rows.iter()
        .map(|row| {
            Ok(ExecutionReportRecord {
                order_id: row.try_get("order_id").map_err(store_error)?,
                status: row.try_get("status").map_err(store_error)?,
                filled_amount_raw: row.try_get("filled_amount_raw").map_err(store_error)?,
                filled_amount_decimals: row
                    .try_get("filled_amount_decimals")
                    .map_err(store_error)?,
                created_epoch: row.try_get("created_epoch").map_err(store_error)?,
            })
        })
        .collect()
}

async fn load_risk_events(pool: &PgPool, run_id: &str) -> Result<Vec<RiskRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT kind, severity, token_address, pool_address, created_at::text AS created_at,
               EXTRACT(EPOCH FROM created_at)::BIGINT AS created_epoch
        FROM alpha_trading.risk_events
        WHERE run_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(run_id)
    .bind(EVENT_ROW_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    rows.iter()
        .map(|row| {
            Ok(RiskRecord {
                kind: row.try_get("kind").map_err(store_error)?,
                severity: row.try_get("severity").map_err(store_error)?,
                token_address: row.try_get("token_address").map_err(store_error)?,
                pool_address: row.try_get("pool_address").map_err(store_error)?,
                created_at: row.try_get("created_at").map_err(store_error)?,
                created_epoch: row.try_get("created_epoch").map_err(store_error)?,
            })
        })
        .collect()
}

async fn load_latest_snapshots(
    pool: &PgPool,
    run_id: &str,
    strategy_id: &str,
) -> Result<Vec<SnapshotRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT ON (snapshots.position_id)
               snapshots.position_id,
               snapshots.current_value_eth,
               snapshots.realized_profit_eth,
               snapshots.unrealized_profit_eth
        FROM alpha_trading.position_snapshots snapshots
        INNER JOIN alpha_trading.positions positions
            ON positions.run_id = snapshots.run_id
           AND positions.position_id = snapshots.position_id
        WHERE snapshots.run_id = $1 AND positions.strategy_name = $2
        ORDER BY snapshots.position_id, snapshots.block_number DESC, snapshots.id DESC
        LIMIT $3
        "#,
    )
    .bind(run_id)
    .bind(strategy_id)
    .bind(SNAPSHOT_ROW_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(store_error)?;

    rows.iter()
        .map(|row| {
            Ok(SnapshotRecord {
                position_id: row.try_get("position_id").map_err(store_error)?,
                current_value_eth: decimal_text(row, "current_value_eth")?,
                realized_profit_eth: decimal_text(row, "realized_profit_eth")?,
                unrealized_profit_eth: decimal_text(row, "unrealized_profit_eth")?,
            })
        })
        .collect()
}
