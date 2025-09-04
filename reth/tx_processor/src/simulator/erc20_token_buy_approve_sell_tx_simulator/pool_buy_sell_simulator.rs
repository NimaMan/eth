/// Core trading viability analyzer - Direct implementation without redundant layers
///
/// This module checks if a token can be bought and sold on a DEX pool by simulating
/// the complete trading sequence: [optional_setup] -> buy -> approve -> sell

use std::sync::Arc;
use eyre::Result;
use alloy_primitives::{Address, U256};
use tx_simulator::{TxSimulator, UnsignedTransaction};
use crate::tx_processor::TxProcessor;

use super::{
    config::PoolViabilityConfig,
    types::{PoolViabilityResult, PoolType},
    pool_adapters::{PoolAdapter, UniswapV2Adapter, UniswapV3Adapter},
    tax_calculator::{
        calculate_buy_tax_from_processed_transaction,
        TaxCalculationResult,
        // calculate_sell_tax_from_processed_transaction, // TODO: Implement
    },
};

use crate::tx_processor::data_models::ProcessedTransaction;
use crate::utils::to_checksum_address;

/// Check if a token pool allows buying and selling tokens
/// 
/// This function simulates the complete trading lifecycle sequentially:
/// 1. Optional setup transaction (e.g., enable trading)
/// 2. Buy tokens from pool using ETH
/// 3. Approve token spending to router
/// 4. Sell tokens back to pool for ETH
/// 
/// The simulation maintains blockchain state between each transaction using SimulationChain,
/// so the buy transaction's state changes are visible to the approve transaction,
/// and both are visible to the sell transaction. This allows accurate tax calculation
/// and detection of trading restrictions.
/// 
/// Returns comprehensive analysis including tax percentages and trading viability
pub async fn check_can_buy_sell_pool(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolViabilityConfig,
) -> Result<PoolViabilityResult> {
    // Get the block number to simulate at
    let block_number = match config.block_number {
        Some(block) => block,
        None => simulator.get_latest_block()?,
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
    
    // Start simulation chain for state preservation
    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;
    
    // Track results
    let mut total_gas_used = 0u64;
    let mut setup_tx_result = None;
    let mut can_buy = false;
    let mut can_approve = false; 
    let mut can_sell = false;
    
    // Step 1: Optional setup transaction (if provided via prior_tx)
    if let Some(prior_tx) = &config.prior_tx {
        let setup_call = UnsignedTransaction {
            from: Some(prior_tx.from_address),
            to: prior_tx.to_address,
            value: Some(prior_tx.value),
            data: Some(prior_tx.input.clone().into()),
            gas: Some(config.gas_limit),
            gas_price: Some(config.gas_price),
            max_fee_per_gas: prior_tx.fees.max_fee_per_gas.map(|v| v.try_into().unwrap_or(0)),
            max_priority_fee_per_gas: prior_tx.fees.max_priority_fee.map(|v| v.try_into().unwrap_or(0)),
            nonce: None,
        };
        
        let setup_sim_result = chain.step_with_trace(setup_call.clone()).await?;
        total_gas_used += setup_sim_result.gas_used;
        
        let setup_processed = tx_processor.process_transaction_from_simulation_result(
            &setup_call,
            &setup_sim_result,
            block_number,
            0, // tx_index
        ).await?;
        
        setup_tx_result = Some(setup_processed);
        
        if !setup_sim_result.success {
            return Ok(create_failed_result(
                config,
                block_number,
                setup_tx_result,
                None,
                None,
                None,
                format!("Setup transaction failed: {:?}", setup_sim_result.revert_reason),
                false,
                false,
                false,
            ));
        }
    }
    
    // Step 2: Buy tokens with ETH
    println!("🔄 Executing BUY transaction...");
    let buy_tx = pool_adapter.build_buy_transaction(
        config.token_address,
        config.test_amount,
        config.buyer_address,
        config.slippage_tolerance,
    )?;
    
    let buy_sim_result = chain.step_with_trace(buy_tx.clone()).await?;
    total_gas_used += buy_sim_result.gas_used;
    
    println!("  Result: success={}, gas_used={}", buy_sim_result.success, buy_sim_result.gas_used);
    
    let buy_processed = tx_processor.process_transaction_from_simulation_result(
        &buy_tx,
        &buy_sim_result,
        block_number,
        1, // tx_index
    ).await?;
    
    can_buy = buy_sim_result.success;
    
    // Extract tokens received from buy transaction
    let tokens_received = if can_buy {
        extract_tokens_received_from_processed_transaction(
            &buy_processed,
            config.buyer_address,
            config.token_address,
            config.token_decimals,
        )
    } else {
        U256::ZERO
    };
    
    println!("  📊 Tokens received from buy: {}", tokens_received);
    
    if !can_buy {
        return Ok(create_failed_result(
            config,
            block_number,
            setup_tx_result,
            Some(buy_processed),
            None,
            None,
            format!("Buy transaction failed: {:?}", buy_sim_result.revert_reason),
            false,
            false,
            false,
        ));
    }
    
    // Step 3: Approve tokens
    let approve_tx = pool_adapter.build_approve_transaction(
        config.token_address,
        U256::MAX,
        config.buyer_address,
    )?;
    
    let approve_sim_result = chain.step_with_trace(approve_tx.clone()).await?;
    total_gas_used += approve_sim_result.gas_used;
    
    let approve_processed = tx_processor.process_transaction_from_simulation_result(
        &approve_tx,
        &approve_sim_result,
        block_number,
        2, // tx_index
    ).await?;
    
    can_approve = approve_sim_result.success;
    
    if !can_approve {
        return Ok(create_failed_result(
            config,
            block_number,
            setup_tx_result,
            Some(buy_processed),
            Some(approve_processed),
            None,
            format!("Approve transaction failed: {:?}", approve_sim_result.revert_reason),
            true, // can_buy succeeded
            false,
            false,
        ));
    }
    
    // Step 4: Sell tokens (with block delay if specified)
    if config.block_delay > 0 {
        // Advance the chain to the sell block
        let sell_block = block_number + config.block_delay;
        chain = simulator.start_simulation_chain(Some(sell_block)).await?;
        
        // Re-apply buy and approve state changes to the new block
        // This ensures the token balance is available for selling
        let _ = chain.step_with_trace(buy_tx).await?;
        let _ = chain.step_with_trace(approve_tx).await?;
    }
    
    println!("🔄 Building SELL transaction with {} tokens...", tokens_received);
    let sell_tx = pool_adapter.build_sell_transaction(
        config.token_address,
        tokens_received,
        config.buyer_address,
        config.slippage_tolerance,
    )?;
    
    println!("🔄 Executing SELL in {}...", 
        if config.block_delay > 0 { 
            format!("block {}", block_number + config.block_delay) 
        } else { 
            "same block".to_string() 
        }
    );
    
    let sell_sim_result = chain.step_with_trace(sell_tx.clone()).await?;
    total_gas_used += sell_sim_result.gas_used;
    
    println!("  Sell Result: success={}, gas_used={}", sell_sim_result.success, sell_sim_result.gas_used);
    
    let sell_processed = tx_processor.process_transaction_from_simulation_result(
        &sell_tx,
        &sell_sim_result,
        if config.block_delay > 0 { block_number + config.block_delay } else { block_number },
        3, // tx_index
    ).await?;
    
    can_sell = sell_sim_result.success;
    
    // Calculate taxes
    let buy_tax = calculate_buy_tax_from_processed_transaction(
        &buy_processed,
        config.pool_address,
        config.buyer_address,
        config.token_address,
    );
    
    // TODO: Implement sell tax calculation
    let sell_tax = TaxCalculationResult::InvalidSimulation { 
        reason: "Sell tax calculation not yet implemented".to_string() 
    };
    
    let eth_received = extract_eth_received_from_processed_transaction(
        &sell_processed,
        config.buyer_address,
    );
    
    // Convert eth_received from Option<f64> to U256
    let eth_received_u256 = match eth_received {
        Some(amount) => {
            // Convert from ETH decimal to wei
            let wei_amount = (amount * 1e18) as u128;
            U256::from(wei_amount)
        },
        None => U256::ZERO,
    };
    
    // Create the result
    Ok(PoolViabilityResult {
        pool_type: config.pool_type,
        pool_address: config.pool_address,
        token_address: config.token_address,
        can_buy,
        can_approve,
        can_sell,
        is_tradeable: can_buy && can_approve && can_sell,
        buy_tax_percent: if can_buy { buy_tax.as_percentage().unwrap_or(0.0) } else { -1.0 },
        sell_tax_percent: if can_sell { sell_tax.as_percentage().unwrap_or(0.0) } else { -1.0 },
        tokens_received,
        eth_spent: config.test_amount,
        eth_received: eth_received_u256,
        buy_transaction: buy_processed,
        sell_transaction: sell_processed,
        approve_transaction: approve_processed,
        prior_transaction: setup_tx_result,
        failure_reason: if can_buy && can_approve && can_sell {
            None
        } else {
            Some(format!("Trading failed at step: {}",
                if !can_buy { "BUY" }
                else if !can_approve { "APPROVE" }
                else { "SELL" }
            ))
        },
        block_number,
    })
}

/// Extract tokens received from a processed transaction
fn extract_tokens_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    token_address: Address,
    token_decimals: u8,
) -> U256 {
    use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
    
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        // Check if token is in DENOM_ADDRESSES (known token)
        if let Some(symbol) = get_token_symbol(&token_address) {
            // Known token - look in currency_net using symbol
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                if amount > U256::ZERO {
                    // Amount is already in smallest unit (wei for ETH, etc)
                    return amount;
                }
            }
        } else {
            // Unknown token - look in token_net using checksummed address
            let token_key = to_checksum_address(&token_address);
            if let Some(&amount) = balance_changes.token_net.get(&token_key) {
                if amount > U256::ZERO {
                    // Already raw amount with decimals
                    return amount;
                }
            }
        }
    }
    
    U256::ZERO
}

/// Extract ETH received from a processed transaction
fn extract_eth_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
) -> Option<f64> {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if let Some(&eth_amount) = balance_changes.currency_net.get("ETH") {
            if eth_amount > U256::ZERO {
                // Convert from wei to ETH for display purposes
                let eth_value = eth_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                return Some(eth_value);
            }
        }
    }
    None
}

/// Create a failed result with appropriate flags
fn create_failed_result(
    config: PoolViabilityConfig,
    block_number: u64,
    setup_tx: Option<crate::tx_processor::data_models::ProcessedTransaction>,
    buy_tx: Option<crate::tx_processor::data_models::ProcessedTransaction>,
    approve_tx: Option<crate::tx_processor::data_models::ProcessedTransaction>,
    sell_tx: Option<crate::tx_processor::data_models::ProcessedTransaction>,
    failure_reason: String,
    can_buy: bool,
    can_approve: bool,
    can_sell: bool,
) -> PoolViabilityResult {
    // Create dummy transactions for missing steps
    let dummy_tx = crate::tx_processor::data_models::ProcessedTransaction {
        hash: Default::default(),
        block_number,
        block_timestamp: 0,
        txn_index: 0,
        from_address: config.buyer_address,
        to_address: None,
        contract_address: None,
        value: U256::ZERO,
        status: "0".to_string(),
        nonce: 0,
        
        // Transaction classification
        txn_type: "UNKNOWN".to_string(),
        actions: vec![],
        
        // Fee information
        fees: Default::default(),
        bribe_amount: 0.0,
        
        // Addresses and contracts involved
        unique_addresses: Default::default(),
        erc20_contracts: Default::default(),
        erc721_contracts: Default::default(),
        erc1155_contracts: Default::default(),
        
        // Transfer events
        eth_transfers: vec![],
        erc20_transfers: vec![],
        erc721_transfers: vec![],
        erc1155_transfers: vec![],
        internal_transactions: vec![],
        
        // DEX events
        uniswap_v2_syncs: vec![],
        uniswap_v2_swaps: vec![],
        uniswap_v3_pools: vec![],
        uniswap_v3_initializations: vec![],
        uniswap_v3_burns: vec![],
        uniswap_v3_mints: vec![],
        uniswap_v3_swaps: vec![],
        uniswap_v3_positions: vec![],
        uniswap_v3_increases: vec![],
        uniswap_v3_decreases: vec![],
        uniswap_v4_initializes: vec![],
        uniswap_v4_modifies: vec![],
        uniswap_v4_swaps: vec![],
        permit2_events: vec![],
        
        // Other events and actions
        approvals: vec![],
        erc721_approvals: vec![],
        mints: vec![],
        burns: vec![],
        deposits: vec![],
        withdraws: vec![],
        pair_events: vec![],
        owner_events: vec![],
        contract_creation_events: vec![],
        trading_enabled_events: vec![],
        trading_disabled_events: vec![],
        
        // Other fields  
        other_events: vec![],
        address_balance_changes: Default::default(),
        latest_states: Default::default(),
        input: vec![],
    };
    
    PoolViabilityResult {
        pool_type: config.pool_type,
        pool_address: config.pool_address,
        token_address: config.token_address,
        can_buy,
        can_approve,
        can_sell,
        is_tradeable: false,
        buy_tax_percent: 0.0,
        sell_tax_percent: 0.0,
        tokens_received: U256::ZERO,
        eth_spent: config.test_amount,
        eth_received: U256::ZERO,
        buy_transaction: buy_tx.unwrap_or_else(|| dummy_tx.clone()),
        sell_transaction: sell_tx.unwrap_or_else(|| dummy_tx.clone()),
        approve_transaction: approve_tx.unwrap_or_else(|| dummy_tx.clone()),
        prior_transaction: setup_tx,
        failure_reason: Some(failure_reason),
        block_number,
    }
}