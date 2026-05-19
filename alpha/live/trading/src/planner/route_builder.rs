use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use eth_alpha_core::{
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderIntent, OrderSide},
};
use serde::{Deserialize, Serialize};
use tx_simulator::tx_builders::{build_sell_swap_with_min_out, AmmSwapRoute};

use crate::PreparedSellRoute;

use super::{LivePrioritySellPlannerError, LivePrioritySellPlannerInput};

const DEFAULT_UNISWAP_V2_SELL_GAS_LIMIT: u64 = 500_000;
const DEFAULT_UNISWAP_V2_ESTIMATED_SELL_GAS_USED: u64 = 180_000;
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
            input.context.tx.from.parse().map_err(|error| {
                LivePrioritySellPlannerError::InvalidInput(format!(
                    "invalid signer address {:?}: {error}",
                    input.context.tx.from
                ))
            })?,
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
