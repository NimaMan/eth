/// Core trading viability analyzer

use std::sync::Arc;
use eyre::Result;
use tx_simulator::{TxSimulator, CallRequest};
use crate::TxProcessor;
use crate::data_models::ProcessedTransaction;

use crate::erc20_token_trading_viability::{
    config::PoolViabilityConfig,
    types::{PoolViabilityResult, PoolType},
    pool_adapters::{PoolAdapter, UniswapV2Adapter, UniswapV3Adapter},
    optional_setup_buy_approve_sell_token_simulator::{OptionalSetupBuyApproveSellTokenSimulator, OptionalSetupBuyApproveSellResult},
};

/// Analyze if a token pool is viable for trading
/// 
/// This function simulates the complete trading lifecycle sequentially:
/// 1. Optional setup transaction (e.g., enable trading)
/// 2. Buy tokens from pool using ETH
/// 3. Approve token spending to router
/// 4. Sell tokens back to pool for ETH
/// 
/// The simulator maintains blockchain state changes between each transaction,
/// so the buy transaction's state changes are visible to the approve transaction,
/// and both are visible to the sell transaction. This allows accurate tax calculation
/// and detection of trading restrictions.
/// 
/// Returns comprehensive analysis including tax percentages and trading viability
pub async fn analyze_pool_viability(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolViabilityConfig,
) -> Result<PoolViabilityResult> {
    // Create the specialized trading simulator using the new architecture
    let trading_simulator = OptionalSetupBuyApproveSellTokenSimulator::new(
        simulator.clone(),
        tx_processor.clone(),
    );
    
    // Convert prior transaction to CallRequest if provided
    let setup_transaction = if let Some(prior_tx) = &config.prior_tx {
        Some(build_call_from_processed_tx(
            prior_tx, 
            config.gas_limit, 
            config.gas_price
        )?)
    } else {
        None
    };
    
    // Create pool adapter based on type
    let pool_adapter: Box<dyn PoolAdapter> = match config.pool_type {
        PoolType::UniswapV2 | PoolType::SushiSwap => {
            Box::new(UniswapV2Adapter::new(config.pool_address))
        },
        PoolType::UniswapV3 { fee_tier } => {
            Box::new(UniswapV3Adapter::new(config.pool_address, fee_tier))
        },
        _ => {
            return Err(eyre::eyre!("Pool type {:?} not yet implemented", config.pool_type));
        }
    };
    
    // Execute the complete trading sequence sequentially with state preservation
    let trading_result = trading_simulator.simulate_optional_setup_then_buy_approve_sell_token_sequence(
        setup_transaction,
        config.test_amount,
        config.token_address,
        config.pool_address,
        config.buyer_address,
        pool_adapter,
        config.slippage_tolerance,
        config.block_number,
    ).await?;
    
    // Convert the trading result to the expected PoolViabilityResult format
    convert_trading_result_to_pool_viability_result(trading_result, config)
}

/// Convert OptionalSetupBuyApproveSellResult to PoolViabilityResult
/// 
/// This function maps the comprehensive trading result from the new simulator
/// to the expected PoolViabilityResult format used by the rest of the system.
fn convert_trading_result_to_pool_viability_result(
    trading_result: OptionalSetupBuyApproveSellResult,
    config: PoolViabilityConfig,
) -> Result<PoolViabilityResult> {
    Ok(PoolViabilityResult {
        pool_type: config.pool_type,
        pool_address: config.pool_address,
        token_address: config.token_address,
        is_tradeable: trading_result.token_is_tradeable,
        buy_tax_percent: trading_result.buy_tax_percentage,
        sell_tax_percent: trading_result.sell_tax_percentage,
        tokens_received: trading_result.tokens_bought_amount,
        eth_spent: trading_result.eth_spent_on_tokens,
        eth_received: trading_result.eth_received_from_selling_tokens,
        buy_transaction: trading_result.token_buy_result,
        sell_transaction: trading_result.token_sell_result,
        approve_transaction: trading_result.token_approve_result,
        prior_transaction: trading_result.setup_tx_result,
        failure_reason: trading_result.failure_reason,
        block_number: trading_result.simulation_block_number,
    })
}

/// Build CallRequest from ProcessedTransaction
/// 
/// Helper function to convert a ProcessedTransaction (from prior setup tx)
/// to a CallRequest that can be executed by the simulator.
fn build_call_from_processed_tx(
    tx: &ProcessedTransaction,
    gas_limit: u64,
    gas_price: u128,
) -> Result<CallRequest> {
    use alloy_primitives::Bytes;
    
    Ok(CallRequest {
        from: Some(tx.from_address),
        to: tx.to_address,
        value: Some(tx.value),
        data: Some(Bytes::from(tx.input.clone())),
        gas: Some(gas_limit),
        gas_price: Some(gas_price),
        max_fee_per_gas: tx.fees.max_fee_per_gas.map(|v| v.try_into().unwrap_or(0)),
        max_priority_fee_per_gas: tx.fees.max_priority_fee.map(|v| v.try_into().unwrap_or(0)),
        nonce: None,
    })
}