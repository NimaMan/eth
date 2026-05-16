//! PostgreSQL persistence for the alpha trading runtime.

pub mod performance;
pub mod result_sets;

use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus},
    ids::{PoolAddress, PositionId},
    order::{OrderIntent, OrderSide},
    position::{Position, PositionSnapshot, PositionState},
    risk::{RiskEvent, RiskKind, RiskSeverity},
    store::{StrategyDecisionRecord, TradingStore},
};
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

const DEFAULT_MAX_CONNECTIONS: u32 = 5;

#[derive(Clone)]
pub struct PostgresTradingStore {
    pool: PgPool,
    run_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyObservationCursor {
    pub event_source: String,
    pub event_key: String,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub block_number: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct StrategyObservationRecord {
    pub strategy_name: String,
    pub event_source: String,
    pub event_key: String,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub block_number: Option<u64>,
    pub event_timestamp: Option<String>,
    pub decision: String,
    pub report_count: usize,
    pub payload: Value,
}

impl PostgresTradingStore {
    pub async fn connect(database_url: &str, run_id: impl Into<String>) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .connect(database_url)
            .await
            .map_err(store_error)?;
        let store = Self {
            pool,
            run_id: run_id.into(),
        };
        store.migrate().await?;
        Ok(store)
    }

    pub fn from_pool(pool: PgPool, run_id: impl Into<String>) -> Self {
        Self {
            pool,
            run_id: run_id.into(),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub async fn migrate(&self) -> Result<()> {
        let mut connection = self.pool.acquire().await.map_err(store_error)?;
        sqlx::query("SET client_min_messages TO WARNING")
            .execute(&mut *connection)
            .await
            .map_err(store_error)?;
        for statement in MIGRATIONS {
            sqlx::query(statement)
                .execute(&mut *connection)
                .await
                .map_err(store_error)?;
        }
        Ok(())
    }

    pub async fn start_run(&self, mode: &str, config: Value) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.trader_runs (
                run_id, mode, status, config, started_at, last_heartbeat_at, metadata
            )
            VALUES ($1, $2, 'running', $3, NOW(), NOW(), '{}'::jsonb)
            ON CONFLICT (run_id) DO UPDATE SET
                mode = EXCLUDED.mode,
                status = 'running',
                config = EXCLUDED.config,
                stopped_at = NULL,
                last_heartbeat_at = NOW()
            "#,
        )
        .bind(&self.run_id)
        .bind(mode)
        .bind(config.clone())
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        let result_set_id = result_set_id_for_run(&self.run_id, mode, &config);
        let result_set_mode = result_set_mode(mode, &config);
        let strategy_suite = config
            .get("strategy_suite")
            .and_then(Value::as_str)
            .map(str::to_string);
        let start_block = config
            .get("from_block")
            .and_then(Value::as_u64)
            .map(u64_to_i64);
        let end_block = config
            .get("to_block")
            .and_then(Value::as_u64)
            .map(u64_to_i64);
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.backtest_result_sets (
                result_set_id, mode, status, strategy_suite, start_block, end_block,
                config, metadata, created_at, updated_at
            )
            VALUES ($1, $2, 'running', $3, $4, $5, $6, '{}'::jsonb, NOW(), NOW())
            ON CONFLICT (result_set_id) DO UPDATE SET
                mode = EXCLUDED.mode,
                status = CASE
                    WHEN alpha_trading.backtest_result_sets.status = 'running' THEN 'running'
                    ELSE EXCLUDED.status
                END,
                strategy_suite = COALESCE(EXCLUDED.strategy_suite, alpha_trading.backtest_result_sets.strategy_suite),
                start_block = COALESCE(EXCLUDED.start_block, alpha_trading.backtest_result_sets.start_block),
                end_block = COALESCE(EXCLUDED.end_block, alpha_trading.backtest_result_sets.end_block),
                config = alpha_trading.backtest_result_sets.config || EXCLUDED.config,
                updated_at = NOW()
            "#,
        )
        .bind(&result_set_id)
        .bind(result_set_mode)
        .bind(strategy_suite)
        .bind(start_block)
        .bind(end_block)
        .bind(config)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.backtest_result_set_runs (
                result_set_id, run_id, created_at
            )
            VALUES ($1, $2, NOW())
            ON CONFLICT (result_set_id, run_id) DO NOTHING
            "#,
        )
        .bind(result_set_id)
        .bind(&self.run_id)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    pub async fn mark_stale_runs(&self, max_age_secs: u64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE alpha_trading.trader_runs
            SET status = 'stale',
                metadata = metadata || jsonb_build_object('reason', 'stale_heartbeat')
            WHERE run_id <> $1
              AND status = 'running'
              AND last_heartbeat_at < NOW() - ($2::bigint * INTERVAL '1 second')
            "#,
        )
        .bind(&self.run_id)
        .bind(u64_to_i64(max_age_secs))
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(result.rows_affected())
    }

    pub async fn heartbeat(&self, metadata: Value) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE alpha_trading.trader_runs
            SET last_heartbeat_at = NOW(), status = 'running', metadata = $2
            WHERE run_id = $1
            "#,
        )
        .bind(&self.run_id)
        .bind(metadata.clone())
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        sqlx::query(
            r#"
            UPDATE alpha_trading.backtest_result_sets rs
            SET status = 'running',
                metadata = rs.metadata || $2,
                updated_at = NOW()
            FROM alpha_trading.backtest_result_set_runs runs
            WHERE runs.result_set_id = rs.result_set_id
              AND runs.run_id = $1
            "#,
        )
        .bind(&self.run_id)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    pub async fn mark_stopped(&self, status: &str, metadata: Value) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE alpha_trading.trader_runs
            SET status = $2, stopped_at = NOW(), last_heartbeat_at = NOW(), metadata = $3
            WHERE run_id = $1
            "#,
        )
        .bind(&self.run_id)
        .bind(status)
        .bind(metadata.clone())
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        sqlx::query(
            r#"
            UPDATE alpha_trading.backtest_result_sets rs
            SET status = $2,
                stopped_at = NOW(),
                metadata = rs.metadata || $3,
                updated_at = NOW()
            FROM alpha_trading.backtest_result_set_runs runs
            WHERE runs.result_set_id = rs.result_set_id
              AND runs.run_id = $1
            "#,
        )
        .bind(&self.run_id)
        .bind(status)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    pub async fn load_strategy_observation_cursors(
        &self,
        strategy_name: &str,
    ) -> Result<Vec<StrategyObservationCursor>> {
        let rows = sqlx::query(
            r#"
            SELECT event_source, event_key, token_address, pool_address, block_number
            FROM alpha_trading.strategy_observations
            WHERE run_id = $1 AND strategy_name = $2
            "#,
        )
        .bind(&self.run_id)
        .bind(strategy_name)
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(StrategyObservationCursor {
                    event_source: row.try_get("event_source").map_err(store_error)?,
                    event_key: row.try_get("event_key").map_err(store_error)?,
                    token_address: row.try_get("token_address").map_err(store_error)?,
                    pool_address: row.try_get("pool_address").map_err(store_error)?,
                    block_number: row
                        .try_get::<Option<i64>, _>("block_number")
                        .map_err(store_error)?
                        .and_then(i64_to_u64),
                })
            })
            .collect()
    }

    pub async fn record_strategy_observation(
        &self,
        record: StrategyObservationRecord,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.strategy_observations (
                run_id, strategy_name, event_source, event_key, token_address,
                pool_address, block_number, event_timestamp, decision, report_count,
                payload, first_seen_at, last_seen_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
            ON CONFLICT (run_id, strategy_name, event_source, event_key) DO UPDATE SET
                token_address = EXCLUDED.token_address,
                pool_address = EXCLUDED.pool_address,
                block_number = EXCLUDED.block_number,
                event_timestamp = EXCLUDED.event_timestamp,
                decision = EXCLUDED.decision,
                report_count = EXCLUDED.report_count,
                payload = EXCLUDED.payload,
                last_seen_at = NOW()
            "#,
        )
        .bind(&self.run_id)
        .bind(record.strategy_name)
        .bind(record.event_source)
        .bind(record.event_key)
        .bind(record.token_address)
        .bind(record.pool_address)
        .bind(record.block_number.map(u64_to_i64))
        .bind(record.event_timestamp)
        .bind(record.decision)
        .bind(usize_to_i32(record.report_count))
        .bind(record.payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    pub async fn load_active_positions(&self, strategy_name: &str) -> Result<Vec<Position>> {
        let rows = sqlx::query(
            r#"
            SELECT payload::text AS payload
            FROM alpha_trading.positions
            WHERE run_id = $1
              AND strategy_name = $2
              AND state NOT IN ('sell_confirmed', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
            ORDER BY updated_at DESC
            "#,
        )
        .bind(&self.run_id)
        .bind(strategy_name)
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;

        rows.into_iter()
            .map(|row| {
                let payload = row.try_get::<String, _>("payload").map_err(store_error)?;
                let mut position =
                    serde_json::from_str::<Position>(&payload).map_err(store_error)?;
                normalize_position_pool_id(&mut position);
                Ok(position)
            })
            .collect()
    }

    pub async fn load_seen_pools(&self, strategy_name: &str) -> Result<Vec<PoolAddress>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT pool_address
            FROM alpha_trading.positions
            WHERE run_id = $1
              AND strategy_name = $2
              AND pool_address IS NOT NULL
            ORDER BY pool_address
            "#,
        )
        .bind(&self.run_id)
        .bind(strategy_name)
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;

        rows.into_iter()
            .map(|row| {
                row.try_get::<String, _>("pool_address")
                    .map(PoolAddress::from)
                    .map_err(store_error)
            })
            .collect()
    }

    pub async fn max_order_sequence_for_prefix(&self, order_prefix: &str) -> Result<u64> {
        let suffix_start = order_prefix.len() + 2;
        let like_pattern = format!("{order_prefix}-%");
        let row = sqlx::query(
            r#"
            SELECT COALESCE(MAX((substring(order_id FROM $3::int))::bigint), 0) AS max_sequence
            FROM alpha_trading.execution_reports
            WHERE run_id = $1
              AND order_id LIKE $2
              AND substring(order_id FROM $3::int) ~ '^[0-9]+$'
            "#,
        )
        .bind(&self.run_id)
        .bind(like_pattern)
        .bind(usize_to_i32(suffix_start))
        .fetch_one(&self.pool)
        .await
        .map_err(store_error)?;
        row.try_get::<i64, _>("max_sequence")
            .map_err(store_error)
            .map(|value| i64_to_u64(value).unwrap_or_default())
    }
}

#[async_trait]
impl TradingStore for PostgresTradingStore {
    async fn upsert_position(&self, position: &Position) -> Result<()> {
        let payload = to_json(position)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.positions (
                run_id, position_id, trade_id, portfolio_id, wallet_id, strategy_name,
                token_address, pool_address, state, entry_order_id, exit_order_id,
                entry_block, exit_block, payload, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())
            ON CONFLICT (run_id, position_id) DO UPDATE SET
                trade_id = EXCLUDED.trade_id,
                portfolio_id = EXCLUDED.portfolio_id,
                wallet_id = EXCLUDED.wallet_id,
                strategy_name = EXCLUDED.strategy_name,
                token_address = EXCLUDED.token_address,
                pool_address = EXCLUDED.pool_address,
                state = EXCLUDED.state,
                entry_order_id = EXCLUDED.entry_order_id,
                exit_order_id = EXCLUDED.exit_order_id,
                entry_block = EXCLUDED.entry_block,
                exit_block = EXCLUDED.exit_block,
                payload = EXCLUDED.payload,
                updated_at = NOW()
            "#,
        )
        .bind(&self.run_id)
        .bind(&position.id.0)
        .bind(&position.trade_id.0)
        .bind(&position.key.portfolio_id.0)
        .bind(&position.key.wallet_id.0)
        .bind(&position.key.strategy_name.0)
        .bind(position.key.token_address.to_string())
        .bind(position.key.pool_address.to_string())
        .bind(position_state_label(&position.state))
        .bind(position.entry_order_id.as_ref().map(|id| id.0.as_str()))
        .bind(position.exit_order_id.as_ref().map(|id| id.0.as_str()))
        .bind(position.entry_block.map(u64_to_i64))
        .bind(position.exit_block.map(u64_to_i64))
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        self.upsert_trade_for_position(position).await?;
        Ok(())
    }

    async fn append_position_snapshot(&self, snapshot: &PositionSnapshot) -> Result<()> {
        let payload = to_json(snapshot)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.position_snapshots (
                run_id, position_id, trade_id, state, block_number,
                observed_block_number, valuation_block_number, current_value_eth,
                realized_profit_eth, unrealized_profit_eth, roi, payload, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
            "#,
        )
        .bind(&self.run_id)
        .bind(&snapshot.position_id.0)
        .bind(&snapshot.trade_id.0)
        .bind(position_state_label(&snapshot.state))
        .bind(u64_to_i64(snapshot.block_number))
        .bind(snapshot.observed_block_number.map(u64_to_i64))
        .bind(snapshot.valuation_block_number.map(u64_to_i64))
        .bind(snapshot.current_value_eth.to_string())
        .bind(snapshot.realized_profit_eth.to_string())
        .bind(snapshot.unrealized_profit_eth.to_string())
        .bind(snapshot.roi.to_string())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        self.append_trade_snapshot(snapshot).await?;
        Ok(())
    }

    async fn record_order_intent(&self, intent: &OrderIntent) -> Result<()> {
        let payload = to_json(intent)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.order_intents (
                run_id, trade_id, portfolio_id, wallet_id, strategy_name, side, token_address,
                pool_address, amount_raw, amount_decimals, max_slippage_bps,
                deadline_secs, payload, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
            "#,
        )
        .bind(&self.run_id)
        .bind(intent.trade_id.as_ref().map(|id| id.0.as_str()))
        .bind(&intent.portfolio_id.0)
        .bind(&intent.wallet_id.0)
        .bind(&intent.strategy_name.0)
        .bind(order_side_label(intent.side))
        .bind(intent.token_address.to_string())
        .bind(intent.pool_address.to_string())
        .bind(amount_raw(&intent.amount))
        .bind(i16::from(intent.amount.decimals))
        .bind(u32_to_i32(intent.max_slippage_bps))
        .bind(u64_to_i64(intent.deadline_secs))
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn record_execution_report(&self, report: &ExecutionReport) -> Result<()> {
        self.record_execution_report_with_context(None, None, report)
            .await
    }

    async fn record_position_execution_report(
        &self,
        position_id: &PositionId,
        report: &ExecutionReport,
    ) -> Result<()> {
        self.record_execution_report_with_context(Some(position_id), None, report)
            .await
    }

    async fn record_order_execution_report(
        &self,
        position_id: &PositionId,
        side: OrderSide,
        report: &ExecutionReport,
    ) -> Result<()> {
        self.record_execution_report_with_context(Some(position_id), Some(side), report)
            .await
    }

    async fn record_risk_event(&self, event: &RiskEvent) -> Result<()> {
        let payload = to_json(event)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.risk_events (
                run_id, kind, severity, token_address, pool_address, pending_tx_hash,
                observed_block, message, payload, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(&self.run_id)
        .bind(risk_kind_label(&event.kind))
        .bind(risk_severity_label(&event.severity))
        .bind(event.token_address.to_string())
        .bind(
            event
                .pool_address
                .as_ref()
                .map(|address| address.to_string()),
        )
        .bind(event.pending_tx_hash.map(|hash| hash.to_string()))
        .bind(event.observed_block.map(u64_to_i64))
        .bind(&event.message)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn record_strategy_decision(&self, record: &StrategyDecisionRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.strategy_decisions (
                run_id, strategy_name, event_source, event_key, block_number,
                token_address, pool_address, action, reason, order_side, payload, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
            "#,
        )
        .bind(&self.run_id)
        .bind(&record.strategy_name)
        .bind(&record.event_source)
        .bind(&record.event_key)
        .bind(record.block_number.map(u64_to_i64))
        .bind(&record.token_address)
        .bind(&record.pool_address)
        .bind(&record.action)
        .bind(&record.reason)
        .bind(record.order_side.map(order_side_label))
        .bind(&record.payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }
}

impl PostgresTradingStore {
    async fn upsert_trade_for_position(&self, position: &Position) -> Result<()> {
        let payload = to_json(position)?;
        let result_set_id = self.result_set_id().await?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.trades (
                trade_id, result_set_id, run_id, position_id, strategy_name,
                token_address, pool_address, state, entry_order_id, exit_order_id,
                entry_block, exit_block, entry_cost_eth, exit_value_eth, gas_cost_eth,
                realized_pnl_eth, payload, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NOW(), NOW())
            ON CONFLICT (trade_id) DO UPDATE SET
                result_set_id = EXCLUDED.result_set_id,
                run_id = EXCLUDED.run_id,
                position_id = EXCLUDED.position_id,
                strategy_name = EXCLUDED.strategy_name,
                token_address = EXCLUDED.token_address,
                pool_address = EXCLUDED.pool_address,
                state = EXCLUDED.state,
                entry_order_id = EXCLUDED.entry_order_id,
                exit_order_id = EXCLUDED.exit_order_id,
                entry_block = EXCLUDED.entry_block,
                exit_block = EXCLUDED.exit_block,
                entry_cost_eth = EXCLUDED.entry_cost_eth,
                exit_value_eth = EXCLUDED.exit_value_eth,
                gas_cost_eth = EXCLUDED.gas_cost_eth,
                realized_pnl_eth = EXCLUDED.realized_pnl_eth,
                payload = EXCLUDED.payload,
                updated_at = NOW()
            "#,
        )
        .bind(&position.trade_id.0)
        .bind(result_set_id)
        .bind(&self.run_id)
        .bind(&position.id.0)
        .bind(&position.key.strategy_name.0)
        .bind(position.key.token_address.to_string())
        .bind(position.key.pool_address.to_string())
        .bind(position_state_label(&position.state))
        .bind(position.entry_order_id.as_ref().map(|id| id.0.as_str()))
        .bind(position.exit_order_id.as_ref().map(|id| id.0.as_str()))
        .bind(position.entry_block.map(u64_to_i64))
        .bind(position.exit_block.map(u64_to_i64))
        .bind(position.entry_cost_basis.map(|value| value.to_string()))
        .bind(position.exit_proceeds.map(|value| value.to_string()))
        .bind(position.gas_cost_eth.to_string())
        .bind(position.realized_pnl().to_string())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn append_trade_snapshot(&self, snapshot: &PositionSnapshot) -> Result<()> {
        let payload = to_json(snapshot)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.trade_snapshots (
                trade_id, run_id, position_id, state, block_number,
                observed_block_number, valuation_block_number, current_value_eth,
                realized_pnl_eth, unrealized_pnl_eth, total_pnl_eth, roi,
                pool_price_to_initial_price_ratio, pool_initial_price_denom_per_token,
                pool_price_denom_per_token, pool_liquidity_denom, pool_token_reserve,
                pool_denom_symbol, payload, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                ((NULLIF($9, '')::numeric + NULLIF($10, '')::numeric)::text),
                $11, $12, $13, $14, $15, $16, $17, $18, NOW()
            )
            "#,
        )
        .bind(&snapshot.trade_id.0)
        .bind(&self.run_id)
        .bind(&snapshot.position_id.0)
        .bind(position_state_label(&snapshot.state))
        .bind(u64_to_i64(snapshot.block_number))
        .bind(snapshot.observed_block_number.map(u64_to_i64))
        .bind(snapshot.valuation_block_number.map(u64_to_i64))
        .bind(snapshot.current_value_eth.to_string())
        .bind(snapshot.realized_profit_eth.to_string())
        .bind(snapshot.unrealized_profit_eth.to_string())
        .bind(snapshot.roi.to_string())
        .bind(
            snapshot
                .pool_price_to_initial_price_ratio
                .map(|value| value.to_string()),
        )
        .bind(
            snapshot
                .pool_initial_price_denom_per_token
                .map(|value| value.to_string()),
        )
        .bind(
            snapshot
                .pool_price_denom_per_token
                .map(|value| value.to_string()),
        )
        .bind(snapshot.pool_liquidity_denom.map(|value| value.to_string()))
        .bind(snapshot.pool_token_reserve.map(|value| value.to_string()))
        .bind(snapshot.pool_denom_symbol.as_deref())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        sqlx::query(
            r#"
            UPDATE alpha_trading.trades
            SET state = $2,
                latest_snapshot_block = $3,
                latest_observed_block = $4,
                latest_valuation_block = $5,
                current_value_eth = $6,
                realized_pnl_eth = $7,
                unrealized_pnl_eth = $8,
                total_pnl_eth = ((NULLIF($7, '')::numeric + NULLIF($8, '')::numeric)::text),
                roi = $9,
                updated_at = NOW()
            WHERE trade_id = $1
            "#,
        )
        .bind(&snapshot.trade_id.0)
        .bind(position_state_label(&snapshot.state))
        .bind(u64_to_i64(snapshot.block_number))
        .bind(snapshot.observed_block_number.map(u64_to_i64))
        .bind(snapshot.valuation_block_number.map(u64_to_i64))
        .bind(snapshot.current_value_eth.to_string())
        .bind(snapshot.realized_profit_eth.to_string())
        .bind(snapshot.unrealized_profit_eth.to_string())
        .bind(snapshot.roi.to_string())
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn record_execution_report_with_context(
        &self,
        position_id: Option<&PositionId>,
        order_side: Option<OrderSide>,
        report: &ExecutionReport,
    ) -> Result<()> {
        let payload = to_json(report)?;
        let trade_id = self
            .resolve_trade_id(position_id, &report.order_id.0)
            .await?
            .or_else(|| position_id.map(|id| id.0.clone()));
        let (filled_amount_raw, filled_amount_decimals) = report
            .filled_amount
            .as_ref()
            .map(|amount| (Some(amount_raw(amount)), Some(i16::from(amount.decimals))))
            .unwrap_or((None, None));
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.execution_reports (
                run_id, position_id, trade_id, order_side, order_id, status, tx_hash, block_number, filled_amount_raw,
                filled_amount_decimals, gas_used, error, payload, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
            "#,
        )
        .bind(&self.run_id)
        .bind(position_id.map(|id| id.0.as_str()))
        .bind(trade_id.as_deref())
        .bind(order_side.map(order_side_label))
        .bind(&report.order_id.0)
        .bind(execution_status_label(&report.status))
        .bind(report.tx_hash.map(|hash| hash.to_string()))
        .bind(report.block_number.map(u64_to_i64))
        .bind(filled_amount_raw)
        .bind(filled_amount_decimals)
        .bind(report.gas_used.map(u64_to_i64))
        .bind(report.error.as_deref())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        if let (Some(trade_id), Some(side)) = (trade_id.as_deref(), order_side) {
            self.record_trade_event(trade_id, side, report).await?;
            self.apply_report_to_trade(trade_id, side, report).await?;
        }
        Ok(())
    }

    async fn result_set_id(&self) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT result_set_id
            FROM alpha_trading.backtest_result_set_runs
            WHERE run_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&self.run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(store_error)?;
        row.map(|row| {
            row.try_get::<String, _>("result_set_id")
                .map_err(store_error)
        })
        .transpose()?
        .ok_or_else(|| AlphaCoreError::Store(format!("run {} has no result set", self.run_id)))
    }

    async fn resolve_trade_id(
        &self,
        position_id: Option<&PositionId>,
        order_id: &str,
    ) -> Result<Option<String>> {
        if let Some(position_id) = position_id {
            if let Some(row) = sqlx::query(
                r#"
                SELECT trade_id
                FROM alpha_trading.positions
                WHERE run_id = $1 AND position_id = $2
                "#,
            )
            .bind(&self.run_id)
            .bind(&position_id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_error)?
            {
                let trade_id: Option<String> = row.try_get("trade_id").map_err(store_error)?;
                if trade_id.is_some() {
                    return Ok(trade_id);
                }
            }
        }
        let row = sqlx::query(
            r#"
            SELECT trade_id
            FROM alpha_trading.trades
            WHERE run_id = $1
              AND ($2 = entry_order_id OR $2 = exit_order_id)
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(&self.run_id)
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(store_error)?;
        row.map(|row| {
            row.try_get::<Option<String>, _>("trade_id")
                .map_err(store_error)
        })
        .transpose()
        .map(Option::flatten)
    }

    async fn record_trade_event(
        &self,
        trade_id: &str,
        side: OrderSide,
        report: &ExecutionReport,
    ) -> Result<()> {
        let payload = to_json(report)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.trade_events (
                trade_id, run_id, event_type, order_side, status, order_id, tx_hash,
                block_number, filled_amount_raw, filled_amount_decimals, gas_used,
                gas_cost_eth, error, payload, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW())
            "#,
        )
        .bind(trade_id)
        .bind(&self.run_id)
        .bind(trade_event_type(side, &report.status))
        .bind(order_side_label(side))
        .bind(execution_status_label(&report.status))
        .bind(&report.order_id.0)
        .bind(report.tx_hash.map(|hash| hash.to_string()))
        .bind(report.block_number.map(u64_to_i64))
        .bind(report.filled_amount.as_ref().map(amount_raw))
        .bind(
            report
                .filled_amount
                .as_ref()
                .map(|amount| i16::from(amount.decimals)),
        )
        .bind(report.gas_used.map(u64_to_i64))
        .bind(report.gas_cost.as_ref().map(amount_to_eth_string))
        .bind(report.error.as_deref())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn apply_report_to_trade(
        &self,
        trade_id: &str,
        side: OrderSide,
        report: &ExecutionReport,
    ) -> Result<()> {
        let block = report.block_number.map(u64_to_i64);
        let filled_eth = report.filled_amount.as_ref().map(amount_to_eth_string);
        let gas_cost = report.gas_cost.as_ref().map(amount_to_eth_string);
        let status = execution_status_label(&report.status);
        let side_label = order_side_label(side);
        sqlx::query(
            r#"
            UPDATE alpha_trading.trades
            SET state = CASE
                    WHEN $2 = 'buy' AND $3 = 'submitted' THEN 'buy_submitted'
                    WHEN $2 = 'buy' AND $3 = 'confirmed' THEN 'buy_confirmed'
                    WHEN $2 = 'buy' AND $3 = 'failed' THEN 'buy_failed'
                    WHEN $2 = 'buy' AND $3 = 'cancelled' THEN 'buy_cancelled'
                    WHEN $2 = 'sell' AND $3 = 'submitted' THEN 'sell_submitted'
                    WHEN $2 = 'sell' AND $3 = 'confirmed' THEN 'sell_confirmed'
                    WHEN $2 = 'sell' AND $3 = 'failed' THEN 'sell_failed'
                    WHEN $2 = 'sell' AND $3 = 'cancelled' THEN 'sell_cancelled'
                    ELSE state
                END,
                entry_order_id = CASE WHEN $2 = 'buy' THEN $4 ELSE entry_order_id END,
                exit_order_id = CASE WHEN $2 = 'sell' THEN $4 ELSE exit_order_id END,
                entry_block = CASE WHEN $2 = 'buy' AND $3 = 'confirmed' THEN $5 ELSE entry_block END,
                exit_block = CASE WHEN $2 = 'sell' AND $3 = 'confirmed' THEN $5 ELSE exit_block END,
                entry_cost_eth = CASE WHEN $2 = 'buy' AND $3 = 'confirmed' THEN $6 ELSE entry_cost_eth END,
                exit_value_eth = CASE WHEN $2 = 'sell' AND $3 = 'confirmed' THEN $6 ELSE exit_value_eth END,
                gas_cost_eth = CASE
                    WHEN $7 IS NULL THEN gas_cost_eth
                    WHEN gas_cost_eth IS NULL OR gas_cost_eth = '' THEN $7
                    ELSE gas_cost_eth
                END,
                updated_at = NOW()
            WHERE trade_id = $1
            "#,
        )
        .bind(trade_id)
        .bind(side_label)
        .bind(status)
        .bind(&report.order_id.0)
        .bind(block)
        .bind(filled_eth)
        .bind(gas_cost)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }
}

fn to_json<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize,
{
    serde_json::to_value(value).map_err(store_error)
}

fn amount_raw(amount: &Amount) -> String {
    amount.raw.to_string()
}

fn amount_to_eth_string(amount: &Amount) -> String {
    let mut raw = amount.raw.to_string();
    let decimals = usize::from(amount.decimals);
    if decimals == 0 {
        return raw;
    }
    if raw.len() <= decimals {
        let zeros = "0".repeat(decimals + 1 - raw.len());
        raw = format!("{zeros}{raw}");
    }
    let split = raw.len() - decimals;
    let (whole, fraction) = raw.split_at(split);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    }
}

fn normalize_position_pool_id(position: &mut Position) {
    if position.trade_id.0 == "legacy-unset" {
        position.trade_id.0 = position.id.0.clone();
    }
    if position.key.pool_address.as_str().contains(':') {
        return;
    }
    let pool_identity = position.key.pool_address.to_string();
    position.key.pool_address =
        eth_alpha_core::ids::TokenPoolId::new(position.key.token_address, pool_identity);
}

fn result_set_id_for_run(run_id: &str, mode: &str, config: &Value) -> String {
    if let Some(id) = config.get("result_set_id").and_then(Value::as_str) {
        if !id.trim().is_empty() {
            return sanitize_id(id);
        }
    }
    let runtime = config
        .get("strategy_runtime")
        .and_then(Value::as_str)
        .unwrap_or(mode);
    if runtime == "live" || mode == "chain-sim" || mode == "live" {
        if let Some(id) = config.get("live_backtest_id").and_then(Value::as_str) {
            if !id.trim().is_empty() {
                return sanitize_id(id);
            }
        }
        return format!("live-{}", sanitize_id(run_id));
    }
    match (
        config.get("from_block").and_then(Value::as_u64),
        config.get("to_block").and_then(Value::as_u64),
    ) {
        (Some(start), Some(end)) => format!("historical-{start}-{end}"),
        _ => format!("historical-{}", sanitize_id(run_id)),
    }
}

fn result_set_mode(mode: &str, config: &Value) -> &'static str {
    let runtime = config
        .get("strategy_runtime")
        .and_then(Value::as_str)
        .unwrap_or(mode);
    if runtime == "live" || mode == "chain-sim" || mode == "live" {
        "live"
    } else {
        "historical"
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn order_side_label(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    }
}

fn execution_status_label(status: &ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::Submitted => "submitted",
        ExecutionStatus::Pending => "pending",
        ExecutionStatus::Confirmed => "confirmed",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
    }
}

fn trade_event_type(side: OrderSide, status: &ExecutionStatus) -> &'static str {
    match (side, status) {
        (OrderSide::Buy, ExecutionStatus::Submitted) => "buy_submitted",
        (OrderSide::Buy, ExecutionStatus::Pending) => "buy_pending",
        (OrderSide::Buy, ExecutionStatus::Confirmed) => "buy_confirmed",
        (OrderSide::Buy, ExecutionStatus::Failed) => "buy_failed",
        (OrderSide::Buy, ExecutionStatus::Cancelled) => "buy_cancelled",
        (OrderSide::Sell, ExecutionStatus::Submitted) => "sell_submitted",
        (OrderSide::Sell, ExecutionStatus::Pending) => "sell_pending",
        (OrderSide::Sell, ExecutionStatus::Confirmed) => "sell_confirmed",
        (OrderSide::Sell, ExecutionStatus::Failed) => "sell_failed",
        (OrderSide::Sell, ExecutionStatus::Cancelled) => "sell_cancelled",
    }
}

fn position_state_label(state: &PositionState) -> &'static str {
    match state {
        PositionState::Init => "init",
        PositionState::BuyIntentCreated => "buy_intent_created",
        PositionState::BuySubmitted => "buy_submitted",
        PositionState::BuyConfirmed => "buy_confirmed",
        PositionState::BuyFailed => "buy_failed",
        PositionState::BuyCancelled => "buy_cancelled",
        PositionState::SellIntentCreated => "sell_intent_created",
        PositionState::SellSubmitted => "sell_submitted",
        PositionState::SellFailed => "sell_failed",
        PositionState::SellCancelled => "sell_cancelled",
        PositionState::SellConfirmed => "sell_confirmed",
        PositionState::Cancelled => "cancelled",
        PositionState::Scammed => "scammed",
    }
}

fn risk_kind_label(kind: &RiskKind) -> String {
    match kind {
        RiskKind::LiquidityRemoval => "liquidity_removal".to_string(),
        RiskKind::TaxChange => "tax_change".to_string(),
        RiskKind::Honeypot => "honeypot".to_string(),
        RiskKind::TradingDisabled => "trading_disabled".to_string(),
        RiskKind::TradingEnabled => "trading_enabled".to_string(),
        RiskKind::LpApproval => "lp_approval".to_string(),
        RiskKind::ScamConfirmed => "scam_confirmed".to_string(),
        RiskKind::Custom(value) => value.clone(),
    }
}

fn risk_severity_label(severity: &RiskSeverity) -> &'static str {
    match severity {
        RiskSeverity::Info => "info",
        RiskSeverity::Warning => "warning",
        RiskSeverity::Critical => "critical",
    }
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn u32_to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn i64_to_u64(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

fn store_error(error: impl std::fmt::Display) -> AlphaCoreError {
    AlphaCoreError::Store(error.to_string())
}

const MIGRATIONS: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS alpha_trading",
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trader_runs (
        run_id TEXT PRIMARY KEY,
        mode TEXT NOT NULL,
        status TEXT NOT NULL,
        config JSONB NOT NULL DEFAULT '{}'::jsonb,
        metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
        started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        stopped_at TIMESTAMPTZ
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.order_intents (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        trade_id TEXT,
        portfolio_id TEXT NOT NULL,
        wallet_id TEXT NOT NULL,
        strategy_name TEXT NOT NULL,
        side TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT NOT NULL,
        amount_raw TEXT NOT NULL,
        amount_decimals SMALLINT NOT NULL,
        max_slippage_bps INTEGER NOT NULL,
        deadline_secs BIGINT NOT NULL,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS trade_id TEXT",
    r#"
    CREATE INDEX IF NOT EXISTS order_intents_run_created_idx
    ON alpha_trading.order_intents (run_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS order_intents_token_created_idx
    ON alpha_trading.order_intents (token_address, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.execution_reports (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT,
        trade_id TEXT,
        order_side TEXT,
        order_id TEXT NOT NULL,
        status TEXT NOT NULL,
        tx_hash TEXT,
        block_number BIGINT,
        filled_amount_raw TEXT,
        filled_amount_decimals SMALLINT,
        gas_used BIGINT,
        error TEXT,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.execution_reports ADD COLUMN IF NOT EXISTS position_id TEXT",
    "ALTER TABLE alpha_trading.execution_reports ADD COLUMN IF NOT EXISTS trade_id TEXT",
    "ALTER TABLE alpha_trading.execution_reports ADD COLUMN IF NOT EXISTS order_side TEXT",
    r#"
    CREATE INDEX IF NOT EXISTS execution_reports_run_created_idx
    ON alpha_trading.execution_reports (run_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS execution_reports_position_created_idx
    ON alpha_trading.execution_reports (run_id, position_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS execution_reports_order_created_idx
    ON alpha_trading.execution_reports (order_id, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.positions (
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT NOT NULL,
        trade_id TEXT,
        portfolio_id TEXT NOT NULL,
        wallet_id TEXT NOT NULL,
        strategy_name TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT NOT NULL,
        state TEXT NOT NULL,
        entry_order_id TEXT,
        exit_order_id TEXT,
        entry_block BIGINT,
        exit_block BIGINT,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (run_id, position_id)
    )
    "#,
    "ALTER TABLE alpha_trading.positions ADD COLUMN IF NOT EXISTS trade_id TEXT",
    "ALTER TABLE alpha_trading.positions ADD COLUMN IF NOT EXISTS entry_block BIGINT",
    "ALTER TABLE alpha_trading.positions ADD COLUMN IF NOT EXISTS exit_block BIGINT",
    "UPDATE alpha_trading.positions SET trade_id = position_id WHERE trade_id IS NULL",
    r#"
    CREATE INDEX IF NOT EXISTS positions_run_state_idx
    ON alpha_trading.positions (run_id, state, updated_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS positions_token_idx
    ON alpha_trading.positions (token_address, updated_at DESC)
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS positions_run_trade_idx
    ON alpha_trading.positions (run_id, trade_id)
    WHERE trade_id IS NOT NULL
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.position_snapshots (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT NOT NULL,
        trade_id TEXT,
        state TEXT NOT NULL,
        block_number BIGINT NOT NULL,
        observed_block_number BIGINT,
        valuation_block_number BIGINT,
        current_value_eth TEXT NOT NULL,
        realized_profit_eth TEXT NOT NULL,
        unrealized_profit_eth TEXT NOT NULL,
        roi TEXT NOT NULL,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.position_snapshots ADD COLUMN IF NOT EXISTS trade_id TEXT",
    "ALTER TABLE alpha_trading.position_snapshots ADD COLUMN IF NOT EXISTS observed_block_number BIGINT",
    "ALTER TABLE alpha_trading.position_snapshots ADD COLUMN IF NOT EXISTS valuation_block_number BIGINT",
    "UPDATE alpha_trading.position_snapshots SET trade_id = position_id WHERE trade_id IS NULL",
    r#"
    CREATE INDEX IF NOT EXISTS position_snapshots_position_block_idx
    ON alpha_trading.position_snapshots (run_id, position_id, block_number DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.backtest_result_sets (
        result_set_id TEXT PRIMARY KEY,
        mode TEXT NOT NULL,
        status TEXT NOT NULL,
        strategy_suite TEXT,
        start_block BIGINT,
        end_block BIGINT,
        config JSONB NOT NULL DEFAULT '{}'::jsonb,
        metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        stopped_at TIMESTAMPTZ
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.backtest_result_set_runs (
        result_set_id TEXT NOT NULL REFERENCES alpha_trading.backtest_result_sets(result_set_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (result_set_id, run_id)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trades (
        trade_id TEXT PRIMARY KEY,
        result_set_id TEXT NOT NULL REFERENCES alpha_trading.backtest_result_sets(result_set_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT,
        strategy_name TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT NOT NULL,
        state TEXT NOT NULL,
        entry_order_id TEXT,
        exit_order_id TEXT,
        entry_block BIGINT,
        exit_block BIGINT,
        latest_snapshot_block BIGINT,
        latest_observed_block BIGINT,
        latest_valuation_block BIGINT,
        entry_cost_eth TEXT,
        exit_value_eth TEXT,
        current_value_eth TEXT,
        realized_pnl_eth TEXT,
        unrealized_pnl_eth TEXT,
        total_pnl_eth TEXT,
        gas_cost_eth TEXT,
        roi TEXT,
        payload JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trades_result_strategy_idx
    ON alpha_trading.trades (result_set_id, strategy_name, updated_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trades_token_pool_idx
    ON alpha_trading.trades (token_address, pool_address, updated_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trade_events (
        id BIGSERIAL PRIMARY KEY,
        trade_id TEXT NOT NULL REFERENCES alpha_trading.trades(trade_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        event_type TEXT NOT NULL,
        order_side TEXT NOT NULL,
        status TEXT NOT NULL,
        order_id TEXT NOT NULL,
        tx_hash TEXT,
        block_number BIGINT,
        filled_amount_raw TEXT,
        filled_amount_decimals SMALLINT,
        gas_used BIGINT,
        gas_cost_eth TEXT,
        error TEXT,
        payload JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trade_events_trade_created_idx
    ON alpha_trading.trade_events (trade_id, created_at ASC, id ASC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trade_snapshots (
        id BIGSERIAL PRIMARY KEY,
        trade_id TEXT NOT NULL REFERENCES alpha_trading.trades(trade_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT,
        state TEXT NOT NULL,
        block_number BIGINT NOT NULL,
        observed_block_number BIGINT,
        valuation_block_number BIGINT,
        current_value_eth TEXT NOT NULL,
        realized_pnl_eth TEXT NOT NULL,
        unrealized_pnl_eth TEXT NOT NULL,
        total_pnl_eth TEXT NOT NULL,
        roi TEXT NOT NULL,
        pool_price_to_initial_price_ratio TEXT,
        pool_initial_price_denom_per_token TEXT,
        pool_price_denom_per_token TEXT,
        pool_liquidity_denom TEXT,
        pool_token_reserve TEXT,
        pool_denom_symbol TEXT,
        payload JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trade_snapshots_trade_block_idx
    ON alpha_trading.trade_snapshots (trade_id, block_number DESC, id DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.risk_events (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        severity TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT,
        pending_tx_hash TEXT,
        observed_block BIGINT,
        message TEXT NOT NULL,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS risk_events_run_created_idx
    ON alpha_trading.risk_events (run_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS risk_events_token_created_idx
    ON alpha_trading.risk_events (token_address, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.strategy_decisions (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        strategy_name TEXT NOT NULL,
        event_source TEXT NOT NULL,
        event_key TEXT NOT NULL,
        block_number BIGINT,
        token_address TEXT,
        pool_address TEXT,
        action TEXT NOT NULL,
        reason TEXT,
        order_side TEXT,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_decisions_run_event_idx
    ON alpha_trading.strategy_decisions (run_id, event_source, block_number, id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_decisions_token_idx
    ON alpha_trading.strategy_decisions (token_address, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.strategy_observations (
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        strategy_name TEXT NOT NULL,
        event_source TEXT NOT NULL,
        event_key TEXT NOT NULL,
        token_address TEXT,
        pool_address TEXT,
        block_number BIGINT,
        event_timestamp TEXT,
        decision TEXT NOT NULL,
        report_count INTEGER NOT NULL DEFAULT 0,
        payload JSONB NOT NULL,
        first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (run_id, strategy_name, event_source, event_key)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_observations_run_seen_idx
    ON alpha_trading.strategy_observations (run_id, strategy_name, last_seen_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_observations_token_idx
    ON alpha_trading.strategy_observations (token_address, last_seen_at DESC)
    "#,
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn labels_are_dashboard_friendly() {
        assert_eq!(order_side_label(OrderSide::Buy), "buy");
        assert_eq!(
            position_state_label(&PositionState::BuyConfirmed),
            "buy_confirmed"
        );
        assert_eq!(
            execution_status_label(&ExecutionStatus::Confirmed),
            "confirmed"
        );
        assert_eq!(risk_kind_label(&RiskKind::LpApproval), "lp_approval");
    }

    #[test]
    fn migrations_cover_runtime_tables() {
        let combined = MIGRATIONS.join("\n");
        for table in [
            "backtest_result_sets",
            "backtest_result_set_runs",
            "trades",
            "trade_events",
            "trade_snapshots",
            "trader_runs",
            "order_intents",
            "execution_reports",
            "positions",
            "position_snapshots",
            "risk_events",
            "strategy_decisions",
            "strategy_observations",
        ] {
            assert!(combined.contains(table));
        }
    }

    #[test]
    fn heartbeat_metadata_is_json() {
        let metadata = json!({
            "live_status": "live",
            "positions": 3,
        });
        assert_eq!(metadata["live_status"], "live");
    }
}
