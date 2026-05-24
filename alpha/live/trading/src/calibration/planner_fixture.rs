use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PoolAddress, PortfolioId, PositionId, StrategyName, TradeId, WalletId},
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    position::{Position, PositionKey, PositionState},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    FixedGasRankProvider, FixedPreSubmitSimulator, GasRankPlan, LiveDirectRawTransactionRequest,
    LivePrioritySellPlanner, LivePrioritySellPlannerConfig, LivePrioritySellPlannerError,
    LivePrioritySellPlannerInput, PlannerTxContext, PreSubmitSimulation, PrioritySellPlanner,
    PrioritySellPlannerOutcome, RankedFeeCandidate, RouteBuildRequest, StaticAllowanceChecker,
    TxPrepRequestContext, UniswapV2SellRouteBuilder, UniswapV2TradingVaultSellRouteBuilder,
    VaultInternalAllowanceChecker, UNISWAP_V2_DIRECT_SELL_GAS_LIMIT,
    UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
};

const DEFAULT_BLOCK: u64 = 25_128_246;
const DEFAULT_DEADLINE_UNIX_SECS: u64 = 1_800_000_000;
const DEFAULT_STRATEGY_NAME: &str = "snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold";
const DEFAULT_STRATEGY_RUN_ID: &str = "kartal-calibration-planner-fixture";
const DEFAULT_VAULT_ADDRESS: Address =
    alloy_primitives::address!("0000000000000000000000000000000000000002");
const WETH_ADDRESS: Address =
    alloy_primitives::address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerCalibrationRoute {
    DirectUniswapV2,
    TradingVaultUniswapV2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannerCalibrationFixtureConfig {
    pub from: Address,
    pub route: PlannerCalibrationRoute,
    pub vault_address: Address,
    pub strategy_name: String,
    pub strategy_run_id: Option<String>,
    pub current_block: u64,
    pub deadline_unix_secs: u64,
}

impl PlannerCalibrationFixtureConfig {
    pub fn new(from: Address) -> Self {
        Self {
            from,
            route: PlannerCalibrationRoute::TradingVaultUniswapV2,
            vault_address: DEFAULT_VAULT_ADDRESS,
            strategy_name: DEFAULT_STRATEGY_NAME.to_string(),
            strategy_run_id: Some(DEFAULT_STRATEGY_RUN_ID.to_string()),
            current_block: DEFAULT_BLOCK,
            deadline_unix_secs: DEFAULT_DEADLINE_UNIX_SECS,
        }
    }
}

pub async fn build_planner_calibration_request(
    config: PlannerCalibrationFixtureConfig,
) -> Result<LiveDirectRawTransactionRequest, PlannerCalibrationFixtureError> {
    let input = planner_input(&config);
    let outcome = match config.route {
        PlannerCalibrationRoute::DirectUniswapV2 => {
            let planner = LivePrioritySellPlanner::new(
                planner_config(),
                UniswapV2SellRouteBuilder::new(RouteBuildRequest::new(
                    UNISWAP_V2_DIRECT_SELL_GAS_LIMIT,
                )),
                simulator(),
                gas_rank(),
                StaticAllowanceChecker::pre_approved(),
            );
            planner.plan_priority_sell(input).await?
        }
        PlannerCalibrationRoute::TradingVaultUniswapV2 => {
            let planner = LivePrioritySellPlanner::new(
                planner_config(),
                UniswapV2TradingVaultSellRouteBuilder::with_gas_limit(
                    config.vault_address,
                    UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
                ),
                simulator(),
                gas_rank(),
                VaultInternalAllowanceChecker,
            );
            planner.plan_priority_sell(input).await?
        }
    };

    match outcome {
        PrioritySellPlannerOutcome::Submit { signal, .. } => {
            Ok(signal.request_with_strategy_metadata())
        }
        PrioritySellPlannerOutcome::Reject(reject) => Err(
            PlannerCalibrationFixtureError::PlannerRejected(reject.reason),
        ),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlannerCalibrationFixtureError {
    #[error(transparent)]
    Planner(#[from] LivePrioritySellPlannerError),
    #[error("planner rejected calibration fixture: {0}")]
    PlannerRejected(String),
}

fn planner_config() -> LivePrioritySellPlannerConfig {
    LivePrioritySellPlannerConfig::default()
}

fn planner_input(config: &PlannerCalibrationFixtureConfig) -> LivePrioritySellPlannerInput {
    let token = token();
    let pool_address = pool_address();
    let strategy_name = StrategyName(config.strategy_name.clone());
    let trade_id = TradeId("kartal-calibration-trade-1".to_string());

    LivePrioritySellPlannerInput {
        context: PlannerTxContext {
            tx: TxPrepRequestContext {
                chain_id: 1,
                from: config.from.to_string(),
                strategy_name: config.strategy_name.clone(),
                strategy_run_id: config.strategy_run_id.clone(),
                observed_block: Some(config.current_block),
                source_metadata: json!({
                    "calibration": true,
                    "fixture": "planner_produced_priority_sell",
                }),
            },
            current_block: config.current_block,
            deadline_unix_secs: config.deadline_unix_secs,
        },
        intent: OrderIntent {
            trade_id: Some(trade_id.clone()),
            portfolio_id: PortfolioId("kartal-calibration-portfolio".to_string()),
            wallet_id: WalletId("kartal-calibration-wallet".to_string()),
            strategy_name: strategy_name.clone(),
            side: OrderSide::Sell,
            token_address: token,
            pool_address: pool_address.clone(),
            protocol: PoolProtocol::UniswapV2,
            amount: Amount {
                raw: U256::from(1_000_000_000_000_000_000u128),
                decimals: 18,
            },
            route: None,
            max_slippage_bps: 500,
            deadline_secs: 60,
            decision_reason: None,
        },
        position: position(strategy_name, trade_id, token, pool_address.clone()),
        pool: pool(token, pool_address),
        min_output_amount: Some("9000000000000000".to_string()),
        source_metadata: json!({
            "calibration": true,
            "fixture_input": "synthetic_open_position",
        }),
    }
}

fn position(
    strategy_name: StrategyName,
    trade_id: TradeId,
    token: Address,
    pool_address: PoolAddress,
) -> Position {
    let mut position = Position::with_trade_id(
        PositionId(trade_id.0.clone()),
        trade_id,
        PositionKey {
            portfolio_id: PortfolioId("kartal-calibration-portfolio".to_string()),
            wallet_id: WalletId("kartal-calibration-wallet".to_string()),
            strategy_name,
            token_address: token,
            pool_address,
            protocol: PoolProtocol::UniswapV2,
        },
    );
    position.state = PositionState::BuyConfirmed;
    position
}

fn pool(token: Address, pool_address: PoolAddress) -> PoolSnapshot {
    PoolSnapshot {
        address: pool_address,
        token_address: token,
        protocol: PoolProtocol::UniswapV2,
        denom_address: Some(WETH_ADDRESS),
        denom_symbol: Some("WETH".to_string()),
        denom_reserve: DecimalAmount::from(10),
        token_reserve: DecimalAmount::from(100),
        price_denom_per_token: Some(DecimalAmount::new(1, 1)),
        initial_price_denom_per_token: Some(DecimalAmount::new(1, 1)),
        price_ratio_to_initial: Some(DecimalAmount::from(1)),
        creation_block: Some(DEFAULT_BLOCK),
        token_decimals: Some(18),
        fee_tier: None,
        uniswap_v4: None,
        latest_block: DEFAULT_BLOCK,
        can_buy: true,
        can_sell: true,
        is_scam: false,
    }
}

fn simulator() -> FixedPreSubmitSimulator {
    FixedPreSubmitSimulator::new(PreSubmitSimulation {
        block_number: DEFAULT_BLOCK,
        block_hash: Some("0xabc".to_string()),
        state_root: None,
        expected_output_token: Some("WETH".to_string()),
        expected_output_amount: Some("10000000000000000".to_string()),
        min_output_amount: Some("9000000000000000".to_string()),
        expected_recovery_eth: DecimalAmount::new(1, 2),
        gas_used: Some(150_000),
        would_revert: false,
        metadata: json!({ "calibration_simulation": "fixed_success" }),
    })
}

fn gas_rank() -> FixedGasRankProvider {
    FixedGasRankProvider::new(GasRankPlan {
        predicted_base_fee_gwei: DecimalAmount::from(10),
        candidates: vec![
            RankedFeeCandidate {
                label: "p50".to_string(),
                priority_fee_gwei: DecimalAmount::from(2),
                max_fee_per_gas_gwei: DecimalAmount::from(12),
                rank_position_p50: Some(25),
                gas_before_p50: Some(900_000),
                likely_fits_at_p50: Some(true),
                source: Some("calibration_fixed_gas_rank".to_string()),
                metadata: None,
            },
            RankedFeeCandidate {
                label: "p90".to_string(),
                priority_fee_gwei: DecimalAmount::from(30),
                max_fee_per_gas_gwei: DecimalAmount::from(40),
                rank_position_p50: Some(10),
                gas_before_p50: Some(450_000),
                likely_fits_at_p50: Some(true),
                source: Some("calibration_fixed_gas_rank".to_string()),
                metadata: None,
            },
        ],
    })
}

fn token() -> Address {
    Address::with_last_byte(0x11)
}

fn pool_contract() -> Address {
    Address::with_last_byte(0x22)
}

fn pool_address() -> PoolAddress {
    PoolAddress::new(token(), pool_contract().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builds_trading_vault_planner_request() {
        let from = Address::with_last_byte(0x33);
        let request = build_planner_calibration_request(PlannerCalibrationFixtureConfig::new(from))
            .await
            .unwrap();

        assert_eq!(request.from, from.to_string());
        assert_eq!(request.to, DEFAULT_VAULT_ADDRESS.to_string());
        assert!(request.data.starts_with("0x5f413d10"));
        assert_eq!(
            request.metadata["strategy_name"],
            json!(DEFAULT_STRATEGY_NAME)
        );
        assert_eq!(
            request.metadata["source"]["planner_source"]["calibration"],
            json!(true)
        );
    }

    #[tokio::test]
    async fn builds_direct_uniswap_v2_planner_request() {
        let from = Address::with_last_byte(0x33);
        let mut config = PlannerCalibrationFixtureConfig::new(from);
        config.route = PlannerCalibrationRoute::DirectUniswapV2;

        let request = build_planner_calibration_request(config).await.unwrap();

        assert_eq!(
            request.to.to_ascii_lowercase(),
            "0x7a250d5630b4cf539739df2c5dacb4c659f2488d"
        );
        assert!(request.data.starts_with("0x791ac947"));
    }
}
