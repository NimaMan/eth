use super::*;
use eth_alpha_core::{ExecutionStatus, RiskKind};
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
    assert_eq!(
        risk_kind_label(&RiskKind::MempoolLiquidityRemoval),
        "mempool_liquidity_removal"
    );
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
