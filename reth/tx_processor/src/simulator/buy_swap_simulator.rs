use std::sync::Arc;
use eyre::Result;
use alloy_primitives::{Address, U256};
use tx_simulator::{TxSimulator, UnsignedTransaction};

use crate::tx_processor::TxProcessor;
use crate::tx_processor::data_models::ProcessedTransaction;

use super::types::PoolType;
use reth_chain_query::tx_builders::{self, amm_swap_route::AmmSwapRoute};

/// Result of a single buy swap simulation
#[derive(Debug, Clone)]
pub struct BuySwapResult {
    pub success: bool,
    pub buyer_address: Address,
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub eth_spent: U256,
    pub tokens_received: U256,
    pub buy_transaction: ProcessedTransaction,
    pub block_number: u64,
    pub failure_reason: Option<String>,
}

/// Simulate a single buy swap with a default amount of 1 ETH.
/// - Reuses the same default buyer address as the buy/approve/sell simulator.
/// - Supports Uniswap V2/V3 adapters (more can be added later).
pub async fn simulate_buy_swap(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    token_address: Address,
    pool_address: Address,
    pool_type: PoolType,
    block_number: Option<u64>,
) -> Result<BuySwapResult> {
    // Default constants
    let eth_amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
    let slippage_tolerance = 0.5_f64;

    // Reuse buyer address from existing config default to stay consistent
    let default_cfg = super::config::PoolViabilityConfig::default();
    let buyer_address = default_cfg.buyer_address;

    // Resolve block
    let block = match block_number {
        Some(b) => b,
        None => simulator.get_latest_block()?,
    };

    // Build route
    let route = match pool_type {
        PoolType::UniswapV2 => AmmSwapRoute::UniswapV2 { pool: pool_address },
        PoolType::SushiSwap => AmmSwapRoute::SushiswapV2 { pool: pool_address },
        PoolType::UniswapV3 { fee_tier } => AmmSwapRoute::UniswapV3 { pool: pool_address, fee_tier },
        _ => return Err(eyre::eyre!("Pool type {:?} not yet supported for buy-only simulation", pool_type)),
    };

    // Build BUY transaction
    let slippage_bps = (slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let buy_tx: UnsignedTransaction = tx_builders::build_buy_swap(
        &route,
        buyer_address,
        token_address,
        eth_amount,
        slippage_bps,
        deadline,
    );
    
    // Use the provider that can simulate and return a ProcessedTransaction directly
    let provider_factory = simulator.provider_factory().clone();
    let processed_tx_provider = crate::processed_tx_provider::ProcessedTxProvider::with_provider_factory(provider_factory)?;
    let processed = processed_tx_provider
        .process_transaction_from_unsigned_tx(buy_tx.clone(), Some(block))
        .await?;

    // Extract tokens received by buyer
    let tokens_received = extract_tokens_received(&processed, buyer_address, token_address);

    let success = processed.status == "1";
    Ok(BuySwapResult {
        success,
        buyer_address,
        token_address,
        pool_address,
        pool_type,
        eth_spent: eth_amount,
        tokens_received,
        buy_transaction: processed,
        block_number: block,
        failure_reason: if success {
            None
        } else {
            Some("Buy transaction failed".to_string())
        },
    })
}

fn extract_tokens_received(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    token_address: Address,
) -> U256 {
    use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
use reth_chain_query::to_checksum_address;

    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if let Some(symbol) = get_token_symbol(&token_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                if amount > U256::ZERO {
                    return amount;
                }
            }
        } else {
            let token_key = to_checksum_address(&token_address);
            if let Some(&amount) = balance_changes.token_net.get(&token_key) {
                if amount > U256::ZERO {
                    return amount;
                }
            }
        }
    }
    U256::ZERO
}
