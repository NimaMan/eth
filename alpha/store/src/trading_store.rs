use async_trait::async_trait;
use eth_alpha_core::{
    error::Result,
    execution::ExecutionReport,
    ids::PositionId,
    order::{OrderIntent, OrderSide},
    position::{Position, PositionSnapshot},
    risk::RiskEvent,
    store::{StrategyDecisionRecord, TradingStore},
};

use crate::{
    amount_raw, order_side_label, position_state_label, protocol_label, risk_kind_label,
    risk_severity_label, store_error, to_json, u32_to_i32, u64_to_i64, PostgresTradingStore,
};

#[async_trait]
impl TradingStore for PostgresTradingStore {
    async fn upsert_position(&self, position: &Position) -> Result<()> {
        let payload = to_json(position)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.positions (
                run_id, position_id, trade_id, portfolio_id, wallet_id, strategy_name,
                token_address, pool_address, protocol, state, entry_order_id, exit_order_id,
                entry_block, exit_block, payload, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), NOW())
            ON CONFLICT (run_id, position_id) DO UPDATE SET
                trade_id = EXCLUDED.trade_id,
                portfolio_id = EXCLUDED.portfolio_id,
                wallet_id = EXCLUDED.wallet_id,
                strategy_name = EXCLUDED.strategy_name,
                token_address = EXCLUDED.token_address,
                pool_address = EXCLUDED.pool_address,
                protocol = EXCLUDED.protocol,
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
        .bind(protocol_label(&position.key.protocol))
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
        let mut tx = self.pool.begin().await.map_err(store_error)?;
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.position_snapshots (
                run_id, position_id, trade_id, state, block_number,
                observed_block_number, valuation_block_number, current_value_eth,
                realized_profit_eth, unrealized_profit_eth, roi, payload, created_at
            )
            SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW()
            WHERE NOT EXISTS (
                SELECT 1
                FROM alpha_trading.position_snapshots existing
                WHERE existing.run_id = $1
                  AND existing.position_id = $2
                  AND existing.state = $4
                  AND existing.block_number = $5::bigint
                  AND existing.valuation_block_number IS NOT DISTINCT FROM $7::bigint
            )
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
        .execute(&mut *tx)
        .await
        .map_err(store_error)?;
        self.append_trade_snapshot_in_tx(&mut tx, snapshot).await?;
        tx.commit().await.map_err(store_error)?;
        Ok(())
    }

    async fn record_order_intent(&self, intent: &OrderIntent) -> Result<()> {
        let payload = to_json(intent)?;
        let decision_reason = intent.decision_reason.as_ref();
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.order_intents (
                run_id, trade_id, portfolio_id, wallet_id, strategy_name, side, token_address,
                pool_address, protocol, amount_raw, amount_decimals, max_slippage_bps,
                deadline_secs, reason_code, reason_category, reason_label, reason_source,
                reason_details, payload, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, NOW()
            )
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
        .bind(protocol_label(&intent.protocol))
        .bind(amount_raw(&intent.amount))
        .bind(i16::from(intent.amount.decimals))
        .bind(u32_to_i32(intent.max_slippage_bps))
        .bind(u64_to_i64(intent.deadline_secs))
        .bind(decision_reason.map(|reason| reason.code.as_str()))
        .bind(decision_reason.map(|reason| reason.category_key()))
        .bind(decision_reason.map(|reason| reason.label.as_str()))
        .bind(decision_reason.and_then(|reason| reason.source.as_deref()))
        .bind(decision_reason.map(|reason| &reason.details))
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
                token_address, pool_address, action, reason,
                reason_code, reason_category, reason_label, reason_source, reason_details,
                order_side, payload, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16, NOW()
            )
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
        .bind(&record.reason_code)
        .bind(&record.reason_category)
        .bind(&record.reason_label)
        .bind(&record.reason_source)
        .bind(&record.reason_details)
        .bind(record.order_side.map(order_side_label))
        .bind(&record.payload)
        .execute(&self.pool)
        .await
        .map_err(store_error)?;
        Ok(())
    }
}
