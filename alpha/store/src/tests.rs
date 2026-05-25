use super::*;
use eth_alpha_core::{
    amount::DecimalAmount,
    ids::{PortfolioId, PositionId, StrategyName, TokenAddress, TokenPoolId, WalletId},
    market::PoolProtocol,
    position::{Position, PositionKey, PositionState},
    ExecutionStatus, RiskKind,
};
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
        "manual_close_requests",
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

#[test]
fn buy_failed_trade_accounting_records_realized_gas_loss_as_total_pnl() {
    let mut position = sample_position(PositionState::BuyFailed);
    position.gas_cost_eth = DecimalAmount::from_str_exact("0.000035334726801858").unwrap();

    let fields = trade_accounting_fields_for_position(&position);

    assert_eq!(fields.current_value_eth.as_deref(), Some("0"));
    assert_eq!(fields.unrealized_pnl_eth.as_deref(), Some("0"));
    assert_eq!(
        fields.total_pnl_eth.as_deref(),
        Some("-0.000035334726801858")
    );
}

#[test]
fn open_exposure_trade_accounting_waits_for_valuation_snapshot() {
    let mut position = sample_position(PositionState::BuyConfirmed);
    position.gas_cost_eth = DecimalAmount::from_str_exact("0.000035334726801858").unwrap();

    let fields = trade_accounting_fields_for_position(&position);

    assert!(fields.current_value_eth.is_none());
    assert!(fields.unrealized_pnl_eth.is_none());
    assert!(fields.total_pnl_eth.is_none());
}

fn sample_position(state: PositionState) -> Position {
    let mut position = Position::new(
        PositionId("pos_test".to_string()),
        PositionKey {
            portfolio_id: PortfolioId("portfolio".to_string()),
            wallet_id: WalletId("wallet".to_string()),
            strategy_name: StrategyName("strategy".to_string()),
            token_address: TokenAddress::ZERO,
            pool_address: TokenPoolId("0x0000000000000000000000000000000000000000".to_string()),
            protocol: PoolProtocol::UniswapV2,
        },
    );
    position.state = state;
    position
}
