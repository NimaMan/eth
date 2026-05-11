use alloy_primitives::{Address, I256, U256};
use eyre::Result;
use std::sync::Arc;
use tx_simulator::{TxSimulator, UnsignedTransaction};

use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;

use super::pool_buy_sell_simulator::PoolBuySellSimulator;
use super::types::{PoolBuySellParameters, PoolType};
use tx_simulator::tx_builders::{self, amm_swap_route::AmmSwapRoute};

/// Result of a single buy swap simulation
#[derive(Debug, Clone)]
pub struct BuySwapResult {
    pub success: bool,
    pub buyer_address: Address,
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub denom_spent: U256,
    pub tokens_received: U256,
    pub buy_transaction: ProcessedTransaction,
    pub block_number: u64,
    pub failure_reason: Option<String>,
}

/// Simulate a single buy swap with a configurable ETH amount.
/// - Reuses the same default buyer address as the buy/approve/sell simulator.
/// - Supports Uniswap V2/V3 adapters (more can be added later).
pub async fn simulate_buy_swap(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    token_address: Address,
    pool_address: Address,
    pool_type: PoolType,
    block_number: Option<u64>,
    eth_amount: U256,
) -> Result<BuySwapResult> {
    let mut config = PoolBuySellParameters::new(token_address, pool_address, pool_type)
        .with_test_amount(eth_amount);
    config.block_number = block_number;
    simulate_buy_swap_with_params(simulator, tx_processor, config).await
}

pub async fn simulate_buy_swap_with_params(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    mut config: PoolBuySellParameters,
) -> Result<BuySwapResult> {
    let block = match config.block_number {
        Some(b) => b,
        None => simulator.latest_historical_context_block_number()?,
    };
    config.block_number = Some(block);

    if matches!(config.pool_type, PoolType::UniswapV4) {
        return simulate_v4_buy_swap(simulator, tx_processor, config, block).await;
    }

    let buyer_address = config.buyer_address;
    // Build route
    let route = if let Some(protocol) = config.pool_type.known_v2_protocol() {
        AmmSwapRoute::V2Router {
            pool: config.pool_address,
            router: protocol.router(),
        }
    } else {
        match config.pool_type {
            PoolType::UniswapV3 { fee_tier } => AmmSwapRoute::UniswapV3 {
                pool: config.pool_address,
                fee_tier,
            },
            _ => {
                return Err(eyre::eyre!(
                    "Pool type {:?} not yet supported for buy-only simulation",
                    config.pool_type
                ));
            }
        }
    };

    // Build BUY transaction
    let slippage_bps = (config.slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let buy_tx: UnsignedTransaction = tx_builders::build_buy_swap(
        &route,
        buyer_address,
        config.token_address,
        config.test_amount,
        slippage_bps,
        deadline,
    );

    // Use the provider that can simulate and return a ProcessedTransaction directly
    let provider_factory = simulator.provider_factory().clone();
    let processed_tx_provider =
        crate::processed_tx_provider::ProcessedTxProvider::with_provider_factory(provider_factory)?;
    let processed = processed_tx_provider
        .process_transaction_from_unsigned_tx(buy_tx.clone(), Some(block))
        .await?;

    // Extract tokens received by buyer
    let tokens_received = extract_tokens_received(&processed, buyer_address, config.token_address);

    let success = processed.status;
    Ok(BuySwapResult {
        success,
        buyer_address,
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
        denom_spent: config.test_amount,
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

async fn simulate_v4_buy_swap(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
    block: u64,
) -> Result<BuySwapResult> {
    let pool_simulator = PoolBuySellSimulator::new(simulator, tx_processor);
    let result = pool_simulator.check_pool(config.clone()).await?;
    let tokens_received = extract_tokens_received(
        &result.buy_transaction,
        config.buyer_address,
        config.token_address,
    );
    let success = result.buy_transaction.status && !tokens_received.is_zero();

    Ok(BuySwapResult {
        success,
        buyer_address: config.buyer_address,
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
        denom_spent: if result.denom_spent.is_zero() {
            config.test_amount
        } else {
            result.denom_spent
        },
        tokens_received,
        buy_transaction: result.buy_transaction,
        block_number: result.block_number.max(block),
        failure_reason: if success {
            None
        } else {
            result
                .failure_reason
                .or_else(|| Some("Universal Router V4 buy transaction failed".to_string()))
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
                if amount > I256::ZERO {
                    return amount.unsigned_abs();
                }
            }
        } else {
            let token_key = to_checksum_address(&token_address);
            if let Some(&amount) = balance_changes.token_net.get(&token_key) {
                if amount > I256::ZERO {
                    return amount.unsigned_abs();
                }
            }
        }
    }
    U256::ZERO
}
