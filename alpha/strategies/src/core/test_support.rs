use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PositionId, TokenPoolId},
    market::{MarketSnapshotRef, PoolProtocol},
    order::OrderSide,
    portfolio::PortfolioState,
    position::{Position, PositionKey},
};
use rust_decimal::Decimal;

use super::super::*;

const WETH_ADDRESS: Address =
    alloy_primitives::address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

pub(super) fn pool() -> PoolSnapshot {
    let token_address = Address::repeat_byte(0x11);
    PoolSnapshot {
        address: TokenPoolId::new(token_address, Address::repeat_byte(0x22).to_string()),
        token_address,
        protocol: PoolProtocol::UniswapV2,
        denom_address: Some(WETH_ADDRESS),
        denom_symbol: Some("WETH".to_string()),
        denom_reserve: Decimal::new(1, 0),
        token_reserve: Decimal::new(100, 0),
        price_denom_per_token: None,
        initial_price_denom_per_token: None,
        price_ratio_to_initial: None,
        creation_block: Some(1),
        token_decimals: None,
        fee_tier: None,
        uniswap_v4: None,
        latest_block: 1,
        can_buy: true,
        can_sell: true,
        is_scam: false,
    }
}

pub(super) fn ctx<'a>(
    market: &'a MarketSnapshotRef,
    portfolio: &'a PortfolioState,
    risks: &'a [RiskEvent],
) -> StrategyContext<'a> {
    StrategyContext {
        market,
        portfolio,
        active_risks: risks,
    }
}

pub(super) fn confirmed_position(strategy: &StrategyEngine, pool: &PoolSnapshot) -> Position {
    let mut position = Position::new(
        PositionId("position-1".to_string()),
        PositionKey {
            portfolio_id: strategy.config.portfolio_id.clone(),
            wallet_id: strategy.config.wallet_id.clone(),
            strategy_name: strategy.name(),
            token_address: pool.token_address,
            pool_address: pool.address.clone(),
            protocol: pool.protocol.clone(),
        },
    );
    position.mark_intent_created(OrderSide::Buy).unwrap();
    position
        .mark_order_submitted(OrderId("buy-1".to_string()), OrderSide::Buy)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id: OrderId("buy-1".to_string()),
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: Some(1),
            filled_amount: Some(Amount {
                raw: Default::default(),
                decimals: 18,
            }),
            token_amount: Some(Amount {
                raw: U256::from(1_000_000u64),
                decimals: 9,
            }),
            gas_used: Some(21_000),
            gas_cost: None,
            mined_evidence: None,
            error: None,
        })
        .unwrap();
    position
}

pub(super) fn lp_approval_risk(pool: &PoolSnapshot, message: impl Into<String>) -> RiskEvent {
    RiskEvent {
        kind: RiskKind::LpApproval,
        severity: RiskSeverity::Warning,
        source: None,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        pending_tx_hash: None,
        observed_block: Some(2),
        message: message.into(),
        evidence: None,
    }
}

pub(super) fn failed_exit_position(strategy: &StrategyEngine, pool: &PoolSnapshot) -> Position {
    let mut position = confirmed_position(strategy, pool);
    position.mark_intent_created(OrderSide::Sell).unwrap();
    position
        .mark_order_submitted(OrderId("sell-1".to_string()), OrderSide::Sell)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id: OrderId("sell-1".to_string()),
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: Some(202),
            filled_amount: None,
            token_amount: None,
            gas_used: Some(21_000),
            gas_cost: None,
            mined_evidence: None,
            error: Some("temporary sell failure".to_string()),
        })
        .unwrap();
    position
}
