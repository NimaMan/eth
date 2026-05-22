//! PostgreSQL persistence for the alpha trading runtime.

pub mod observations;
pub mod performance;
pub mod strategy_run_results;

mod codec;
mod postgres;
mod runtime;
mod schema;
mod trading_store;

pub use postgres::{
    ActiveHoldCounterRecord, PostgresTradingStore, StrategyObservationCursor,
    StrategyObservationRecord, SubmittedExecutionRecord,
};

pub(crate) use codec::*;
pub(crate) use postgres::DEFAULT_MAX_CONNECTIONS;

use eth_alpha_core::{
    error::{AlphaCoreError, Result},
    execution::ExecutionReport,
    ids::PositionId,
    order::OrderSide,
    position::{Position, PositionSnapshot, PositionState},
};
use sqlx::{Postgres, Row, Transaction};

use schema::MIGRATIONS;

impl PostgresTradingStore {
    async fn upsert_trade_for_position(&self, position: &Position) -> Result<()> {
        let payload = to_json(position)?;
        let result_set_id = self.result_set_id().await?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.trades (
                trade_id, result_set_id, run_id, position_id, strategy_name,
                token_address, pool_address, protocol, state, entry_order_id, exit_order_id,
                entry_block, exit_block, entry_cost_eth, exit_value_eth, gas_cost_eth,
                realized_pnl_eth, payload, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, NOW(), NOW())
            ON CONFLICT (trade_id) DO UPDATE SET
                result_set_id = EXCLUDED.result_set_id,
                run_id = EXCLUDED.run_id,
                position_id = EXCLUDED.position_id,
                strategy_name = EXCLUDED.strategy_name,
                token_address = EXCLUDED.token_address,
                pool_address = EXCLUDED.pool_address,
                protocol = EXCLUDED.protocol,
                state = EXCLUDED.state,
                entry_order_id = EXCLUDED.entry_order_id,
                exit_order_id = EXCLUDED.exit_order_id,
                entry_block = EXCLUDED.entry_block,
                exit_block = EXCLUDED.exit_block,
                entry_cost_eth = EXCLUDED.entry_cost_eth,
                exit_value_eth = EXCLUDED.exit_value_eth,
                gas_cost_eth = EXCLUDED.gas_cost_eth,
                realized_pnl_eth = EXCLUDED.realized_pnl_eth,
                total_pnl_eth = CASE
                    WHEN EXCLUDED.state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                         AND NULLIF(EXCLUDED.realized_pnl_eth, '') IS NOT NULL
                         AND NULLIF(alpha_trading.trades.unrealized_pnl_eth, '') IS NOT NULL
                    THEN (
                        NULLIF(EXCLUDED.realized_pnl_eth, '')::numeric
                        + NULLIF(alpha_trading.trades.unrealized_pnl_eth, '')::numeric
                    )::text
                    ELSE alpha_trading.trades.total_pnl_eth
                END,
                roi = CASE
                    WHEN EXCLUDED.state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                         AND NULLIF(EXCLUDED.entry_cost_eth, '') IS NOT NULL
                         AND NULLIF(EXCLUDED.entry_cost_eth, '')::numeric <> 0
                         AND NULLIF(EXCLUDED.realized_pnl_eth, '') IS NOT NULL
                         AND NULLIF(alpha_trading.trades.unrealized_pnl_eth, '') IS NOT NULL
                    THEN (
                        (
                            NULLIF(EXCLUDED.realized_pnl_eth, '')::numeric
                            + NULLIF(alpha_trading.trades.unrealized_pnl_eth, '')::numeric
                        ) / NULLIF(EXCLUDED.entry_cost_eth, '')::numeric
                    )::text
                    ELSE alpha_trading.trades.roi
                END,
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
        .bind(protocol_label(&position.key.protocol))
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
        if position.state == PositionState::SellFailed {
            if let Some(block_number) = position.last_exit_failure_block {
                self.append_failed_exit_snapshot(position, block_number)
                    .await?;
            }
        }
        Ok(())
    }

    async fn append_failed_exit_snapshot(
        &self,
        position: &Position,
        block_number: u64,
    ) -> Result<()> {
        let block_number = u64_to_i64(block_number);
        sqlx::query(
            r#"
            WITH source AS (
                SELECT t.trade_id,
                       t.run_id,
                       t.position_id,
                       t.state,
                       $2::bigint AS block_number,
                       LEAST(COALESCE(t.latest_observed_block, $2::bigint), $2::bigint) AS observed_block_number,
                       $2::bigint AS valuation_block_number,
                       COALESCE(NULLIF(t.current_value_eth, ''), '0') AS current_value_eth,
                       COALESCE(NULLIF(t.realized_pnl_eth, ''), '0') AS realized_pnl_eth,
                       COALESCE(NULLIF(t.unrealized_pnl_eth, ''), '0') AS unrealized_pnl_eth,
                       (
                           COALESCE(NULLIF(t.realized_pnl_eth, '')::numeric, 0)
                           + COALESCE(NULLIF(t.unrealized_pnl_eth, '')::numeric, 0)
                       )::text AS total_pnl_eth,
                       CASE
                           WHEN NULLIF(t.entry_cost_eth, '') IS NOT NULL
                                AND NULLIF(t.entry_cost_eth, '')::numeric <> 0
                           THEN (
                               (
                                   COALESCE(NULLIF(t.realized_pnl_eth, '')::numeric, 0)
                                   + COALESCE(NULLIF(t.unrealized_pnl_eth, '')::numeric, 0)
                               ) / NULLIF(t.entry_cost_eth, '')::numeric
                           )::text
                           ELSE COALESCE(NULLIF(t.roi, ''), '0')
                       END AS roi
                FROM alpha_trading.trades t
                WHERE t.trade_id = $1
                  AND t.state = 'sell_failed'
            ),
            position_updated AS (
                UPDATE alpha_trading.position_snapshots ps
                SET observed_block_number = source.observed_block_number,
                    current_value_eth = source.current_value_eth,
                    realized_profit_eth = source.realized_pnl_eth,
                    unrealized_profit_eth = source.unrealized_pnl_eth,
                    roi = source.roi,
                    payload = jsonb_build_object(
                        'position_id', source.position_id,
                        'trade_id', source.trade_id,
                        'state', 'SellFailed',
                        'block_number', source.block_number,
                        'observed_block_number', source.observed_block_number,
                        'valuation_block_number', source.valuation_block_number,
                        'current_value_eth', source.current_value_eth,
                        'realized_profit_eth', source.realized_pnl_eth,
                        'unrealized_profit_eth', source.unrealized_pnl_eth,
                        'roi', source.roi
                    )
                FROM source
                WHERE ps.run_id = source.run_id
                  AND ps.position_id = source.position_id
                  AND ps.trade_id = source.trade_id
                  AND ps.state = source.state
                  AND ps.block_number = source.block_number
                  AND ps.valuation_block_number IS NOT DISTINCT FROM source.valuation_block_number
                RETURNING ps.id
            ),
            position_inserted AS (
                INSERT INTO alpha_trading.position_snapshots (
                    run_id, position_id, trade_id, state, block_number,
                    observed_block_number, valuation_block_number, current_value_eth,
                    realized_profit_eth, unrealized_profit_eth, roi, payload, created_at
                )
                SELECT source.run_id,
                       source.position_id,
                       source.trade_id,
                       source.state,
                       source.block_number,
                       source.observed_block_number,
                       source.valuation_block_number,
                       source.current_value_eth,
                       source.realized_pnl_eth,
                       source.unrealized_pnl_eth,
                       source.roi,
                       jsonb_build_object(
                           'position_id', source.position_id,
                           'trade_id', source.trade_id,
                           'state', 'SellFailed',
                           'block_number', source.block_number,
                           'observed_block_number', source.observed_block_number,
                           'valuation_block_number', source.valuation_block_number,
                           'current_value_eth', source.current_value_eth,
                           'realized_profit_eth', source.realized_pnl_eth,
                           'unrealized_profit_eth', source.unrealized_pnl_eth,
                           'roi', source.roi
                       ),
                       NOW()
                FROM source
                WHERE NOT EXISTS (SELECT 1 FROM position_updated)
                  AND NOT EXISTS (
                      SELECT 1
                      FROM alpha_trading.position_snapshots existing
                      WHERE existing.run_id = source.run_id
                        AND existing.position_id = source.position_id
                        AND existing.trade_id = source.trade_id
                        AND existing.state = source.state
                        AND existing.block_number = source.block_number
                        AND existing.valuation_block_number IS NOT DISTINCT FROM source.valuation_block_number
                  )
                RETURNING id
            ),
            position_written AS (
                SELECT id FROM position_updated
                UNION ALL
                SELECT id FROM position_inserted
            ),
            updated AS (
                UPDATE alpha_trading.trade_snapshots ts
                SET observed_block_number = source.observed_block_number,
                    valuation_block_number = source.valuation_block_number,
                    current_value_eth = source.current_value_eth,
                    realized_pnl_eth = source.realized_pnl_eth,
                    unrealized_pnl_eth = source.unrealized_pnl_eth,
                    total_pnl_eth = source.total_pnl_eth,
                    roi = source.roi,
                    pool_price_to_initial_price_ratio = NULL,
                    pool_initial_price_denom_per_token = NULL,
                    pool_price_denom_per_token = NULL,
                    pool_liquidity_denom = NULL,
                    pool_token_reserve = NULL,
                    pool_denom_symbol = NULL,
                    payload = jsonb_build_object(
                        'position_id', source.position_id,
                        'trade_id', source.trade_id,
                        'state', 'SellFailed',
                        'block_number', source.block_number,
                        'observed_block_number', source.observed_block_number,
                        'valuation_block_number', source.valuation_block_number,
                        'current_value_eth', source.current_value_eth,
                        'realized_profit_eth', source.realized_pnl_eth,
                        'unrealized_profit_eth', source.unrealized_pnl_eth,
                        'roi', source.roi
                    )
                FROM source
                WHERE ts.trade_id = source.trade_id
                  AND ts.state = source.state
                  AND ts.block_number = source.block_number
                  AND ts.valuation_block_number IS NOT DISTINCT FROM source.valuation_block_number
                  AND EXISTS (SELECT 1 FROM position_written)
                RETURNING ts.trade_id
            ),
            inserted AS (
                INSERT INTO alpha_trading.trade_snapshots (
                    trade_id, run_id, position_id, state, block_number,
                    observed_block_number, valuation_block_number, current_value_eth,
                    realized_pnl_eth, unrealized_pnl_eth, total_pnl_eth, roi,
                    pool_price_to_initial_price_ratio, pool_initial_price_denom_per_token,
                    pool_price_denom_per_token, pool_liquidity_denom, pool_token_reserve,
                    pool_denom_symbol, payload, created_at
                )
                SELECT source.trade_id,
                       source.run_id,
                       source.position_id,
                       source.state,
                       source.block_number,
                       source.observed_block_number,
                       source.valuation_block_number,
                       source.current_value_eth,
                       source.realized_pnl_eth,
                       source.unrealized_pnl_eth,
                       source.total_pnl_eth,
                       source.roi,
                       NULL,
                       NULL,
                       NULL,
                       NULL,
                       NULL,
                       NULL,
                       jsonb_build_object(
                           'position_id', source.position_id,
                           'trade_id', source.trade_id,
                           'state', 'SellFailed',
                           'block_number', source.block_number,
                           'observed_block_number', source.observed_block_number,
                           'valuation_block_number', source.valuation_block_number,
                           'current_value_eth', source.current_value_eth,
                           'realized_profit_eth', source.realized_pnl_eth,
                           'unrealized_profit_eth', source.unrealized_pnl_eth,
                           'roi', source.roi
                       ),
                       NOW()
                FROM source
                WHERE EXISTS (SELECT 1 FROM position_written)
                  AND NOT EXISTS (SELECT 1 FROM updated)
                  AND NOT EXISTS (
                      SELECT 1
                      FROM alpha_trading.trade_snapshots existing
                      WHERE existing.trade_id = source.trade_id
                        AND existing.state = source.state
                        AND existing.block_number = source.block_number
                        AND existing.valuation_block_number IS NOT DISTINCT FROM source.valuation_block_number
                  )
                RETURNING trade_id
            )
            UPDATE alpha_trading.trades t
            SET latest_snapshot_block = source.block_number,
                latest_observed_block = source.observed_block_number,
                latest_valuation_block = source.valuation_block_number,
                current_value_eth = source.current_value_eth,
                realized_pnl_eth = source.realized_pnl_eth,
                unrealized_pnl_eth = source.unrealized_pnl_eth,
                total_pnl_eth = source.total_pnl_eth,
                roi = source.roi,
                updated_at = NOW()
            FROM source
            WHERE t.trade_id = source.trade_id
              AND (
                  t.latest_snapshot_block IS NULL
                  OR t.latest_snapshot_block <= source.block_number
              )
            "#,
        )
        .bind(&position.trade_id.0)
        .bind(block_number)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn append_trade_snapshot_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        snapshot: &PositionSnapshot,
    ) -> Result<()> {
        let payload = to_json(snapshot)?;
        let update_result = sqlx::query(
            r#"
            UPDATE alpha_trading.trade_snapshots
            SET run_id = $2,
                position_id = $3,
                observed_block_number = $6,
                current_value_eth = $8,
                realized_pnl_eth = $9,
                unrealized_pnl_eth = $10,
                total_pnl_eth = ((NULLIF($9, '')::numeric + NULLIF($10, '')::numeric)::text),
                roi = $11,
                pool_price_to_initial_price_ratio = $12,
                pool_initial_price_denom_per_token = $13,
                pool_price_denom_per_token = $14,
                pool_liquidity_denom = $15,
                pool_token_reserve = $16,
                pool_denom_symbol = $17,
                payload = $18
            WHERE trade_id = $1
              AND state = $4
              AND block_number = $5::bigint
              AND valuation_block_number IS NOT DISTINCT FROM $7::bigint
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
        .bind(&payload)
        .execute(&mut **tx)
        .await
        .map_err(store_error)?;
        let mut rows_affected = update_result.rows_affected();
        if rows_affected == 0 {
            let insert_result = sqlx::query(
                r#"
                INSERT INTO alpha_trading.trade_snapshots (
                    trade_id, run_id, position_id, state, block_number,
                    observed_block_number, valuation_block_number, current_value_eth,
                    realized_pnl_eth, unrealized_pnl_eth, total_pnl_eth, roi,
                    pool_price_to_initial_price_ratio, pool_initial_price_denom_per_token,
                    pool_price_denom_per_token, pool_liquidity_denom, pool_token_reserve,
                    pool_denom_symbol, payload, created_at
                )
                SELECT
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    ((NULLIF($9, '')::numeric + NULLIF($10, '')::numeric)::text),
                    $11, $12, $13, $14, $15, $16, $17, $18, NOW()
                WHERE NOT EXISTS (
                    SELECT 1
                    FROM alpha_trading.trade_snapshots existing
                    WHERE existing.trade_id = $1
                      AND existing.state = $4
                      AND existing.block_number = $5::bigint
                      AND existing.valuation_block_number IS NOT DISTINCT FROM $7::bigint
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
            .bind(&payload)
            .execute(&mut **tx)
            .await
            .map_err(store_error)?;
            rows_affected = insert_result.rows_affected();
        }
        if rows_affected == 0 {
            return Ok(());
        }
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
              AND (
                  latest_snapshot_block IS NULL
                  OR latest_snapshot_block <= $3
              )
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
        .execute(&mut **tx)
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
        let evidence = report.mined_evidence.as_ref();
        let gas_policy_profiles = evidence
            .and_then(|evidence| evidence.gas_policy_profiles.as_ref())
            .and_then(|profiles| serde_json::to_value(profiles).ok());
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.trade_events (
                trade_id, run_id, event_type, order_side, status, order_id, tx_hash,
                block_number, filled_amount_raw, filled_amount_decimals, gas_used,
                gas_cost_eth, gas_policy_action, gas_policy_signal, gas_policy_status,
                gas_policy_profile, gas_policy_profiles, gas_rank_source,
                gas_estimated_max_cost_eth, gas_estimated_priority_spend_eth,
                gas_policy_guard, error, payload, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, NOW()
            )
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
        .bind(evidence.and_then(|evidence| evidence.gas_policy_action.as_deref()))
        .bind(evidence.and_then(|evidence| evidence.gas_policy_signal.as_deref()))
        .bind(evidence.and_then(|evidence| evidence.gas_policy_status.as_deref()))
        .bind(evidence.and_then(|evidence| evidence.gas_policy_profile.as_deref()))
        .bind(gas_policy_profiles)
        .bind(evidence.and_then(|evidence| evidence.gas_rank_source.as_deref()))
        .bind(evidence.and_then(|evidence| evidence.gas_estimated_max_cost_eth.as_deref()))
        .bind(evidence.and_then(|evidence| evidence.gas_estimated_priority_spend_eth.as_deref()))
        .bind(evidence.and_then(|evidence| evidence.gas_policy_guard.as_deref()))
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
                current_value_eth = CASE
                    WHEN $2 = 'sell' AND $3 = 'confirmed' THEN '0'
                    ELSE current_value_eth
                END,
                unrealized_pnl_eth = CASE
                    WHEN $2 = 'sell' AND $3 = 'confirmed' THEN '0'
                    ELSE unrealized_pnl_eth
                END,
                total_pnl_eth = CASE
                    WHEN $2 = 'sell' AND $3 = 'confirmed' THEN realized_pnl_eth
                    ELSE total_pnl_eth
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

#[cfg(test)]
mod tests;
