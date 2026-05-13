mod balance_setup;
mod common;
mod denom_output;
mod router_protocols;
mod uniswap_v3_universal_router;
mod uniswap_v4_universal_router;

use alloy_primitives::{Address, U256};
use eyre::Result;
use std::sync::Arc;
use tx_simulator::TxSimulator;

use super::types::{PoolBuySellParameters, PoolType};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;
use router_protocols::simulate_router_protocol_sell;
use uniswap_v3_universal_router::simulate_universal_router_v3_sell;
use uniswap_v4_universal_router::simulate_universal_router_v4_sell;

#[derive(Debug, Clone)]
pub struct SellSwapResult {
    pub success: bool,
    pub seller_address: Address,
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub tokens_sold: U256,
    pub denom_received: U256,
    pub sell_transaction: ProcessedTransaction,
    pub block_number: u64,
    pub failure_reason: Option<String>,
}

pub async fn simulate_sell_swap(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    token_address: Address,
    pool_address: Address,
    pool_type: PoolType,
    tokens_to_sell: U256,
    block_number: Option<u64>,
) -> Result<SellSwapResult> {
    let mut config = PoolBuySellParameters::new(token_address, pool_address, pool_type);
    config.block_number = block_number;
    simulate_sell_swap_with_params(simulator, tx_processor, config, tokens_to_sell).await
}

pub async fn simulate_sell_swap_with_params(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    mut config: PoolBuySellParameters,
    tokens_to_sell: U256,
) -> Result<SellSwapResult> {
    let block = match config.block_number {
        Some(b) => b,
        None => simulator.latest_historical_context_block_number()?,
    };
    config.block_number = Some(block);

    match config.pool_type {
        PoolType::UniswapV4 => {
            simulate_universal_router_v4_sell(
                simulator,
                tx_processor,
                config,
                tokens_to_sell,
                block,
            )
            .await
        }
        PoolType::UniswapV3 { fee_tier } => {
            simulate_universal_router_v3_sell(
                simulator,
                tx_processor,
                config,
                tokens_to_sell,
                fee_tier,
                block,
            )
            .await
        }
        pool_type
            if pool_type.known_v2_protocol().is_some()
                || pool_type.known_v3_protocol().is_some() =>
        {
            simulate_router_protocol_sell(simulator, tx_processor, config, tokens_to_sell, block)
                .await
        }
        pool_type => Err(eyre::eyre!(
            "Pool type {:?} not yet supported for sell-only simulation",
            pool_type
        )),
    }
}
