use eyre::{eyre, Result};
use serde::Serialize;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

const STRATEGY_ID: &str = "snipe-all-v1";
const STRATEGY_NAME: &str = "Snipe All v1";
const MAX_ROW_LIMIT: i64 = 250;

#[derive(Clone)]
pub struct AlphaTradingStore {
    pool: PgPool,
}

#[derive(Debug, Serialize)]
pub struct AlphaStrategyListResponse {
    pub count: usize,
    pub strategies: Vec<AlphaStrategySummary>,
}

#[derive(Debug, Serialize)]
pub struct AlphaStrategyDetailResponse {
    pub strategy: AlphaStrategySummary,
    pub recent_runs: Vec<TraderRunView>,
    pub positions: Vec<PositionView>,
    pub orders: Vec<OrderIntentView>,
    pub execution_reports: Vec<ExecutionReportView>,
    pub risk_events: Vec<RiskEventView>,
}

#[derive(Debug, Serialize)]
pub struct AlphaStrategySummary {
    pub strategy_id: String,
    pub name: String,
    pub description: String,
    pub mode: Option<String>,
    pub status: String,
    pub run_id: Option<String>,
    pub started_at: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub stopped_at: Option<String>,
    pub trading_enabled: Option<bool>,
    pub live_status: Option<String>,
    pub live_current_block: Option<u64>,
    pub positions: i64,
    pub open_positions: i64,
    pub orders: i64,
    pub execution_reports: i64,
    pub risk_events: i64,
    pub latest_order_at: Option<String>,
    pub latest_execution_report_at: Option<String>,
    pub latest_risk_event_at: Option<String>,
    pub rules: Vec<StrategyRuleView>,
}

#[derive(Debug, Serialize)]
pub struct StrategyRuleView {
    pub rule_id: &'static str,
    pub name: &'static str,
    pub status: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Serialize)]
pub struct TraderRunView {
    pub run_id: String,
    pub mode: String,
    pub status: String,
    pub started_at: String,
    pub last_heartbeat_at: String,
    pub stopped_at: Option<String>,
    pub config: Value,
    pub metadata: Value,
}

#[derive(Debug, Serialize)]
pub struct PositionView {
    pub position_id: String,
    pub portfolio_id: String,
    pub wallet_id: String,
    pub strategy_name: String,
    pub token_address: String,
    pub pool_address: String,
    pub state: String,
    pub entry_order_id: Option<String>,
    pub exit_order_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct OrderIntentView {
    pub id: i64,
    pub portfolio_id: String,
    pub wallet_id: String,
    pub strategy_name: String,
    pub side: String,
    pub token_address: String,
    pub pool_address: String,
    pub amount_raw: String,
    pub amount_decimals: i16,
    pub max_slippage_bps: i32,
    pub deadline_secs: i64,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct ExecutionReportView {
    pub id: i64,
    pub order_id: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub filled_amount_raw: Option<String>,
    pub filled_amount_decimals: Option<i16>,
    pub gas_used: Option<i64>,
    pub error: Option<String>,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct RiskEventView {
    pub id: i64,
    pub kind: String,
    pub severity: String,
    pub token_address: String,
    pub pool_address: Option<String>,
    pub pending_tx_hash: Option<String>,
    pub observed_block: Option<i64>,
    pub message: String,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Default)]
struct StrategyCounts {
    positions: i64,
    open_positions: i64,
    orders: i64,
    execution_reports: i64,
    risk_events: i64,
    latest_order_at: Option<String>,
    latest_execution_report_at: Option<String>,
    latest_risk_event_at: Option<String>,
}

impl AlphaTradingStore {
    pub fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_lazy(database_url)?;
        Ok(Self { pool })
    }

    pub async fn list_strategies(&self) -> Result<AlphaStrategyListResponse> {
        let strategies = vec![self.strategy_summary(STRATEGY_ID).await?];
        Ok(AlphaStrategyListResponse {
            count: strategies.len(),
            strategies,
        })
    }

    pub async fn strategy_detail(
        &self,
        strategy_id: &str,
    ) -> Result<Option<AlphaStrategyDetailResponse>> {
        if strategy_id != STRATEGY_ID {
            return Ok(None);
        }

        let strategy = self.strategy_summary(strategy_id).await?;
        let Some(run_id) = strategy.run_id.clone() else {
            return Ok(Some(AlphaStrategyDetailResponse {
                strategy,
                recent_runs: Vec::new(),
                positions: Vec::new(),
                orders: Vec::new(),
                execution_reports: Vec::new(),
                risk_events: Vec::new(),
            }));
        };

        Ok(Some(AlphaStrategyDetailResponse {
            strategy,
            recent_runs: self.recent_runs(10).await?,
            positions: self.positions(&run_id, strategy_id, 100).await?,
            orders: self.orders(&run_id, strategy_id, 100).await?,
            execution_reports: self.execution_reports(&run_id, 100).await?,
            risk_events: self.risk_events(&run_id, 100).await?,
        }))
    }

    async fn strategy_summary(&self, strategy_id: &str) -> Result<AlphaStrategySummary> {
        if strategy_id != STRATEGY_ID {
            return Err(eyre!("unknown alpha strategy: {strategy_id}"));
        }

        let latest_run = self.latest_run().await?;
        let counts = match latest_run.as_ref() {
            Some(run) => self.strategy_counts(&run.run_id, strategy_id).await?,
            None => StrategyCounts::default(),
        };

        Ok(summary_from_run(latest_run.as_ref(), counts))
    }

    async fn latest_run(&self) -> Result<Option<TraderRunView>> {
        let row = sqlx::query(
            r#"
            SELECT run_id, mode, status, started_at::text AS started_at,
                   last_heartbeat_at::text AS last_heartbeat_at,
                   stopped_at::text AS stopped_at,
                   config::text AS config, metadata::text AS metadata
            FROM alpha_trading.trader_runs
            ORDER BY last_heartbeat_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(row_to_run).transpose()
    }

    async fn recent_runs(&self, limit: i64) -> Result<Vec<TraderRunView>> {
        let rows = sqlx::query(
            r#"
            SELECT run_id, mode, status, started_at::text AS started_at,
                   last_heartbeat_at::text AS last_heartbeat_at,
                   stopped_at::text AS stopped_at,
                   config::text AS config, metadata::text AS metadata
            FROM alpha_trading.trader_runs
            ORDER BY last_heartbeat_at DESC
            LIMIT $1
            "#,
        )
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_run).collect()
    }

    async fn strategy_counts(&self, run_id: &str, strategy_id: &str) -> Result<StrategyCounts> {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM alpha_trading.positions
                 WHERE run_id = $1 AND strategy_name = $2) AS positions,
                (SELECT COUNT(*) FROM alpha_trading.positions
                 WHERE run_id = $1 AND strategy_name = $2
                   AND state NOT IN ('sell_confirmed', 'failed', 'cancelled', 'scammed')) AS open_positions,
                (SELECT COUNT(*) FROM alpha_trading.order_intents
                 WHERE run_id = $1 AND strategy_name = $2) AS orders,
                (SELECT COUNT(*) FROM alpha_trading.execution_reports
                 WHERE run_id = $1) AS execution_reports,
                (SELECT COUNT(*) FROM alpha_trading.risk_events
                 WHERE run_id = $1) AS risk_events,
                (SELECT MAX(created_at)::text FROM alpha_trading.order_intents
                 WHERE run_id = $1 AND strategy_name = $2) AS latest_order_at,
                (SELECT MAX(created_at)::text FROM alpha_trading.execution_reports
                 WHERE run_id = $1) AS latest_execution_report_at,
                (SELECT MAX(created_at)::text FROM alpha_trading.risk_events
                 WHERE run_id = $1) AS latest_risk_event_at
            "#,
        )
        .bind(run_id)
        .bind(strategy_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(StrategyCounts {
            positions: int(&row, "positions")?,
            open_positions: int(&row, "open_positions")?,
            orders: int(&row, "orders")?,
            execution_reports: int(&row, "execution_reports")?,
            risk_events: int(&row, "risk_events")?,
            latest_order_at: optional_text(&row, "latest_order_at")?,
            latest_execution_report_at: optional_text(&row, "latest_execution_report_at")?,
            latest_risk_event_at: optional_text(&row, "latest_risk_event_at")?,
        })
    }

    async fn positions(
        &self,
        run_id: &str,
        strategy_id: &str,
        limit: i64,
    ) -> Result<Vec<PositionView>> {
        let rows = sqlx::query(
            r#"
            SELECT position_id, portfolio_id, wallet_id, strategy_name, token_address,
                   pool_address, state, entry_order_id, exit_order_id,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   payload::text AS payload
            FROM alpha_trading.positions
            WHERE run_id = $1 AND strategy_name = $2
            ORDER BY updated_at DESC
            LIMIT $3
            "#,
        )
        .bind(run_id)
        .bind(strategy_id)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_position).collect()
    }

    async fn orders(
        &self,
        run_id: &str,
        strategy_id: &str,
        limit: i64,
    ) -> Result<Vec<OrderIntentView>> {
        let rows = sqlx::query(
            r#"
            SELECT id, portfolio_id, wallet_id, strategy_name, side, token_address,
                   pool_address, amount_raw, amount_decimals, max_slippage_bps,
                   deadline_secs, created_at::text AS created_at, payload::text AS payload
            FROM alpha_trading.order_intents
            WHERE run_id = $1 AND strategy_name = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(run_id)
        .bind(strategy_id)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_order).collect()
    }

    async fn execution_reports(
        &self,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<ExecutionReportView>> {
        let rows = sqlx::query(
            r#"
            SELECT id, order_id, status, tx_hash, block_number, filled_amount_raw,
                   filled_amount_decimals, gas_used, error,
                   created_at::text AS created_at, payload::text AS payload
            FROM alpha_trading.execution_reports
            WHERE run_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(run_id)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_execution_report).collect()
    }

    async fn risk_events(&self, run_id: &str, limit: i64) -> Result<Vec<RiskEventView>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, severity, token_address, pool_address, pending_tx_hash,
                   observed_block, message, created_at::text AS created_at,
                   payload::text AS payload
            FROM alpha_trading.risk_events
            WHERE run_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(run_id)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_risk_event).collect()
    }
}

fn summary_from_run(run: Option<&TraderRunView>, counts: StrategyCounts) -> AlphaStrategySummary {
    let metadata = run.map(|run| &run.metadata).unwrap_or(&Value::Null);
    AlphaStrategySummary {
        strategy_id: STRATEGY_ID.to_string(),
        name: STRATEGY_NAME.to_string(),
        description: "Paper strategy that buys every newly observed eligible live pool once and exits on matching liquidity-removal risk.".to_string(),
        mode: run.map(|run| run.mode.clone()),
        status: run
            .map(|run| run.status.clone())
            .unwrap_or_else(|| "not_started".to_string()),
        run_id: run.map(|run| run.run_id.clone()),
        started_at: run.map(|run| run.started_at.clone()),
        last_heartbeat_at: run.map(|run| run.last_heartbeat_at.clone()),
        stopped_at: run.and_then(|run| run.stopped_at.clone()),
        trading_enabled: metadata_bool(metadata, "trading_enabled"),
        live_status: metadata_string(metadata, "live_status"),
        live_current_block: metadata_u64(metadata, "live_current_block"),
        positions: counts.positions,
        open_positions: counts.open_positions,
        orders: counts.orders,
        execution_reports: counts.execution_reports,
        risk_events: counts.risk_events,
        latest_order_at: counts.latest_order_at,
        latest_execution_report_at: counts.latest_execution_report_at,
        latest_risk_event_at: counts.latest_risk_event_at,
        rules: strategy_rules(),
    }
}

fn strategy_rules() -> Vec<StrategyRuleView> {
    vec![
        StrategyRuleView {
            rule_id: "entry_pool_update",
            name: "Buy Eligible Pools",
            status: "active",
            description:
                "Submit one paper buy for each new live pool that can buy, can sell, is not flagged as scam, and meets the configured liquidity floor.",
        },
        StrategyRuleView {
            rule_id: "exit_liquidity_removal",
            name: "Exit On Liquidity Removal",
            status: "active",
            description:
                "Submit a paper sell when a liquidity-removal risk event matches an open position.",
        },
        StrategyRuleView {
            rule_id: "lp_approval",
            name: "LP Approval Response",
            status: "scaffolded",
            description:
                "LP approval events are recorded for analysis. Private-creator immediate exits will be added here.",
        },
        StrategyRuleView {
            rule_id: "tax_honeypot",
            name: "Tax And Honeypot Response",
            status: "scaffolded",
            description:
                "Tax and honeypot risk events are recorded now; sell and blocklist actions will be enabled after validation.",
        },
        StrategyRuleView {
            rule_id: "creator_label",
            name: "Creator Execution Label",
            status: "scaffolded",
            description:
                "Creator public/private behavior labels are planned so the strategy can react differently to LP approvals.",
        },
    ]
}

fn row_to_run(row: &sqlx::postgres::PgRow) -> Result<TraderRunView> {
    Ok(TraderRunView {
        run_id: text(row, "run_id")?,
        mode: text(row, "mode")?,
        status: text(row, "status")?,
        started_at: text(row, "started_at")?,
        last_heartbeat_at: text(row, "last_heartbeat_at")?,
        stopped_at: optional_text(row, "stopped_at")?,
        config: json_text(row, "config")?,
        metadata: json_text(row, "metadata")?,
    })
}

fn row_to_position(row: &sqlx::postgres::PgRow) -> Result<PositionView> {
    Ok(PositionView {
        position_id: text(row, "position_id")?,
        portfolio_id: text(row, "portfolio_id")?,
        wallet_id: text(row, "wallet_id")?,
        strategy_name: text(row, "strategy_name")?,
        token_address: text(row, "token_address")?,
        pool_address: text(row, "pool_address")?,
        state: text(row, "state")?,
        entry_order_id: optional_text(row, "entry_order_id")?,
        exit_order_id: optional_text(row, "exit_order_id")?,
        created_at: text(row, "created_at")?,
        updated_at: text(row, "updated_at")?,
        payload: json_text(row, "payload")?,
    })
}

fn row_to_order(row: &sqlx::postgres::PgRow) -> Result<OrderIntentView> {
    Ok(OrderIntentView {
        id: int(row, "id")?,
        portfolio_id: text(row, "portfolio_id")?,
        wallet_id: text(row, "wallet_id")?,
        strategy_name: text(row, "strategy_name")?,
        side: text(row, "side")?,
        token_address: text(row, "token_address")?,
        pool_address: text(row, "pool_address")?,
        amount_raw: text(row, "amount_raw")?,
        amount_decimals: small_int(row, "amount_decimals")?,
        max_slippage_bps: int32(row, "max_slippage_bps")?,
        deadline_secs: int(row, "deadline_secs")?,
        created_at: text(row, "created_at")?,
        payload: json_text(row, "payload")?,
    })
}

fn row_to_execution_report(row: &sqlx::postgres::PgRow) -> Result<ExecutionReportView> {
    Ok(ExecutionReportView {
        id: int(row, "id")?,
        order_id: text(row, "order_id")?,
        status: text(row, "status")?,
        tx_hash: optional_text(row, "tx_hash")?,
        block_number: optional_int(row, "block_number")?,
        filled_amount_raw: optional_text(row, "filled_amount_raw")?,
        filled_amount_decimals: optional_small_int(row, "filled_amount_decimals")?,
        gas_used: optional_int(row, "gas_used")?,
        error: optional_text(row, "error")?,
        created_at: text(row, "created_at")?,
        payload: json_text(row, "payload")?,
    })
}

fn row_to_risk_event(row: &sqlx::postgres::PgRow) -> Result<RiskEventView> {
    Ok(RiskEventView {
        id: int(row, "id")?,
        kind: text(row, "kind")?,
        severity: text(row, "severity")?,
        token_address: text(row, "token_address")?,
        pool_address: optional_text(row, "pool_address")?,
        pending_tx_hash: optional_text(row, "pending_tx_hash")?,
        observed_block: optional_int(row, "observed_block")?,
        message: text(row, "message")?,
        created_at: text(row, "created_at")?,
        payload: json_text(row, "payload")?,
    })
}

fn metadata_bool(metadata: &Value, key: &str) -> Option<bool> {
    metadata.get(key).and_then(Value::as_bool)
}

fn metadata_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn metadata_u64(metadata: &Value, key: &str) -> Option<u64> {
    metadata.get(key).and_then(Value::as_u64)
}

fn json_text(row: &sqlx::postgres::PgRow, column: &str) -> Result<Value> {
    let value = text(row, column)?;
    serde_json::from_str(&value).map_err(|err| eyre!("failed to parse {column} json: {err}"))
}

fn text(row: &sqlx::postgres::PgRow, column: &str) -> Result<String> {
    row.try_get::<String, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn optional_text(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<String>> {
    row.try_get::<Option<String>, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn int(row: &sqlx::postgres::PgRow, column: &str) -> Result<i64> {
    row.try_get::<i64, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn optional_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<i64>> {
    row.try_get::<Option<i64>, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn int32(row: &sqlx::postgres::PgRow, column: &str) -> Result<i32> {
    row.try_get::<i32, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn small_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<i16> {
    row.try_get::<i16, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn optional_small_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<i16>> {
    row.try_get::<Option<i16>, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

fn clamp_limit(limit: i64) -> i64 {
    limit.clamp(1, MAX_ROW_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strategy_catalog_is_dashboard_ready() {
        let rules = strategy_rules();

        assert_eq!(rules[0].rule_id, "entry_pool_update");
        assert!(rules.iter().any(|rule| rule.status == "active"));
        assert!(rules.iter().any(|rule| rule.status == "scaffolded"));
    }

    #[test]
    fn summary_reads_live_metadata() {
        let run = TraderRunView {
            run_id: "run-1".to_string(),
            mode: "paper".to_string(),
            status: "running".to_string(),
            started_at: "2026-05-08 00:00:00+00".to_string(),
            last_heartbeat_at: "2026-05-08 00:00:01+00".to_string(),
            stopped_at: None,
            config: json!({}),
            metadata: json!({
                "trading_enabled": true,
                "live_status": "live",
                "live_current_block": 25000000,
            }),
        };

        let summary = summary_from_run(Some(&run), StrategyCounts::default());

        assert_eq!(summary.strategy_id, STRATEGY_ID);
        assert_eq!(summary.trading_enabled, Some(true));
        assert_eq!(summary.live_status.as_deref(), Some("live"));
        assert_eq!(summary.live_current_block, Some(25_000_000));
    }
}
