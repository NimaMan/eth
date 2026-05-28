use std::sync::Arc;

use alloy_primitives::{address, Address, Bytes, U256};
use eth_alpha_core::{
    error::Result,
    ids::PoolAddress,
    market::{PoolProtocol, PoolSnapshot},
};
use tx_processor::{PoolBuySellParameters, PoolType, UniswapV4PoolConfig as TxUniswapV4PoolConfig};
use tx_simulator::TxSimulator;

const ERC20_DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const USDT_ADDRESS: Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
const DAI_ADDRESS: Address = address!("6B175474E89094C44Da98b954EedeAC495271d0F");

pub(super) async fn pool_simulation_parameters(
    simulator: &Arc<TxSimulator>,
    pool: &PoolSnapshot,
    token_address: Address,
    amount: U256,
    block: u64,
) -> std::result::Result<PoolBuySellParameters, String> {
    let pool_contract_address = parse_pool_address(&pool.address)
        .map_err(|error| format!("invalid pool address for chain simulation: {error}"))?;
    let pool_type = pool_type_for_pool(pool).ok_or_else(|| {
        format!(
            "protocol {:?} not supported by chain simulator",
            pool.protocol
        )
    })?;
    let token_decimals = match pool.token_decimals {
        Some(decimals) => decimals,
        None => query_erc20_decimals(simulator, token_address, block).await?,
    };
    let denom_address = pool
        .denom_address
        .ok_or_else(|| format!("pool {} has no denomination address", pool.address))?;
    let denom_decimals = denom_decimals(simulator, denom_address, block).await?;

    let mut params = PoolBuySellParameters::new(token_address, pool_contract_address, pool_type)
        .with_test_amount(amount)
        .with_block(block)
        .with_denom_address(denom_address)
        .with_denom_decimals(denom_decimals)
        .with_token_decimals(token_decimals);

    if let Some(v4) = &pool.uniswap_v4 {
        params = params.with_uniswap_v4_config(TxUniswapV4PoolConfig {
            pool_manager: v4.pool_manager,
            pool_id: v4.pool_id,
            currency0: v4.currency0,
            currency1: v4.currency1,
            fee: v4.fee,
            tick_spacing: v4.tick_spacing,
            hooks: v4.hooks,
            hook_data: Vec::new(),
        });
    }

    Ok(params)
}

fn pool_type_for_pool(pool: &PoolSnapshot) -> Option<PoolType> {
    match &pool.protocol {
        PoolProtocol::UniswapV2 => Some(PoolType::UniswapV2),
        PoolProtocol::UniswapV3 => Some(PoolType::UniswapV3 {
            fee_tier: pool.fee_tier.unwrap_or(3000),
        }),
        PoolProtocol::UniswapV4 => Some(PoolType::UniswapV4),
        PoolProtocol::PancakeSwapV2 => Some(PoolType::PancakeSwapV2),
        PoolProtocol::Unknown(s) => match s.as_str() {
            "sushi" | "sushiswap" => Some(PoolType::SushiSwap),
            _ => None,
        },
    }
}

async fn denom_decimals(
    simulator: &Arc<TxSimulator>,
    denom_address: Address,
    block: u64,
) -> std::result::Result<u8, String> {
    if denom_address.is_zero() || denom_address == WETH_ADDRESS || denom_address == DAI_ADDRESS {
        return Ok(18);
    }
    if denom_address == USDC_ADDRESS || denom_address == USDT_ADDRESS {
        return Ok(6);
    }
    query_erc20_decimals(simulator, denom_address, block).await
}

async fn query_erc20_decimals(
    simulator: &Arc<TxSimulator>,
    token_address: Address,
    block: u64,
) -> std::result::Result<u8, String> {
    let result = simulator
        .simulate_view_function(
            token_address,
            Bytes::from_static(&ERC20_DECIMALS_SELECTOR),
            Some(block),
        )
        .await
        .map_err(|error| format!("token decimals simulation failed: {error}"))?;
    if !result.success {
        return Err("token decimals simulation reverted".to_string());
    }
    if result.output.len() < 32 {
        return Err(format!(
            "token decimals simulation returned short output: {} bytes",
            result.output.len()
        ));
    }
    Ok(result.decode_uint8())
}

fn parse_pool_address(pool_id: &PoolAddress) -> Result<Address> {
    let s = pool_id.as_str();
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 {
        return Err(eth_alpha_core::error::AlphaCoreError::InvalidPoolAddress(
            s.to_string(),
        ));
    }
    let pool_part = parts[1];
    let addr_str = pool_part.split('#').next().unwrap_or(pool_part);
    addr_str
        .parse::<Address>()
        .map_err(|_| eth_alpha_core::error::AlphaCoreError::InvalidPoolAddress(s.to_string()))
}
