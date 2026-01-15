use std::sync::Arc;

use eyre::{eyre, Result, WrapErr};
use tx_simulator::TxSimulator;

use reth_chain_query::dex::{
    fetch_uniswap_v2_pair_address, fetch_uniswap_v3_pool_address, SUSHISWAP_FACTORY,
    UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY,
};

use crate::simulator::types::{PoolBuySellParameters, PoolType};

pub(super) async fn validate_pool_registration(
    simulator: Arc<TxSimulator>,
    config: &PoolBuySellParameters,
    block_number: u64,
) -> Result<()> {
    match config.pool_type {
        PoolType::UniswapV2 | PoolType::SushiSwap => {
            let denom = config.denom_address;
            if denom.is_zero() {
                return Err(eyre!(
                    "denom_address missing for {:?} pool",
                    config.pool_type
                ));
            }
            let factory = match config.pool_type {
                PoolType::UniswapV2 => UNISWAP_V2_FACTORY,
                PoolType::SushiSwap => SUSHISWAP_FACTORY,
                _ => unreachable!(),
            };
            let resolved = fetch_uniswap_v2_pair_address(
                simulator.as_ref(),
                factory,
                config.token_address,
                denom,
                Some(block_number),
            )
            .await
            .wrap_err_with(|| {
                format!(
                    "{:?} factory getPair check failed at block {}",
                    config.pool_type, block_number
                )
            })?;

            if resolved.is_zero() {
                return Err(eyre!(
                    "No {:?} pool found for token {} with denom {} at block {}",
                    config.pool_type,
                    config.token_address,
                    denom,
                    block_number
                ));
            }

            if resolved != config.pool_address {
                return Err(eyre!(
                    "{:?} factory reports pool {} but configuration provided {} for token {} / denom {} at block {}",
                    config.pool_type,
                    resolved,
                    config.pool_address,
                    config.token_address,
                    denom,
                    block_number
                ));
            }
        }
        PoolType::UniswapV3 { fee_tier } => {
            let denom = config.denom_address;
            if denom.is_zero() {
                return Err(eyre!("denom_address missing for Uniswap V3 pool checks"));
            }

            let resolved = fetch_uniswap_v3_pool_address(
                simulator.as_ref(),
                UNISWAP_V3_FACTORY,
                config.token_address,
                denom,
                fee_tier,
                Some(block_number),
            )
            .await
            .wrap_err_with(|| {
                format!(
                    "Uniswap V3 factory getPool check failed at block {}",
                    block_number
                )
            })?;

            if resolved.is_zero() {
                return Err(eyre!(
                    "No Uniswap V3 pool found for token {} with denom {} at fee tier {} and block {}",
                    config.token_address,
                    denom,
                    fee_tier,
                    block_number
                ));
            }

            if resolved != config.pool_address {
                return Err(eyre!(
                    "Uniswap V3 factory reports pool {} but configuration provided {} for token {} / denom {} at fee tier {} and block {}",
                    resolved,
                    config.pool_address,
                    config.token_address,
                    denom,
                    fee_tier,
                    block_number
                ));
            }
        }
        PoolType::UniswapV4 => {}
        _ => {}
    }

    Ok(())
}
