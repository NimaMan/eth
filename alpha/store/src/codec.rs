use eth_alpha_core::{
    amount::Amount,
    error::{AlphaCoreError, Result},
    execution::ExecutionStatus,
    market::PoolProtocol,
    order::OrderSide,
    position::{Position, PositionState},
    risk::{RiskKind, RiskSeverity},
};
use serde_json::Value;

pub(crate) fn to_json<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize,
{
    serde_json::to_value(value).map_err(store_error)
}

pub(crate) fn amount_raw(amount: &Amount) -> String {
    amount.raw.to_string()
}

pub(crate) fn amount_to_eth_string(amount: &Amount) -> String {
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

pub(crate) fn normalize_position_pool_id(position: &mut Position) {
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

pub(crate) fn result_set_id_for_run(run_id: &str, mode: &str, config: &Value) -> String {
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

pub(crate) fn result_set_mode(mode: &str, config: &Value) -> &'static str {
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

pub(crate) fn order_side_label(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    }
}

pub(crate) fn parse_order_side_label(value: &str) -> Result<OrderSide> {
    match value.trim().to_ascii_lowercase().as_str() {
        "buy" => Ok(OrderSide::Buy),
        "sell" => Ok(OrderSide::Sell),
        other => Err(AlphaCoreError::Store(format!(
            "unknown order side label {other:?}"
        ))),
    }
}

pub(crate) fn protocol_label(protocol: &PoolProtocol) -> String {
    protocol.label().into_owned()
}

pub(crate) fn execution_status_label(status: &ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::Submitted => "submitted",
        ExecutionStatus::Pending => "pending",
        ExecutionStatus::Confirmed => "confirmed",
        ExecutionStatus::Deferred => "deferred",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
    }
}

pub(crate) fn trade_event_type(side: OrderSide, status: &ExecutionStatus) -> &'static str {
    match (side, status) {
        (OrderSide::Buy, ExecutionStatus::Submitted) => "buy_submitted",
        (OrderSide::Buy, ExecutionStatus::Pending) => "buy_pending",
        (OrderSide::Buy, ExecutionStatus::Confirmed) => "buy_confirmed",
        (OrderSide::Buy, ExecutionStatus::Deferred) => "buy_deferred",
        (OrderSide::Buy, ExecutionStatus::Failed) => "buy_failed",
        (OrderSide::Buy, ExecutionStatus::Cancelled) => "buy_cancelled",
        (OrderSide::Sell, ExecutionStatus::Submitted) => "sell_submitted",
        (OrderSide::Sell, ExecutionStatus::Pending) => "sell_pending",
        (OrderSide::Sell, ExecutionStatus::Confirmed) => "sell_confirmed",
        (OrderSide::Sell, ExecutionStatus::Deferred) => "sell_deferred",
        (OrderSide::Sell, ExecutionStatus::Failed) => "sell_failed",
        (OrderSide::Sell, ExecutionStatus::Cancelled) => "sell_cancelled",
    }
}

pub(crate) fn position_state_label(state: &PositionState) -> &'static str {
    match state {
        PositionState::Init => "init",
        PositionState::BuyIntentCreated => "buy_intent_created",
        PositionState::BuySubmitted => "buy_submitted",
        PositionState::BuyConfirmed => "buy_confirmed",
        PositionState::BuyDeferred => "buy_deferred",
        PositionState::BuyFailed => "buy_failed",
        PositionState::BuyCancelled => "buy_cancelled",
        PositionState::SellIntentCreated => "sell_intent_created",
        PositionState::SellSubmitted => "sell_submitted",
        PositionState::SellFailed => "sell_failed",
        PositionState::SellCancelled => "sell_cancelled",
        PositionState::SellConfirmed => "sell_confirmed",
        PositionState::Cancelled => "cancelled",
        // Cause-agnostic terminal "closed at zero value" state. Legacy
        // `"terminal_zero"` and `"scammed"` labels are still accepted on read
        // paths for back-compat with already-persisted rows.
        PositionState::ClosedZeroValuation => "closed_zero_valuation",
    }
}

pub(crate) fn risk_kind_label(kind: &RiskKind) -> String {
    match kind {
        RiskKind::LiquidityRemoval => "liquidity_removal".to_string(),
        RiskKind::MempoolLiquidityRemoval => "mempool_liquidity_removal".to_string(),
        RiskKind::TaxChange => "tax_change".to_string(),
        RiskKind::Honeypot => "honeypot".to_string(),
        RiskKind::TradingDisabled => "trading_disabled".to_string(),
        RiskKind::TradingEnabled => "trading_enabled".to_string(),
        RiskKind::LpApproval => "lp_approval".to_string(),
        RiskKind::ScamConfirmed => "scam_confirmed".to_string(),
        RiskKind::Custom(value) => value.clone(),
    }
}

pub(crate) fn risk_severity_label(severity: &RiskSeverity) -> &'static str {
    match severity {
        RiskSeverity::Info => "info",
        RiskSeverity::Warning => "warning",
        RiskSeverity::Critical => "critical",
    }
}

pub(crate) fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(crate) fn u32_to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

pub(crate) fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

pub(crate) fn i64_to_u64(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

pub(crate) fn store_error(error: impl std::fmt::Display) -> AlphaCoreError {
    AlphaCoreError::Store(error.to_string())
}
