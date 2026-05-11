//! Strategy performance rollups for the alpha trading dashboard.

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use eth_alpha_core::error::{AlphaCoreError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

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
    pub capital_deployed_eth: Option<f64>,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
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
    pub capital_deployed_eth: Option<f64>,
    pub buy_volume_eth: Option<f64>,
    pub sell_volume_eth: Option<f64>,
    pub current_value_eth: Option<f64>,
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
    capital_deployed_eth: f64,
    buy_volume_eth: f64,
    sell_volume_eth: f64,
    current_value_eth: f64,
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
            state: position.state.clone(),
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

    fn into_view(self) -> StrategyPoolPerformance {
        let duration_seconds = match (self.created_epoch, self.updated_epoch) {
            (Some(created), Some(updated)) if updated >= created => {
                Some((updated - created) as u64)
            }
            _ => None,
        };
        let total_pnl_eth = self.total_pnl_eth();
        let roi_percent = self.roi_percent();
        StrategyPoolPerformance {
            token_address: self.token_address,
            pool_address: self.pool_address,
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
            capital_deployed_eth: positive_or_none(self.capital_deployed_eth),
            buy_volume_eth: positive_or_none(self.buy_volume_eth),
            sell_volume_eth: positive_or_none(self.sell_volume_eth),
            current_value_eth: if self.snapshot_count == 0 {
                None
            } else {
                Some(self.current_value_eth)
            },
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
        SELECT position_id, token_address, pool_address, state, entry_order_id,
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
        SELECT side, token_address, pool_address,
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

fn build_pool_rollups(
    positions: &[PositionRecord],
    orders: &[OrderRecord],
    risks: &[RiskRecord],
    report_by_order: &HashMap<String, Vec<ExecutionReportRecord>>,
    snapshot_by_position: &HashMap<String, SnapshotRecord>,
    generated_at_unix_secs: u64,
) -> Vec<PoolRollup> {
    let mut pools: HashMap<(String, String), PoolRollup> = HashMap::new();
    let mut token_to_pool_keys: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for position in positions {
        let key = pool_key(&position.token_address, &position.pool_address);
        token_to_pool_keys
            .entry(key.0.clone())
            .or_default()
            .push(key.clone());
        let rollup = pools
            .entry(key)
            .or_insert_with(|| PoolRollup::position(position));
        if is_terminal_state(&position.state) {
            rollup.updated_epoch = Some(position.updated_epoch);
        } else {
            rollup.updated_epoch = Some(generated_at_unix_secs as i64);
        }
        if let Some(order_id) = position.entry_order_id.as_deref() {
            if let Some(reports) = report_by_order.get(order_id) {
                let entry_value_eth = rollup.add_reports(reports, ReportSide::Buy);
                if let Some(exit_order_id) = position.exit_order_id.as_deref() {
                    if let Some(exit_reports) = report_by_order.get(exit_order_id) {
                        let exit_value_eth = confirmed_filled_eth(exit_reports);
                        if entry_value_eth > 0.0 && exit_value_eth > 0.0 {
                            rollup.add_realized_pnl(entry_value_eth, exit_value_eth);
                        }
                    }
                }
            }
        }
        if let Some(order_id) = position.exit_order_id.as_deref() {
            if let Some(reports) = report_by_order.get(order_id) {
                rollup.add_reports(reports, ReportSide::Sell);
            }
        }
        if let Some(snapshot) = snapshot_by_position.get(&position.position_id) {
            rollup.add_snapshot(snapshot);
        }
    }

    for order in orders {
        let key = pool_key(&order.token_address, &order.pool_address);
        pools
            .entry(key)
            .or_insert_with(|| PoolRollup::order_only(order))
            .add_order(order);
    }

    for risk in risks {
        let keys = if let Some(pool_address) = risk.pool_address.as_deref() {
            vec![pool_key(&risk.token_address, pool_address)]
        } else {
            token_to_pool_keys
                .get(&normalize_key(&risk.token_address))
                .cloned()
                .filter(|keys| !keys.is_empty())
                .unwrap_or_else(|| vec![pool_key(&risk.token_address, "")])
        };
        for key in keys {
            pools
                .entry(key)
                .or_insert_with(|| PoolRollup::risk_only(risk))
                .add_risk(risk);
        }
    }

    pools.into_values().collect()
}

fn build_totals(
    positions: &[PositionRecord],
    orders: &[OrderRecord],
    reports: &[ExecutionReportRecord],
    risks: &[RiskRecord],
    pools: &[PoolRollup],
) -> StrategyPerformanceTotals {
    let open_positions = positions
        .iter()
        .filter(|position| !is_terminal_state(&position.state))
        .count() as u64;
    let buy_orders = orders.iter().filter(|order| order.side == "buy").count() as u64;
    let sell_orders = orders.iter().filter(|order| order.side == "sell").count() as u64;
    let confirmed_reports = reports
        .iter()
        .filter(|report| report.status == "confirmed")
        .count() as u64;
    let failed_reports = reports
        .iter()
        .filter(|report| is_failed_report_status(&report.status))
        .count() as u64;
    let critical_risk_events = risks
        .iter()
        .filter(|risk| risk.severity == "critical")
        .count() as u64;

    let capital_deployed_eth = pools
        .iter()
        .map(|pool| pool.capital_deployed_eth)
        .sum::<f64>();
    let buy_volume_eth = pools.iter().map(|pool| pool.buy_volume_eth).sum::<f64>();
    let sell_volume_eth = pools.iter().map(|pool| pool.sell_volume_eth).sum::<f64>();
    let pools_with_snapshots = pools.iter().filter(|pool| pool.snapshot_count > 0).count() as u64;
    let pools_with_pnl = pools
        .iter()
        .filter(|pool| pool.pnl_observation_count > 0)
        .count() as u64;
    let has_snapshots = pools_with_snapshots > 0;
    let has_pnl = pools_with_pnl > 0;
    let current_value_eth = pools.iter().map(|pool| pool.current_value_eth).sum::<f64>();
    let realized_pnl_eth = pools.iter().map(|pool| pool.realized_pnl_eth).sum::<f64>();
    let unrealized_pnl_eth = pools
        .iter()
        .map(|pool| pool.unrealized_pnl_eth)
        .sum::<f64>();
    let total_pnl_eth = realized_pnl_eth + unrealized_pnl_eth;

    StrategyPerformanceTotals {
        positions: positions.len() as u64,
        open_positions,
        closed_positions: positions.len() as u64 - open_positions,
        buy_orders,
        sell_orders,
        execution_reports: reports.len() as u64,
        confirmed_reports,
        failed_reports,
        confirmed_buys: pools.iter().map(|pool| pool.confirmed_buy_count).sum(),
        confirmed_sells: pools.iter().map(|pool| pool.confirmed_sell_count).sum(),
        risk_events: risks.len() as u64,
        critical_risk_events,
        capital_deployed_eth: positive_or_none(capital_deployed_eth),
        buy_volume_eth: positive_or_none(buy_volume_eth),
        sell_volume_eth: positive_or_none(sell_volume_eth),
        current_value_eth: has_snapshots.then_some(current_value_eth),
        realized_pnl_eth: has_pnl.then_some(realized_pnl_eth),
        unrealized_pnl_eth: has_snapshots.then_some(unrealized_pnl_eth),
        total_pnl_eth: has_pnl.then_some(total_pnl_eth),
        roi_percent: if has_pnl && capital_deployed_eth > 0.0 {
            Some((total_pnl_eth / capital_deployed_eth) * 100.0)
        } else {
            None
        },
        pools_with_snapshots,
        pools_with_pnl,
    }
}

fn build_timeline(
    positions: &[PositionRecord],
    orders: &[OrderRecord],
    reports: &[ExecutionReportRecord],
    risks: &[RiskRecord],
    report_by_order: &HashMap<String, Vec<ExecutionReportRecord>>,
    bucket_secs: u64,
    bucket_limit: u64,
) -> Vec<StrategyPerformancePoint> {
    let mut buckets: BTreeMap<u64, TimelineBucket> = BTreeMap::new();

    for position in positions {
        bucket_mut(&mut buckets, position.created_epoch, bucket_secs).opened_positions += 1;
        if is_terminal_state(&position.state) {
            bucket_mut(&mut buckets, position.updated_epoch, bucket_secs).closed_positions += 1;
        }
        if let Some(order_id) = position.entry_order_id.as_deref() {
            if let Some(entry_reports) = report_by_order.get(order_id) {
                for report in entry_reports {
                    if report.status == "confirmed" {
                        let bucket = bucket_mut(&mut buckets, report.created_epoch, bucket_secs);
                        bucket.confirmed_buys += 1;
                        bucket.buy_volume_eth += report.filled_amount_eth().unwrap_or(0.0);
                    }
                }
            }
        }
        if let Some(order_id) = position.exit_order_id.as_deref() {
            if let Some(exit_reports) = report_by_order.get(order_id) {
                for report in exit_reports {
                    if report.status == "confirmed" {
                        let bucket = bucket_mut(&mut buckets, report.created_epoch, bucket_secs);
                        bucket.confirmed_sells += 1;
                        bucket.sell_volume_eth += report.filled_amount_eth().unwrap_or(0.0);
                    }
                }
            }
        }
    }

    for order in orders {
        let bucket = bucket_mut(&mut buckets, order.created_epoch, bucket_secs);
        match order.side.as_str() {
            "buy" => {
                bucket.buy_orders += 1;
            }
            "sell" => {
                bucket.sell_orders += 1;
            }
            _ => {}
        }
    }

    for report in reports {
        let bucket = bucket_mut(&mut buckets, report.created_epoch, bucket_secs);
        bucket.execution_reports += 1;
        if report.status == "confirmed" {
            bucket.confirmed_reports += 1;
        } else if is_failed_report_status(&report.status) {
            bucket.failed_reports += 1;
        }
    }

    for risk in risks {
        let bucket = bucket_mut(&mut buckets, risk.created_epoch, bucket_secs);
        bucket.risk_events += 1;
        if risk.severity == "critical" {
            bucket.critical_risk_events += 1;
        }
    }

    let mut points = buckets
        .into_iter()
        .map(
            |(bucket_start_unix_secs, bucket)| StrategyPerformancePoint {
                bucket_start_unix_secs,
                buy_orders: bucket.buy_orders,
                sell_orders: bucket.sell_orders,
                buy_volume_eth: positive_or_none(bucket.buy_volume_eth),
                sell_volume_eth: positive_or_none(bucket.sell_volume_eth),
                execution_reports: bucket.execution_reports,
                confirmed_reports: bucket.confirmed_reports,
                failed_reports: bucket.failed_reports,
                confirmed_buys: bucket.confirmed_buys,
                confirmed_sells: bucket.confirmed_sells,
                opened_positions: bucket.opened_positions,
                closed_positions: bucket.closed_positions,
                risk_events: bucket.risk_events,
                critical_risk_events: bucket.critical_risk_events,
                realized_pnl_eth: None,
                unrealized_pnl_eth: None,
                total_pnl_eth: None,
            },
        )
        .collect::<Vec<_>>();
    if points.len() > bucket_limit as usize {
        points.drain(0..points.len() - bucket_limit as usize);
    }
    points
}

#[derive(Default)]
struct TimelineBucket {
    buy_orders: u64,
    sell_orders: u64,
    buy_volume_eth: f64,
    sell_volume_eth: f64,
    execution_reports: u64,
    confirmed_reports: u64,
    failed_reports: u64,
    confirmed_buys: u64,
    confirmed_sells: u64,
    opened_positions: u64,
    closed_positions: u64,
    risk_events: u64,
    critical_risk_events: u64,
}

fn bucket_mut(
    buckets: &mut BTreeMap<u64, TimelineBucket>,
    epoch: i64,
    bucket_secs: u64,
) -> &mut TimelineBucket {
    let safe_epoch = u64::try_from(epoch).unwrap_or_default();
    let bucket_start = safe_epoch - safe_epoch % bucket_secs;
    buckets.entry(bucket_start).or_default()
}

fn reports_by_order(
    reports: &[ExecutionReportRecord],
) -> HashMap<String, Vec<ExecutionReportRecord>> {
    let mut by_order: HashMap<String, Vec<ExecutionReportRecord>> = HashMap::new();
    for report in reports {
        by_order
            .entry(report.order_id.clone())
            .or_default()
            .push(report.clone());
    }
    by_order
}

fn confirmed_filled_eth(reports: &[ExecutionReportRecord]) -> f64 {
    reports
        .iter()
        .filter(|report| report.status == "confirmed")
        .filter_map(ExecutionReportRecord::filled_amount_eth)
        .sum()
}

fn snapshots_by_position(snapshots: Vec<SnapshotRecord>) -> HashMap<String, SnapshotRecord> {
    snapshots
        .into_iter()
        .map(|snapshot| (snapshot.position_id.clone(), snapshot))
        .collect()
}

fn breakdown<'a>(values: impl Iterator<Item = &'a str>) -> Vec<StrategyBreakdown> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    let mut total = 0_u64;
    for value in values {
        total += 1;
        *counts.entry(value.to_string()).or_default() += 1;
    }
    let mut rows = counts
        .into_iter()
        .map(|(key, count)| StrategyBreakdown {
            key,
            count,
            share_percent: if total > 0 {
                count as f64 / total as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    rows
}

fn compare_pool_rollups(left: &PoolRollup, right: &PoolRollup) -> std::cmp::Ordering {
    right
        .total_pnl_eth()
        .unwrap_or(f64::NEG_INFINITY)
        .partial_cmp(&left.total_pnl_eth().unwrap_or(f64::NEG_INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| right.critical_risk_count.cmp(&left.critical_risk_count))
        .then_with(|| right.risk_event_count.cmp(&left.risk_event_count))
        .then_with(|| right.buy_order_count.cmp(&left.buy_order_count))
        .then_with(|| right.updated_epoch.cmp(&left.updated_epoch))
}

fn pool_key(token_address: &str, pool_address: &str) -> (String, String) {
    (normalize_key(token_address), normalize_key(pool_address))
}

fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn amount_to_f64(raw: &str, decimals: i16) -> Option<f64> {
    let value = raw.parse::<f64>().ok()?;
    let scale = 10_f64.powi(i32::from(decimals).clamp(0, 36));
    if scale.is_finite() && scale > 0.0 {
        Some(value / scale)
    } else {
        None
    }
}

fn decimal_text(row: &sqlx::postgres::PgRow, name: &str) -> Result<Option<f64>> {
    let value: String = row.try_get(name).map_err(store_error)?;
    Ok(value.parse::<f64>().ok())
}

fn is_terminal_state(state: &str) -> bool {
    matches!(state, "sell_confirmed" | "failed" | "cancelled" | "scammed")
}

fn is_failed_report_status(status: &str) -> bool {
    matches!(status, "failed" | "rejected" | "reverted" | "timeout")
}

fn positive_or_none(value: f64) -> Option<f64> {
    if value > 0.0 {
        Some(value)
    } else {
        None
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn store_error(error: impl std::fmt::Display) -> AlphaCoreError {
    AlphaCoreError::Store(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_raw_amount_to_decimal_eth() {
        assert_eq!(
            amount_to_f64("1500000000000000000", 18).map(|value| value.round() as i64),
            Some(2)
        );
        assert_eq!(amount_to_f64("2500000", 6), Some(2.5));
    }

    #[test]
    fn terminal_states_match_position_lifecycle() {
        assert!(is_terminal_state("sell_confirmed"));
        assert!(is_terminal_state("failed"));
        assert!(is_terminal_state("cancelled"));
        assert!(is_terminal_state("scammed"));
        assert!(!is_terminal_state("buy_confirmed"));
    }

    #[test]
    fn buckets_events_by_requested_window() {
        let mut buckets = BTreeMap::new();
        let epoch = 1_775_000_123_i64;
        let bucket_start = epoch - epoch % 300;
        bucket_mut(&mut buckets, bucket_start + 12, 300).buy_orders += 1;
        bucket_mut(&mut buckets, bucket_start + 240, 300).buy_orders += 1;
        bucket_mut(&mut buckets, bucket_start + 301, 300).sell_orders += 1;

        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets.values().next().unwrap().buy_orders, 2);
        assert_eq!(buckets.values().last().unwrap().sell_orders, 1);
    }
}
