use alloy_primitives::Address;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    error::{AlphaCoreError, Result},
    ids::{PoolAddress, PortfolioId, PositionId, StrategyName, TradeId, WalletId},
    market::{PoolProtocol, PoolSnapshot},
    order::OrderIntent,
    position::{Position, PositionKey},
};
use eth_live_trading::{
    GasEstimateConfig, PlannerTxContext, PreparedSellRoute, TxPrepRequestContext,
};
use serde_json::json;

pub(super) fn shadow_route(
    gas_limit: u64,
    simulated_gas_used: u64,
    gas_estimate: &GasEstimateConfig,
) -> Result<PreparedSellRoute> {
    PreparedSellRoute {
        protocol: "live_backtest_chain_sim_gas_policy".to_string(),
        router_address: "0x0000000000000000000000000000000000000000".to_string(),
        calldata: "0x00".to_string(),
        value_wei: "0".to_string(),
        gas_limit,
        estimated_gas_used: None,
        max_slippage_bps: None,
    }
    .with_simulated_gas_used(simulated_gas_used, gas_estimate)
    .map_err(|error| AlphaCoreError::Execution(format!("gas-policy shadow route failed: {error}")))
}

pub(super) fn shadow_input(intent: &OrderIntent) -> eth_live_trading::LivePrioritySellPlannerInput {
    let pool_address = PoolAddress::from(
        "0x0000000000000000000000000000000000000000:0x0000000000000000000000000000000000000000",
    );
    let strategy_name = StrategyName("gas-policy-shadow".to_string());
    let trade_id = TradeId("gas-policy-shadow".to_string());
    let token_address = Address::ZERO;
    eth_live_trading::LivePrioritySellPlannerInput {
        context: PlannerTxContext {
            tx: TxPrepRequestContext {
                chain_id: 1,
                from: Address::ZERO.to_string(),
                strategy_name: strategy_name.0.clone(),
                strategy_run_id: None,
                observed_block: None,
                required_state_block: 0,
                source_metadata: json!({"source": "live_backtest_chain_sim_gas_policy"}),
            },
            current_block: 0,
            deadline_unix_secs: 0,
        },
        intent: OrderIntent {
            trade_id: Some(trade_id.clone()),
            portfolio_id: PortfolioId("gas-policy-shadow".to_string()),
            wallet_id: WalletId("gas-policy-shadow".to_string()),
            strategy_name: strategy_name.clone(),
            side: intent.side,
            token_address,
            pool_address: pool_address.clone(),
            protocol: PoolProtocol::UniswapV2,
            amount: Amount::zero(18),
            route: None,
            max_slippage_bps: 0,
            deadline_secs: 0,
            decision_reason: intent.decision_reason.clone(),
        },
        position: Position::with_trade_id(
            PositionId(trade_id.0.clone()),
            trade_id,
            PositionKey {
                portfolio_id: PortfolioId("gas-policy-shadow".to_string()),
                wallet_id: WalletId("gas-policy-shadow".to_string()),
                strategy_name,
                token_address,
                pool_address: pool_address.clone(),
                protocol: PoolProtocol::UniswapV2,
            },
        ),
        pool: PoolSnapshot {
            address: pool_address,
            token_address,
            protocol: PoolProtocol::UniswapV2,
            denom_address: None,
            denom_symbol: None,
            denom_reserve: DecimalAmount::ZERO,
            token_reserve: DecimalAmount::ZERO,
            price_denom_per_token: None,
            initial_price_denom_per_token: None,
            price_ratio_to_initial: None,
            creation_block: Some(0),
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 0,
            can_buy: false,
            can_sell: false,
            is_scam: false,
        },
        min_output_amount: None,
        source_metadata: json!({"source": "live_backtest_chain_sim_gas_policy"}),
    }
}
