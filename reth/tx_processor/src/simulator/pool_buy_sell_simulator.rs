/// Core trading viability analyzer - Direct implementation without redundant layers
use std::sync::Arc;
use eyre::Result;
use alloy_primitives::{Address, U256};
use tx_simulator::{TxSimulator, UnsignedTransaction};
use crate::tx_processor::TxProcessor;

use super::{
    config::PoolViabilityConfig,
    types::{PoolViabilityResult, PoolType},
};
use crate::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction,
    calculate_sell_tax_from_processed_transaction,
};

use crate::tx_processor::data_models::ProcessedTransaction;

pub async fn check_can_buy_sell_pool(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolViabilityConfig,
) -> Result<PoolViabilityResult> {
    let block_number = match config.block_number { Some(b) => b, None => simulator.get_latest_block()? };

    let route = match config.pool_type {
        PoolType::UniswapV2 => reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute::UniswapV2 { pool: config.pool_address },
        PoolType::SushiSwap => reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute::SushiswapV2 { pool: config.pool_address },
        PoolType::UniswapV3 { fee_tier } => reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute::UniswapV3 { pool: config.pool_address, fee_tier },
        PoolType::UniswapV4 => {
            // Return a well-formed failure result to callers with a clear reason
            return Ok(create_failed_result(
                config,
                block_number,
                None,
                None,
                None,
                None,
                "Uniswap V4 swap simulation not yet implemented (PoolManager lock/Router integration required)".to_string(),
                false,
                false,
                false,
            ));
        }
        _ => return Err(eyre::eyre!("Pool type {:?} not yet implemented", config.pool_type)),
    };

    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;
    let mut setup_tx_result = None;

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
        let setup_processed = tx_processor.process_transaction_from_simulation_result(&setup_call,&setup_sim_result,block_number,0).await?;
        setup_tx_result = Some(setup_processed);
        if !setup_sim_result.success {
            return Ok(create_failed_result(config, block_number, setup_tx_result, None, None, None, format!("Setup transaction failed: {:?}", setup_sim_result.revert_reason), false, false, false));
        }
    }

    // BUY
    let slippage_bps = (config.slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let buy_tx = reth_chain_query::tx_builders::build_buy_swap(&route, config.buyer_address, config.token_address, config.test_amount, slippage_bps, deadline);
    let buy_sim_result = chain.step_with_trace(buy_tx.clone()).await?;
    let buy_processed = tx_processor.process_transaction_from_simulation_result(&buy_tx, &buy_sim_result, block_number, 1).await?;
    let can_buy = buy_sim_result.success;
    let tokens_received = if can_buy { extract_tokens_received_from_processed_transaction(&buy_processed, config.buyer_address, config.token_address, config.token_decimals) } else { U256::ZERO };
    if !can_buy {
        return Ok(create_failed_result(config, block_number, setup_tx_result, Some(buy_processed), None, None, format!("Buy transaction failed: {:?}", buy_sim_result.revert_reason), false, false, false));
    }

    // APPROVE
    let approve_tx = reth_chain_query::tx_builders::build_approve_for_route(&route, config.buyer_address, config.token_address, U256::MAX);
    let approve_sim_result = chain.step_with_trace(approve_tx.clone()).await?;
    let approve_processed = tx_processor.process_transaction_from_simulation_result(&approve_tx, &approve_sim_result, block_number, 2).await?;
    let can_approve = approve_sim_result.success;
    if !can_approve {
        return Ok(create_failed_result(config, block_number, setup_tx_result, Some(buy_processed), Some(approve_processed), None, format!("Approve transaction failed: {:?}", approve_sim_result.revert_reason), true, false, false));
    }

    // SELL (optional delay)
    if config.block_delay > 0 {
        let requested_sell_block = block_number + config.block_delay;
        let latest = simulator.get_latest_block()?;
        let sell_block = if requested_sell_block > latest { latest } else { requested_sell_block };
        chain = simulator.start_simulation_chain(Some(sell_block)).await?;
        let _ = chain.step_with_trace(buy_tx).await?;
        let _ = chain.step_with_trace(approve_tx).await?;
    }
    let sell_tx = reth_chain_query::tx_builders::build_sell_swap(&route, config.buyer_address, config.token_address, tokens_received, slippage_bps, deadline);
    let sell_block = if config.block_delay > 0 { block_number + config.block_delay } else { block_number };
    let sell_sim_result = chain.step_with_trace(sell_tx.clone()).await?;
    let sell_processed = tx_processor.process_transaction_from_simulation_result(&sell_tx, &sell_sim_result, sell_block, 3).await?;
    let can_sell = sell_sim_result.success;

    // Taxes and ETH
    let buy_tax = calculate_buy_tax_from_processed_transaction(&buy_processed, config.pool_address, config.buyer_address, config.token_address);
    let sell_tax = calculate_sell_tax_from_processed_transaction(&sell_processed, config.pool_address, config.buyer_address);
    let eth_received_u256 = extract_eth_received_from_processed_transaction(&sell_processed, config.buyer_address).unwrap_or(U256::ZERO);

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
        failure_reason: if can_buy && can_approve && can_sell { None } else { Some(format!("Trading failed at step: {}", if !can_buy { "BUY" } else if !can_approve { "APPROVE" } else { "SELL" })) },
        block_number,
    })
}

fn extract_tokens_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    token_address: Address,
    _token_decimals: u8,
) -> U256 {
    use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
    use reth_chain_query::to_checksum_address;
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if let Some(symbol) = get_token_symbol(&token_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                if amount > U256::ZERO { return amount; }
            }
        } else {
            let token_key = to_checksum_address(&token_address);
            if let Some(&amount) = balance_changes.token_net.get(&token_key) {
                if amount > U256::ZERO { return amount; }
            }
        }
    }
    U256::ZERO
}

fn extract_eth_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
) -> Option<U256> {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if let Some(&eth_amount) = balance_changes.currency_net.get("ETH") {
            if eth_amount > U256::ZERO { return Some(eth_amount); }
        }
    }
    None
}

fn create_failed_result(
    config: PoolViabilityConfig,
    block_number: u64,
    setup_tx: Option<ProcessedTransaction>,
    buy_tx: Option<ProcessedTransaction>,
    approve_tx: Option<ProcessedTransaction>,
    sell_tx: Option<ProcessedTransaction>,
    failure_reason: String,
    can_buy: bool,
    can_approve: bool,
    can_sell: bool,
) -> PoolViabilityResult {
    let dummy_tx = ProcessedTransaction {
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
        txn_type: "UNKNOWN".to_string(),
        actions: vec![],
        fees: Default::default(),
        bribe_amount: 0.0,
        unique_addresses: Default::default(),
        erc20_contracts: Default::default(),
        erc721_contracts: Default::default(),
        erc1155_contracts: Default::default(),
        eth_transfers: vec![],
        erc20_transfers: vec![],
        erc721_transfers: vec![],
        erc1155_transfers: vec![],
        internal_transactions: vec![],
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
