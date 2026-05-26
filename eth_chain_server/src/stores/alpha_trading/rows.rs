use eyre::{eyre, Result};
use serde_json::Value;
use sqlx::Row;

use super::types::{
    ExecutionReportView, OrderIntentView, PositionDecisionAuditView, PositionView, RiskEventView,
    StrategyDecisionView, TraderRunView,
};

pub(super) fn row_to_run(row: &sqlx::postgres::PgRow) -> Result<TraderRunView> {
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

pub(super) fn row_to_position(row: &sqlx::postgres::PgRow) -> Result<PositionView> {
    Ok(PositionView {
        position_id: text(row, "position_id")?,
        portfolio_id: text(row, "portfolio_id")?,
        wallet_id: text(row, "wallet_id")?,
        strategy_name: text(row, "strategy_name")?,
        token_address: text(row, "token_address")?,
        pool_address: text(row, "pool_address")?,
        protocol: optional_text(row, "protocol")?,
        state: text(row, "state")?,
        entry_order_id: optional_text(row, "entry_order_id")?,
        exit_order_id: optional_text(row, "exit_order_id")?,
        created_at: text(row, "created_at")?,
        updated_at: text(row, "updated_at")?,
        payload: json_text(row, "payload")?,
    })
}

pub(super) fn row_to_order(row: &sqlx::postgres::PgRow) -> Result<OrderIntentView> {
    Ok(OrderIntentView {
        id: int(row, "id")?,
        portfolio_id: text(row, "portfolio_id")?,
        wallet_id: text(row, "wallet_id")?,
        strategy_name: text(row, "strategy_name")?,
        side: text(row, "side")?,
        token_address: text(row, "token_address")?,
        pool_address: text(row, "pool_address")?,
        protocol: optional_text(row, "protocol")?,
        amount_raw: text(row, "amount_raw")?,
        amount_decimals: small_int(row, "amount_decimals")?,
        max_slippage_bps: int32(row, "max_slippage_bps")?,
        deadline_secs: int(row, "deadline_secs")?,
        reason_code: optional_text(row, "reason_code")?,
        reason_category: optional_text(row, "reason_category")?,
        reason_label: optional_text(row, "reason_label")?,
        reason_source: optional_text(row, "reason_source")?,
        reason_details: json_text(row, "reason_details")?,
        created_at: text(row, "created_at")?,
        payload: json_text(row, "payload")?,
    })
}

pub(super) fn row_to_execution_report(row: &sqlx::postgres::PgRow) -> Result<ExecutionReportView> {
    Ok(ExecutionReportView {
        id: int(row, "id")?,
        position_id: optional_text(row, "position_id")?,
        order_side: optional_text(row, "order_side")?,
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

pub(super) fn row_to_risk_event(row: &sqlx::postgres::PgRow) -> Result<RiskEventView> {
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

pub(super) fn row_to_strategy_decision(
    row: &sqlx::postgres::PgRow,
) -> Result<StrategyDecisionView> {
    Ok(StrategyDecisionView {
        id: int(row, "id")?,
        strategy_name: text(row, "strategy_name")?,
        event_source: text(row, "event_source")?,
        event_key: text(row, "event_key")?,
        block_number: optional_int(row, "block_number")?,
        token_address: optional_text(row, "token_address")?,
        pool_address: optional_text(row, "pool_address")?,
        action: text(row, "action")?,
        reason: optional_text(row, "reason")?,
        reason_code: optional_text(row, "reason_code")?,
        reason_category: optional_text(row, "reason_category")?,
        reason_label: optional_text(row, "reason_label")?,
        reason_source: optional_text(row, "reason_source")?,
        reason_details: json_text(row, "reason_details")?,
        order_side: optional_text(row, "order_side")?,
        created_at: text(row, "created_at")?,
        payload: json_text(row, "payload")?,
    })
}

pub(super) fn row_to_position_decision_audit(
    row: &sqlx::postgres::PgRow,
) -> Result<PositionDecisionAuditView> {
    Ok(PositionDecisionAuditView {
        bucket: text(row, "bucket")?,
        rank: int(row, "rank")?,
        position_id: text(row, "position_id")?,
        token_address: text(row, "token_address")?,
        pool_address: text(row, "pool_address")?,
        state: text(row, "state")?,
        entry_block: optional_int(row, "entry_block")?,
        snapshot_block: optional_int(row, "snapshot_block")?,
        pnl_eth: optional_text(row, "pnl_eth")?,
        roi: optional_text(row, "roi")?,
        entry_reason: optional_text(row, "entry_reason")?,
        entry_decision_block: optional_int(row, "entry_decision_block")?,
        exit_reason: optional_text(row, "exit_reason")?,
        exit_decision_block: optional_int(row, "exit_decision_block")?,
        exit_decision_source: optional_text(row, "exit_decision_source")?,
        sell_status: optional_text(row, "sell_status")?,
        sell_report_block: optional_int(row, "sell_report_block")?,
        sell_error: optional_text(row, "sell_error")?,
    })
}

pub(super) fn metadata_bool(metadata: &Value, key: &str) -> Option<bool> {
    metadata.get(key).and_then(Value::as_bool)
}

pub(super) fn metadata_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(super) fn metadata_u64(metadata: &Value, key: &str) -> Option<u64> {
    metadata.get(key).and_then(Value::as_u64)
}

pub(super) fn json_text(row: &sqlx::postgres::PgRow, column: &str) -> Result<Value> {
    let value = text(row, column)?;
    serde_json::from_str(&value).map_err(|err| eyre!("failed to parse {column} json: {err}"))
}

pub(super) fn text(row: &sqlx::postgres::PgRow, column: &str) -> Result<String> {
    row.try_get::<String, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

pub(super) fn optional_text(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<String>> {
    row.try_get::<Option<String>, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

pub(super) fn int(row: &sqlx::postgres::PgRow, column: &str) -> Result<i64> {
    row.try_get::<i64, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

pub(super) fn optional_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<i64>> {
    row.try_get::<Option<i64>, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

pub(super) fn int32(row: &sqlx::postgres::PgRow, column: &str) -> Result<i32> {
    row.try_get::<i32, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

pub(super) fn small_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<i16> {
    row.try_get::<i16, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}

pub(super) fn optional_small_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<i16>> {
    row.try_get::<Option<i16>, _>(column)
        .map_err(|err| eyre!("failed to read {column}: {err}"))
}
