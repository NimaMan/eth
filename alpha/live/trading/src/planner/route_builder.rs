use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use eth_alpha_core::{
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderIntent, OrderSide},
};
use serde::{Deserialize, Serialize};
use tx_simulator::tx_builders::{
    AmmSwapRoute, build_sell_swap_with_min_out,
    build_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth,
};

use crate::PreparedSellRoute;

use super::{LivePrioritySellPlannerError, LivePrioritySellPlannerInput};

const DEFAULT_UNISWAP_V2_SELL_GAS_LIMIT: u64 = 500_000;
const DEFAULT_UNISWAP_V2_ESTIMATED_SELL_GAS_USED: u64 = 180_000;
const DEFAULT_UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT: u64 = 300_000;
const DEFAULT_UNISWAP_V2_TRADING_VAULT_ESTIMATED_SELL_GAS_USED: u64 = 130_000;
const WETH_ADDRESS: Address =
    alloy_primitives::address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteBuildRequest {
    pub gas_limit: u64,
    pub estimated_gas_used: u64,
}

impl Default for RouteBuildRequest {
    fn default() -> Self {
        Self {
            gas_limit: DEFAULT_UNISWAP_V2_SELL_GAS_LIMIT,
            estimated_gas_used: DEFAULT_UNISWAP_V2_ESTIMATED_SELL_GAS_USED,
        }
    }
}

#[async_trait]
pub trait SellRouteBuilder: Send + Sync {
    async fn build_route(
        &self,
        input: &LivePrioritySellPlannerInput,
    ) -> Result<PreparedSellRoute, LivePrioritySellPlannerError>;
}

#[derive(Clone, Debug, Default)]
pub struct UniswapV2SellRouteBuilder {
    request: RouteBuildRequest,
}

impl UniswapV2SellRouteBuilder {
    pub fn new(request: RouteBuildRequest) -> Self {
        Self { request }
    }
}

#[async_trait]
impl SellRouteBuilder for UniswapV2SellRouteBuilder {
    async fn build_route(
        &self,
        input: &LivePrioritySellPlannerInput,
    ) -> Result<PreparedSellRoute, LivePrioritySellPlannerError> {
        validate_sell_intent(&input.intent, &input.pool)?;
        ensure_weth_denom(&input.pool)?;
        let pool_contract = parse_pool_contract(&input.pool)?;
        let min_output = parse_min_output(input.min_output_amount.as_deref())?;
        let unsigned = build_sell_swap_with_min_out(
            &AmmSwapRoute::UniswapV2 {
                pool: pool_contract,
            },
            parse_owner_address(input)?,
            input.intent.token_address,
            input.intent.amount.raw,
            min_output,
            input.context.deadline_unix_secs,
        );
        let to = unsigned.to.ok_or_else(|| {
            LivePrioritySellPlannerError::Route("tx builder returned no router address".to_string())
        })?;
        let data = unsigned.data.ok_or_else(|| {
            LivePrioritySellPlannerError::Route("tx builder returned no calldata".to_string())
        })?;
        let gas_limit = unsigned.gas.unwrap_or(self.request.gas_limit);

        Ok(PreparedSellRoute {
            protocol: input.pool.protocol.label().to_string(),
            router_address: to.to_string(),
            calldata: format!("0x{}", hex::encode(data.as_ref())),
            value_wei: unsigned.value.unwrap_or(U256::ZERO).to_string(),
            gas_limit,
            estimated_gas_used: self.request.estimated_gas_used.min(gas_limit),
            max_slippage_bps: Some(input.intent.max_slippage_bps),
        })
    }
}

#[derive(Clone, Debug)]
pub struct UniswapV2TradingVaultSellRouteBuilder {
    vault_address: Address,
    request: RouteBuildRequest,
}

impl UniswapV2TradingVaultSellRouteBuilder {
    pub fn new(vault_address: Address, request: RouteBuildRequest) -> Self {
        Self {
            vault_address,
            request,
        }
    }

    pub fn with_default_gas(vault_address: Address) -> Self {
        Self {
            vault_address,
            request: RouteBuildRequest {
                gas_limit: DEFAULT_UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
                estimated_gas_used: DEFAULT_UNISWAP_V2_TRADING_VAULT_ESTIMATED_SELL_GAS_USED,
            },
        }
    }
}

#[async_trait]
impl SellRouteBuilder for UniswapV2TradingVaultSellRouteBuilder {
    async fn build_route(
        &self,
        input: &LivePrioritySellPlannerInput,
    ) -> Result<PreparedSellRoute, LivePrioritySellPlannerError> {
        validate_sell_intent(&input.intent, &input.pool)?;
        ensure_weth_denom(&input.pool)?;
        let min_output = parse_min_output(input.min_output_amount.as_deref())?;
        let unsigned = build_uniswap_v2_trading_vault_emergency_sell_v2_exact_tokens_for_eth(
            self.vault_address,
            parse_owner_address(input)?,
            input.intent.token_address,
            input.intent.amount.raw,
            min_output,
            input.context.deadline_unix_secs,
        );
        let to = unsigned.to.ok_or_else(|| {
            LivePrioritySellPlannerError::Route("tx builder returned no vault address".to_string())
        })?;
        let data = unsigned.data.ok_or_else(|| {
            LivePrioritySellPlannerError::Route("tx builder returned no calldata".to_string())
        })?;
        let gas_limit = self.request.gas_limit.max(unsigned.gas.unwrap_or(0));

        Ok(PreparedSellRoute {
            protocol: "uniswap_v2_trading_vault".to_string(),
            router_address: to.to_string(),
            calldata: format!("0x{}", hex::encode(data.as_ref())),
            value_wei: unsigned.value.unwrap_or(U256::ZERO).to_string(),
            gas_limit,
            estimated_gas_used: self.request.estimated_gas_used.min(gas_limit),
            max_slippage_bps: Some(input.intent.max_slippage_bps),
        })
    }
}

fn validate_sell_intent(
    intent: &OrderIntent,
    pool: &PoolSnapshot,
) -> Result<(), LivePrioritySellPlannerError> {
    if intent.side != OrderSide::Sell {
        return Err(LivePrioritySellPlannerError::UnsupportedIntent(
            "v1 live planner only supports sell intents".to_string(),
        ));
    }
    if intent.protocol != PoolProtocol::UniswapV2 || pool.protocol != PoolProtocol::UniswapV2 {
        return Err(LivePrioritySellPlannerError::UnsupportedIntent(format!(
            "v1 live planner only supports Uniswap V2 sells; intent={:?} pool={:?}",
            intent.protocol, pool.protocol
        )));
    }
    if intent.token_address != pool.token_address {
        return Err(LivePrioritySellPlannerError::InvalidInput(format!(
            "intent token {} does not match pool token {}",
            intent.token_address, pool.token_address
        )));
    }
    if intent.pool_address != pool.address {
        return Err(LivePrioritySellPlannerError::InvalidInput(format!(
            "intent pool {} does not match snapshot pool {}",
            intent.pool_address, pool.address
        )));
    }
    if intent.amount.raw.is_zero() {
        return Err(LivePrioritySellPlannerError::InvalidInput(
            "sell amount is zero".to_string(),
        ));
    }
    if !pool.can_sell {
        return Err(LivePrioritySellPlannerError::InvalidInput(
            "pool snapshot says can_sell=false".to_string(),
        ));
    }
    Ok(())
}

fn ensure_weth_denom(pool: &PoolSnapshot) -> Result<(), LivePrioritySellPlannerError> {
    if pool
        .denom_address
        .map(|denom| denom == WETH_ADDRESS)
        .unwrap_or(false)
        || pool
            .denom_symbol
            .as_deref()
            .map(|symbol| matches!(symbol.to_ascii_uppercase().as_str(), "ETH" | "WETH"))
            .unwrap_or(false)
    {
        return Ok(());
    }

    Err(LivePrioritySellPlannerError::UnsupportedIntent(
        "v1 Uniswap V2 live sell route only supports ETH/WETH-denominated pools".to_string(),
    ))
}

fn parse_owner_address(
    input: &LivePrioritySellPlannerInput,
) -> Result<Address, LivePrioritySellPlannerError> {
    input.context.tx.from.parse().map_err(|error| {
        LivePrioritySellPlannerError::InvalidInput(format!(
            "invalid signer address {:?}: {error}",
            input.context.tx.from
        ))
    })
}

fn parse_pool_contract(pool: &PoolSnapshot) -> Result<Address, LivePrioritySellPlannerError> {
    let value = pool.address.as_str().rsplit(':').next().ok_or_else(|| {
        LivePrioritySellPlannerError::InvalidInput(format!(
            "pool address {} has no pool contract segment",
            pool.address
        ))
    })?;
    value.parse::<Address>().map_err(|error| {
        LivePrioritySellPlannerError::InvalidInput(format!(
            "invalid pool contract address {value:?}: {error}"
        ))
    })
}

fn parse_min_output(value: Option<&str>) -> Result<U256, LivePrioritySellPlannerError> {
    let Some(value) = value else {
        return Ok(U256::ZERO);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(U256::ZERO);
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).map_err(|error| {
            LivePrioritySellPlannerError::InvalidInput(format!(
                "invalid min_output_amount hex quantity {trimmed:?}: {error}"
            ))
        })
    } else {
        U256::from_str_radix(trimmed, 10).map_err(|error| {
            LivePrioritySellPlannerError::InvalidInput(format!(
                "invalid min_output_amount decimal quantity {trimmed:?}: {error}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};
    use eth_alpha_core::{
        amount::{Amount, DecimalAmount},
        ids::{PoolAddress, PortfolioId, PositionId, StrategyName, TradeId, WalletId},
        market::{PoolProtocol, PoolSnapshot},
        order::{OrderIntent, OrderSide},
        position::{Position, PositionKey, PositionState},
    };
    use serde_json::json;

    use super::*;
    use crate::TxPrepRequestContext;

    fn token() -> Address {
        Address::with_last_byte(0x11)
    }

    fn pool_contract() -> Address {
        Address::with_last_byte(0x22)
    }

    fn pool_address() -> PoolAddress {
        PoolAddress::new(token(), pool_contract().to_string())
    }

    fn strategy_name() -> StrategyName {
        StrategyName("alpha11-live-univ2-lp30-riskexit-hold20".to_string())
    }

    fn input() -> LivePrioritySellPlannerInput {
        let pool_address = pool_address();
        let strategy_name = strategy_name();
        let mut position = Position::with_trade_id(
            PositionId("position".to_string()),
            TradeId("trade".to_string()),
            PositionKey {
                portfolio_id: PortfolioId("portfolio".to_string()),
                wallet_id: WalletId("wallet".to_string()),
                strategy_name: strategy_name.clone(),
                token_address: token(),
                pool_address: pool_address.clone(),
                protocol: PoolProtocol::UniswapV2,
            },
        );
        position.state = PositionState::BuyConfirmed;

        LivePrioritySellPlannerInput {
            context: super::super::PlannerTxContext {
                tx: TxPrepRequestContext {
                    chain_id: 1,
                    from: Address::with_last_byte(0x33).to_string(),
                    strategy_name: strategy_name.0.clone(),
                    strategy_run_id: Some("run-1".to_string()),
                    observed_block: Some(25_128_246),
                    source_metadata: json!({ "signal_id": 222 }),
                },
                current_block: 25_128_246,
                deadline_unix_secs: 1_800_000_000,
            },
            intent: OrderIntent {
                trade_id: Some(TradeId("trade".to_string())),
                portfolio_id: PortfolioId("portfolio".to_string()),
                wallet_id: WalletId("wallet".to_string()),
                strategy_name,
                side: OrderSide::Sell,
                token_address: token(),
                pool_address: pool_address.clone(),
                protocol: PoolProtocol::UniswapV2,
                amount: Amount {
                    raw: U256::from(1_000_000u64),
                    decimals: 18,
                },
                route: None,
                max_slippage_bps: 500,
                deadline_secs: 60,
                decision_reason: None,
            },
            position,
            pool: PoolSnapshot {
                address: pool_address,
                token_address: token(),
                protocol: PoolProtocol::UniswapV2,
                denom_address: Some(WETH_ADDRESS),
                denom_symbol: Some("WETH".to_string()),
                denom_reserve: DecimalAmount::from(10),
                token_reserve: DecimalAmount::from(100),
                price_denom_per_token: Some(DecimalAmount::new(1, 1)),
                initial_price_denom_per_token: Some(DecimalAmount::new(1, 1)),
                price_ratio_to_initial: Some(DecimalAmount::from(1)),
                token_decimals: Some(18),
                fee_tier: None,
                uniswap_v4: None,
                latest_block: 25_128_246,
                can_buy: true,
                can_sell: true,
                is_scam: false,
            },
            min_output_amount: Some("9000000000000000".to_string()),
            source_metadata: json!({ "planner": "test" }),
        }
    }

    #[tokio::test]
    async fn uniswap_v2_trading_vault_route_targets_vault_and_internal_sell_selector() {
        let vault = Address::with_last_byte(0xaa);
        let route = UniswapV2TradingVaultSellRouteBuilder::with_default_gas(vault)
            .build_route(&input())
            .await
            .unwrap();

        assert_eq!(route.protocol, "uniswap_v2_trading_vault");
        assert!(
            route
                .router_address
                .eq_ignore_ascii_case(&vault.to_string())
        );
        assert!(route.calldata.starts_with("0x5f413d10"));
        assert_eq!(route.value_wei, "0");
        assert_eq!(
            route.gas_limit,
            DEFAULT_UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT
        );
        assert_eq!(
            route.estimated_gas_used,
            DEFAULT_UNISWAP_V2_TRADING_VAULT_ESTIMATED_SELL_GAS_USED
        );
    }
}
