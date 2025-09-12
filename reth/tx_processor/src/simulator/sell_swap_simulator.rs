use std::sync::Arc;
use eyre::Result;
use alloy_primitives::{Address, U256};
use tx_simulator::TxSimulator;

use crate::tx_processor::TxProcessor;
use crate::tx_processor::data_models::ProcessedTransaction;
use super::types::PoolType;
use reth_chain_query::tx_builders::{self, amm_swap_route::AmmSwapRoute};

#[derive(Debug, Clone)]
pub struct SellSwapResult {
    pub success: bool,
    pub seller_address: Address,
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub tokens_sold: U256,
    pub eth_received: U256,
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
    // Defaults consistent with buy simulator
    let slippage_tolerance = 0.5_f64;
    let default_cfg = super::config::PoolViabilityConfig::default();
    let seller_address = default_cfg.buyer_address;

    let block = match block_number { Some(b) => b, None => simulator.get_latest_block()? };

    // Build route
    let route = match pool_type {
        PoolType::UniswapV2 => AmmSwapRoute::UniswapV2 { pool: pool_address },
        PoolType::SushiSwap => AmmSwapRoute::SushiswapV2 { pool: pool_address },
        PoolType::UniswapV3 { fee_tier } => AmmSwapRoute::UniswapV3 { pool: pool_address, fee_tier },
        _ => return Err(eyre::eyre!("Pool type {:?} not yet supported for sell-only simulation", pool_type)),
    };

    // Build SELL transaction
    let slippage_bps = (slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let sell_tx = tx_builders::build_sell_swap(
        &route,
        seller_address,
        token_address,
        tokens_to_sell,
        slippage_bps,
        deadline,
    );

    // Use ProcessedTxProvider via simulator's provider factory
    let provider_factory = simulator.provider_factory().clone();
    let processed_tx_provider = crate::processed_tx_provider::ProcessedTxProvider::with_provider_factory(provider_factory)?;
    let processed = processed_tx_provider
        .process_transaction_from_unsigned_tx(sell_tx.clone(), Some(block))
        .await?;

    // Extract ETH received by seller
    let eth_received = extract_eth_received(&processed, seller_address);
    let success = processed.status == "1";

    Ok(SellSwapResult {
        success,
        seller_address,
        token_address,
        pool_address,
        pool_type,
        tokens_sold: tokens_to_sell,
        eth_received,
        sell_transaction: processed,
        block_number: block,
        failure_reason: if success { None } else { Some("Sell transaction failed".to_string()) },
    })
}

fn extract_eth_received(processed_tx: &ProcessedTransaction, recipient_address: Address) -> U256 {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if let Some(&amount) = balance_changes.currency_net.get("ETH") {
            if amount > U256::ZERO { return amount; }
        }
    }
    U256::ZERO
}

