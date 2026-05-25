use super::*;
use eth_alpha_core::{
    ids::{OrderId, PoolAddress, PositionId, TradeId},
    market::PoolProtocol,
    position::Position,
};
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};

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
                stopped_at = NULL,
                strategy_suite = COALESCE(EXCLUDED.strategy_suite, alpha_trading.backtest_result_sets.strategy_suite),
                start_block = COALESCE(EXCLUDED.start_block, alpha_trading.backtest_result_sets.start_block),
                end_block = COALESCE(EXCLUDED.end_block, alpha_trading.backtest_result_sets.end_block),
                config = alpha_trading.backtest_result_sets.config || EXCLUDED.config,
                metadata = alpha_trading.backtest_result_sets.metadata - 'reason',
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
        let stale_count: i64 = sqlx::query_scalar(
            r#"
            WITH stale_runs AS (
                UPDATE alpha_trading.trader_runs
                SET status = 'stale',
                    stopped_at = COALESCE(stopped_at, NOW()),
                    metadata = metadata || jsonb_build_object('reason', 'stale_heartbeat')
                WHERE run_id <> $1
                  AND status = 'running'
                  AND last_heartbeat_at < NOW() - ($2::bigint * INTERVAL '1 second')
                RETURNING run_id
            ),
            stale_result_sets AS (
                UPDATE alpha_trading.backtest_result_sets rs
                SET status = 'stale',
                    stopped_at = COALESCE(rs.stopped_at, NOW()),
                    metadata = rs.metadata || jsonb_build_object('reason', 'stale_heartbeat'),
                    updated_at = NOW()
                FROM alpha_trading.backtest_result_set_runs runs
                JOIN stale_runs sr ON sr.run_id = runs.run_id
                WHERE runs.result_set_id = rs.result_set_id
                  AND rs.status = 'running'
                RETURNING rs.result_set_id
            )
            SELECT COUNT(*)::bigint FROM stale_runs
            "#,
        )
        .bind(&self.run_id)
        .bind(u64_to_i64(max_age_secs))
        .fetch_one(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(stale_count.max(0) as u64)
    }

    pub async fn heartbeat(&self, metadata: Value) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE alpha_trading.trader_runs
            SET last_heartbeat_at = NOW(), status = 'running', stopped_at = NULL, metadata = $2
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
                stopped_at = NULL,
                metadata = (rs.metadata - 'reason') || $2,
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

    pub async fn claim_pending_manual_close_requests(
        &self,
        limit: usize,
    ) -> Result<Vec<ManualCloseRequest>> {
        let rows = sqlx::query(
            r#"
            WITH claim AS (
                SELECT request_id
                FROM alpha_trading.manual_close_requests
                WHERE run_id = $1
                  AND status = 'pending'
                ORDER BY created_at ASC
                LIMIT $2
                FOR UPDATE SKIP LOCKED
            )
            UPDATE alpha_trading.manual_close_requests requests
            SET status = 'processing',
                updated_at = NOW()
            FROM claim
            WHERE requests.request_id = claim.request_id
            RETURNING
                requests.request_id,
                requests.run_id,
                requests.strategy_name,
                requests.trade_id,
                requests.position_id,
                requests.token_address,
                requests.pool_address,
                requests.requested_percent,
                requests.requested_raw_amount,
                requests.reason_code,
                requests.payload
            "#,
        )
        .bind(&self.run_id)
        .bind(usize_to_i32(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(ManualCloseRequest {
                    request_id: row.try_get("request_id").map_err(store_error)?,
                    run_id: row.try_get("run_id").map_err(store_error)?,
                    strategy_name: row.try_get("strategy_name").map_err(store_error)?,
                    trade_id: row.try_get("trade_id").map_err(store_error)?,
                    position_id: row.try_get("position_id").map_err(store_error)?,
                    token_address: row.try_get("token_address").map_err(store_error)?,
                    pool_address: row.try_get("pool_address").map_err(store_error)?,
                    requested_percent: row.try_get("requested_percent").map_err(store_error)?,
                    requested_raw_amount: row
                        .try_get("requested_raw_amount")
                        .map_err(store_error)?,
                    reason_code: row.try_get("reason_code").map_err(store_error)?,
                    payload: row.try_get("payload").map_err(store_error)?,
                })
            })
            .collect()
    }

    pub async fn mark_manual_close_request_processed(&self, request_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE alpha_trading.manual_close_requests
            SET status = 'processed',
                processed_at = NOW(),
                updated_at = NOW(),
                error = NULL
            WHERE run_id = $1
              AND request_id = $2
            "#,
        )
        .bind(&self.run_id)
        .bind(request_id)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    pub async fn mark_manual_close_request_failed(
        &self,
        request_id: &str,
        error: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE alpha_trading.manual_close_requests
            SET status = 'failed',
                processed_at = NOW(),
                updated_at = NOW(),
                error = $3
            WHERE run_id = $1
              AND request_id = $2
            "#,
        )
        .bind(&self.run_id)
        .bind(request_id)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    pub async fn load_active_positions(&self, strategy_name: &str) -> Result<Vec<Position>> {
        let rows = sqlx::query(
            r#"
            SELECT protocol, payload::text AS payload
            FROM alpha_trading.positions
            WHERE run_id = $1
              AND strategy_name = $2
              AND state NOT IN ('sell_confirmed', 'buy_deferred', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
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
                let protocol = row
                    .try_get::<Option<String>, _>("protocol")
                    .map_err(store_error)?;
                if let Some(protocol) = protocol.as_deref().filter(|value| !value.trim().is_empty())
                {
                    position.key.protocol = PoolProtocol::from_label(protocol);
                }
                normalize_position_pool_id(&mut position);
                Ok(position)
            })
            .collect()
    }

    pub async fn load_terminal_positions(&self, strategy_name: &str) -> Result<Vec<Position>> {
        let rows = sqlx::query(
            r#"
            SELECT protocol, payload::text AS payload
            FROM alpha_trading.positions
            WHERE run_id = $1
              AND strategy_name = $2
              AND state IN ('sell_confirmed', 'buy_deferred', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed')
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
                let protocol = row
                    .try_get::<Option<String>, _>("protocol")
                    .map_err(store_error)?;
                if let Some(protocol) = protocol.as_deref().filter(|value| !value.trim().is_empty())
                {
                    position.key.protocol = PoolProtocol::from_label(protocol);
                }
                normalize_position_pool_id(&mut position);
                Ok(position)
            })
            .collect()
    }

    pub async fn load_submitted_executions(
        &self,
        limit: usize,
    ) -> Result<Vec<SubmittedExecutionRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (er.order_id)
                   er.order_id,
                   er.tx_hash,
                   er.block_number AS submitted_block_number,
                   er.position_id,
                   er.trade_id,
                   er.order_side,
                   positions.token_address,
                   er.payload #>> '{mined_evidence,selected_gas_limit}' AS selected_gas_limit,
                   er.payload #>> '{mined_evidence,selected_max_fee_per_gas_wei}' AS selected_max_fee_per_gas_wei,
                   er.payload #>> '{mined_evidence,selected_max_priority_fee_per_gas_wei}' AS selected_max_priority_fee_per_gas_wei,
                   er.payload #>> '{mined_evidence,selected_bribe_priority_fee_per_gas_wei}' AS selected_bribe_priority_fee_per_gas_wei,
                   er.payload #>> '{mined_evidence,selected_bribe_max_fee_per_gas_wei}' AS selected_bribe_max_fee_per_gas_wei,
                   er.payload #>> '{mined_evidence,gas_policy_action}' AS gas_policy_action,
                   er.payload #>> '{mined_evidence,gas_policy_signal}' AS gas_policy_signal,
                   er.payload #>> '{mined_evidence,gas_policy_status}' AS gas_policy_status,
                   er.payload #>> '{mined_evidence,gas_policy_profile}' AS gas_policy_profile,
                   er.payload #> '{mined_evidence,gas_policy_profiles}' AS gas_policy_profiles,
                   er.payload #>> '{mined_evidence,gas_rank_source}' AS gas_rank_source,
                   er.payload #>> '{mined_evidence,gas_estimated_max_cost_eth}' AS gas_estimated_max_cost_eth,
                   er.payload #>> '{mined_evidence,gas_estimated_priority_spend_eth}' AS gas_estimated_priority_spend_eth,
                   er.payload #>> '{mined_evidence,gas_policy_guard}' AS gas_policy_guard,
                   er.payload #>> '{mined_evidence,private_execution_transport}' AS private_execution_transport,
                   er.payload #>> '{mined_evidence,bundle_hash}' AS bundle_hash,
                   er.payload #>> '{mined_evidence,bundle_target_block}' AS bundle_target_block,
                   er.payload #>> '{mined_evidence,bundle_max_block}' AS bundle_max_block,
                   er.payload #>> '{mined_evidence,gas_policy_tail_after_tx_hash}' AS gas_policy_tail_after_tx_hash,
                   er.payload #>> '{mined_evidence,gas_policy_dependency_priority_fee_wei}' AS gas_policy_dependency_priority_fee_wei,
                   er.payload #>> '{mined_evidence,gas_policy_dependency_gas_price_wei}' AS gas_policy_dependency_gas_price_wei
            FROM alpha_trading.execution_reports er
            JOIN alpha_trading.positions positions
              ON positions.run_id = er.run_id
             AND positions.position_id = er.position_id
            WHERE er.run_id = $1
              AND er.status = 'submitted'
              AND er.tx_hash IS NOT NULL
              AND er.position_id IS NOT NULL
              AND er.order_side IN ('buy', 'sell')
              AND (
                    (er.order_side = 'buy' AND positions.state = 'buy_submitted')
                 OR (er.order_side = 'sell' AND positions.state = 'sell_submitted')
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.execution_reports final
                  WHERE final.run_id = er.run_id
                    AND final.order_id = er.order_id
                    AND final.status IN ('confirmed', 'deferred', 'failed', 'cancelled')
              )
            ORDER BY er.order_id, er.created_at DESC, er.id DESC
            LIMIT $2
            "#,
        )
        .bind(&self.run_id)
        .bind(usize_to_i32(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;

        rows.into_iter()
            .map(|row| {
                let order_id = OrderId(row.try_get::<String, _>("order_id").map_err(store_error)?);
                let tx_hash = row
                    .try_get::<String, _>("tx_hash")
                    .map_err(store_error)?
                    .parse()
                    .map_err(store_error)?;
                let submitted_block_number = row
                    .try_get::<Option<i64>, _>("submitted_block_number")
                    .map_err(store_error)?
                    .and_then(i64_to_u64);
                let position_id = PositionId(
                    row.try_get::<String, _>("position_id")
                        .map_err(store_error)?,
                );
                let trade_id = row
                    .try_get::<Option<String>, _>("trade_id")
                    .map_err(store_error)?
                    .map(TradeId);
                let order_side = parse_order_side_label(
                    &row.try_get::<String, _>("order_side")
                        .map_err(store_error)?,
                )?;
                let token_address = row
                    .try_get::<String, _>("token_address")
                    .map_err(store_error)?
                    .parse()
                    .map_err(store_error)?;
                let selected_gas_limit = row
                    .try_get::<Option<String>, _>("selected_gas_limit")
                    .map_err(store_error)?;
                let selected_max_fee_per_gas_wei = row
                    .try_get::<Option<String>, _>("selected_max_fee_per_gas_wei")
                    .map_err(store_error)?;
                let selected_max_priority_fee_per_gas_wei = row
                    .try_get::<Option<String>, _>("selected_max_priority_fee_per_gas_wei")
                    .map_err(store_error)?;
                let selected_bribe_priority_fee_per_gas_wei = row
                    .try_get::<Option<String>, _>("selected_bribe_priority_fee_per_gas_wei")
                    .map_err(store_error)?;
                let selected_bribe_max_fee_per_gas_wei = row
                    .try_get::<Option<String>, _>("selected_bribe_max_fee_per_gas_wei")
                    .map_err(store_error)?;
                let gas_policy_action = row
                    .try_get::<Option<String>, _>("gas_policy_action")
                    .map_err(store_error)?;
                let gas_policy_signal = row
                    .try_get::<Option<String>, _>("gas_policy_signal")
                    .map_err(store_error)?;
                let gas_policy_status = row
                    .try_get::<Option<String>, _>("gas_policy_status")
                    .map_err(store_error)?;
                let gas_policy_profile = row
                    .try_get::<Option<String>, _>("gas_policy_profile")
                    .map_err(store_error)?;
                let gas_policy_profiles = row
                    .try_get::<Option<Value>, _>("gas_policy_profiles")
                    .map_err(store_error)?
                    .and_then(json_string_array);
                let gas_rank_source = row
                    .try_get::<Option<String>, _>("gas_rank_source")
                    .map_err(store_error)?;
                let gas_estimated_max_cost_eth = row
                    .try_get::<Option<String>, _>("gas_estimated_max_cost_eth")
                    .map_err(store_error)?;
                let gas_estimated_priority_spend_eth = row
                    .try_get::<Option<String>, _>("gas_estimated_priority_spend_eth")
                    .map_err(store_error)?;
                let gas_policy_guard = row
                    .try_get::<Option<String>, _>("gas_policy_guard")
                    .map_err(store_error)?;
                let private_execution_transport = row
                    .try_get::<Option<String>, _>("private_execution_transport")
                    .map_err(store_error)?;
                let bundle_hash = row
                    .try_get::<Option<String>, _>("bundle_hash")
                    .map_err(store_error)?;
                let bundle_target_block = row
                    .try_get::<Option<String>, _>("bundle_target_block")
                    .map_err(store_error)?
                    .and_then(|value| value.parse::<u64>().ok());
                let bundle_max_block = row
                    .try_get::<Option<String>, _>("bundle_max_block")
                    .map_err(store_error)?
                    .and_then(|value| value.parse::<u64>().ok());
                let gas_policy_tail_after_tx_hash = row
                    .try_get::<Option<String>, _>("gas_policy_tail_after_tx_hash")
                    .map_err(store_error)?;
                let gas_policy_dependency_priority_fee_wei = row
                    .try_get::<Option<String>, _>("gas_policy_dependency_priority_fee_wei")
                    .map_err(store_error)?;
                let gas_policy_dependency_gas_price_wei = row
                    .try_get::<Option<String>, _>("gas_policy_dependency_gas_price_wei")
                    .map_err(store_error)?;
                Ok(SubmittedExecutionRecord {
                    order_id,
                    tx_hash,
                    submitted_block_number,
                    position_id,
                    trade_id,
                    order_side,
                    token_address,
                    selected_gas_limit,
                    selected_max_fee_per_gas_wei,
                    selected_max_priority_fee_per_gas_wei,
                    selected_bribe_priority_fee_per_gas_wei,
                    selected_bribe_max_fee_per_gas_wei,
                    gas_policy_action,
                    gas_policy_signal,
                    gas_policy_status,
                    gas_policy_profile,
                    gas_policy_profiles,
                    gas_rank_source,
                    gas_estimated_max_cost_eth,
                    gas_estimated_priority_spend_eth,
                    gas_policy_guard,
                    private_execution_transport,
                    bundle_hash,
                    bundle_target_block,
                    bundle_max_block,
                    gas_policy_tail_after_tx_hash,
                    gas_policy_dependency_priority_fee_wei,
                    gas_policy_dependency_gas_price_wei,
                })
            })
            .collect()
    }

    pub async fn load_chain_sim_submitted_executions(
        &self,
        limit: usize,
    ) -> Result<Vec<ChainSimSubmittedExecutionRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (er.order_id)
                   er.order_id,
                   er.block_number AS submitted_block_number,
                   er.position_id,
                   er.trade_id,
                   er.order_side,
                   positions.token_address,
                   er.payload #>> '{mined_evidence,expected_confirmation_block}' AS expected_confirmation_block,
                   oi.payload::text AS intent_payload
            FROM alpha_trading.execution_reports er
            JOIN alpha_trading.positions positions
              ON positions.run_id = er.run_id
             AND positions.position_id = er.position_id
            JOIN LATERAL (
                SELECT payload
                FROM alpha_trading.order_intents oi
                WHERE oi.run_id = er.run_id
                  AND oi.trade_id = er.trade_id
                  AND oi.side = er.order_side
                  AND oi.created_at <= er.created_at
                ORDER BY oi.created_at DESC, oi.id DESC
                LIMIT 1
            ) oi ON TRUE
            WHERE er.run_id = $1
              AND er.status = 'submitted'
              AND er.position_id IS NOT NULL
              AND er.order_side IN ('buy', 'sell')
              AND er.payload #>> '{mined_evidence,receipt_status}' = 'live_backtest_chain_sim_submitted'
              AND (
                    (er.order_side = 'buy' AND positions.state = 'buy_submitted')
                 OR (er.order_side = 'sell' AND positions.state = 'sell_submitted')
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.execution_reports final
                  WHERE final.run_id = er.run_id
                    AND final.order_id = er.order_id
                    AND final.status IN ('confirmed', 'deferred', 'failed', 'cancelled')
              )
            ORDER BY er.order_id, er.created_at DESC, er.id DESC
            LIMIT $2
            "#,
        )
        .bind(&self.run_id)
        .bind(usize_to_i32(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;

        rows.into_iter()
            .map(|row| {
                let order_id = OrderId(row.try_get::<String, _>("order_id").map_err(store_error)?);
                let submitted_block_number = row
                    .try_get::<Option<i64>, _>("submitted_block_number")
                    .map_err(store_error)?
                    .and_then(i64_to_u64);
                let expected_confirmation_block = row
                    .try_get::<Option<String>, _>("expected_confirmation_block")
                    .map_err(store_error)?
                    .and_then(|value| value.parse::<u64>().ok());
                let position_id = PositionId(
                    row.try_get::<String, _>("position_id")
                        .map_err(store_error)?,
                );
                let trade_id = row
                    .try_get::<Option<String>, _>("trade_id")
                    .map_err(store_error)?
                    .map(TradeId);
                let order_side = parse_order_side_label(
                    &row.try_get::<String, _>("order_side")
                        .map_err(store_error)?,
                )?;
                let token_address = row
                    .try_get::<String, _>("token_address")
                    .map_err(store_error)?
                    .parse()
                    .map_err(store_error)?;
                let intent_payload = row
                    .try_get::<String, _>("intent_payload")
                    .map_err(store_error)?;
                let intent = serde_json::from_str(&intent_payload).map_err(store_error)?;

                Ok(ChainSimSubmittedExecutionRecord {
                    order_id,
                    submitted_block_number,
                    expected_confirmation_block,
                    position_id,
                    trade_id,
                    order_side,
                    token_address,
                    intent,
                })
            })
            .collect()
    }

    pub async fn load_active_hold_counters(
        &self,
        strategy_name: &str,
    ) -> Result<Vec<ActiveHoldCounterRecord>> {
        let rows = sqlx::query(
            r#"
            WITH active_positions AS (
                SELECT position_id, token_address, pool_address, entry_block
                FROM alpha_trading.positions
                WHERE run_id = $1
                  AND strategy_name = $2
                  AND state = 'buy_confirmed'
                  AND entry_block IS NOT NULL
            )
            SELECT p.position_id,
                   count(DISTINCT sd.block_number) FILTER (
                       WHERE sd.reason = 'position_open_no_exit'
                         AND sd.action = 'hold'
                         AND sd.block_number IS NOT NULL
                   ) AS active_hold_blocks,
                   max(sd.block_number) FILTER (
                       WHERE sd.reason = 'position_open_no_exit'
                         AND sd.action = 'hold'
                   ) AS last_active_hold_block
            FROM active_positions p
            LEFT JOIN alpha_trading.strategy_decisions sd
              ON sd.run_id = $1
             AND sd.strategy_name = $2
             AND lower(sd.token_address) = lower(p.token_address)
             AND lower(sd.pool_address) = lower(p.pool_address)
             AND sd.block_number >= p.entry_block
            GROUP BY p.position_id
            ORDER BY p.position_id
            "#,
        )
        .bind(&self.run_id)
        .bind(strategy_name)
        .fetch_all(&self.pool)
        .await
        .map_err(store_error)?;

        rows.into_iter()
            .map(|row| {
                let count = row
                    .try_get::<i64, _>("active_hold_blocks")
                    .map_err(store_error)?;
                Ok(ActiveHoldCounterRecord {
                    position_id: PositionId(
                        row.try_get::<String, _>("position_id")
                            .map_err(store_error)?,
                    ),
                    count: i64_to_u64(count).unwrap_or_default(),
                    last_block: row
                        .try_get::<Option<i64>, _>("last_active_hold_block")
                        .map_err(store_error)?
                        .and_then(i64_to_u64),
                })
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
              AND state <> 'buy_deferred'
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

fn json_string_array(value: Value) -> Option<Vec<String>> {
    let values = value.as_array()?;
    let strings = values
        .iter()
        .filter_map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .or_else(|| Some(value.to_string()))
                .filter(|text| !text.trim().is_empty())
        })
        .collect::<Vec<_>>();
    (!strings.is_empty()).then_some(strings)
}
