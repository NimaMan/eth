use crate::tx_processor::TxProcessor;
use alloy_primitives::{Address, I256, U256};
use eyre::{eyre, Result};
use reth_primitives::{Header as _, SealedHeader};
use reth_provider::{AccountReader, HeaderProvider};
use std::{convert::TryFrom, sync::Arc};
use tx_simulator::{TxSimulator, UnsignedTransaction};

use super::types::{PoolBuySellParameters, PoolBuySellSimulationResult, PoolType};
use crate::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction,
};

use crate::tx_processor::data_models::ProcessedTransaction;
use reth_chain_query::tx_builders::amm::{
    build_router_deploy_tx as build_v4_router_deploy_tx,
    build_swap_exact_input_single_tx as build_v4_swap_tx,
    build_token_approval_tx as build_v4_token_approval_tx,
    build_weth_deposit_tx as build_v4_weth_deposit_tx,
    compute_contract_address as compute_v4_contract_address, UniswapV4PoolKey as BuilderV4PoolKey,
};

pub async fn check_can_buy_sell_pool(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
) -> Result<PoolBuySellSimulationResult> {
    if config.token_decimals == 0 {
        return Err(eyre!("token_decimals must be provided (non-zero)"));
    }

    if matches!(config.pool_type, PoolType::UniswapV4) {
        return check_can_buy_sell_uniswap_v4(simulator, tx_processor, config).await;
    }

    let block_number = match config.block_number {
        Some(b) => b,
        None => simulator.get_latest_block()?,
    };

    let route = match config.pool_type {
        PoolType::UniswapV2 => {
            reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute::UniswapV2 {
                pool: config.pool_address,
            }
        }
        PoolType::SushiSwap => {
            reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute::SushiswapV2 {
                pool: config.pool_address,
            }
        }
        PoolType::UniswapV3 { fee_tier } => {
            reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute::UniswapV3 {
                pool: config.pool_address,
                fee_tier,
            }
        }
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
        _ => {
            return Err(eyre::eyre!(
                "Pool type {:?} not yet implemented",
                config.pool_type
            ))
        }
    };

    let (mut chain, base_fee) = match config.block_header.clone() {
        Some(block_header) => {
            let base_fee = block_header
                .header()
                .base_fee_per_gas
                .map(|fee| fee as u128);
            let chain = simulator
                .start_simulation_chain(None, Some(block_header))
                .await?;
            (chain, base_fee)
        }
        None => {
            let provider = simulator.provider_factory().provider()?;
            let header = provider
                .header_by_number(block_number)?
                .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;
            let base_fee = header.base_fee_per_gas.map(|fee| fee as u128);
            let sealed = SealedHeader::seal_slow(header);
            let chain = simulator.start_simulation_chain(None, Some(sealed)).await?;
            (chain, base_fee)
        }
    };
    let mut setup_tx_result = None;

    if let Some(prior_tx) = &config.prior_tx {
        let prior_gas_limit = config.prior_gas_limit.unwrap_or(config.buy_gas_limit);
        let prior_max_fee = config.prior_max_fee_per_gas.or_else(|| {
            prior_tx
                .fees
                .max_fee_per_gas
                .and_then(|v| u128::try_from(v).ok())
        });
        let prior_max_priority = config.prior_max_priority_fee_per_gas.or_else(|| {
            prior_tx
                .fees
                .max_priority_fee
                .and_then(|v| u128::try_from(v).ok())
        });

        let prior_gas_price = if prior_max_fee.is_none() {
            u128::try_from(prior_tx.fees.gas_price).ok()
        } else {
            None
        };

        let mut setup_call = UnsignedTransaction {
            from: Some(prior_tx.from_address),
            to: prior_tx.to_address,
            value: Some(prior_tx.value),
            data: Some(prior_tx.input.clone().into()),
            gas: Some(prior_gas_limit),
            gas_price: prior_gas_price,
            max_fee_per_gas: prior_max_fee,
            max_priority_fee_per_gas: prior_max_priority,
            nonce: None,
        };
        let has_explicit_fee = setup_call.gas_price.is_some()
            || setup_call.max_fee_per_gas.is_some()
            || setup_call.max_priority_fee_per_gas.is_some();
        if !has_explicit_fee {
            apply_fee_policy(&mut setup_call, &config, base_fee);
        } else if setup_call.gas.is_none() {
            setup_call.gas = Some(prior_gas_limit);
        }
        let setup_sim_result = chain.step_with_trace(setup_call.clone()).await?;
        let setup_processed = tx_processor
            .process_transaction_from_simulation_result(
                &setup_call,
                &setup_sim_result,
                block_number,
                0,
            )
            .await?;
        setup_tx_result = Some(setup_processed);
        if !setup_sim_result.success {
            return Ok(create_failed_result(
                config,
                block_number,
                setup_tx_result,
                None,
                None,
                None,
                format!(
                    "Setup transaction failed: {:?}",
                    setup_sim_result.revert_reason
                ),
                false,
                false,
                false,
            ));
        }
    }

    // BUY
    let slippage_bps = (config.slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let mut buy_tx = reth_chain_query::tx_builders::build_buy_swap(
        &route,
        config.buyer_address,
        config.token_address,
        config.test_amount,
        slippage_bps,
        deadline,
    );
    buy_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut buy_tx, &config, base_fee);
    let buy_sim_result = chain.step_with_trace(buy_tx.clone()).await?;
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(&buy_tx, &buy_sim_result, block_number, 1)
        .await?;
    let can_buy = buy_sim_result.success;
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

    // APPROVE
    let mut approve_tx = reth_chain_query::tx_builders::build_approve_for_route(
        &route,
        config.buyer_address,
        config.token_address,
        U256::MAX,
    );
    approve_tx.gas = Some(config.approve_gas_limit);
    apply_fee_policy(&mut approve_tx, &config, base_fee);
    let approve_sim_result = chain.step_with_trace(approve_tx.clone()).await?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(
            &approve_tx,
            &approve_sim_result,
            block_number,
            2,
        )
        .await?;
    let can_approve = approve_sim_result.success;
    if !can_approve {
        return Ok(create_failed_result(
            config,
            block_number,
            setup_tx_result,
            Some(buy_processed),
            Some(approve_processed),
            None,
            format!(
                "Approve transaction failed: {:?}",
                approve_sim_result.revert_reason
            ),
            true,
            false,
            false,
        ));
    }

    // SELL (optional delay)
    if config.block_delay > 0 {
        let requested_sell_block = block_number + config.block_delay;
        let latest = simulator.get_latest_block()?;
        let sell_block = if requested_sell_block > latest {
            latest
        } else {
            requested_sell_block
        };
        let provider = simulator.provider_factory().provider()?;
        let header = provider
            .header_by_number(sell_block)?
            .ok_or_else(|| eyre!("No header for block {}", sell_block))?;
        let sealed = SealedHeader::seal_slow(header);
        chain = simulator.start_simulation_chain(None, Some(sealed)).await?;
        let _ = chain.step_with_trace(buy_tx).await?;
        let _ = chain.step_with_trace(approve_tx).await?;
    }
    let mut sell_tx = reth_chain_query::tx_builders::build_sell_swap(
        &route,
        config.buyer_address,
        config.token_address,
        tokens_received,
        slippage_bps,
        deadline,
    );
    sell_tx.gas = Some(config.sell_gas_limit);
    apply_fee_policy(&mut sell_tx, &config, base_fee);
    let sell_block = if config.block_delay > 0 {
        block_number + config.block_delay
    } else {
        block_number
    };
    let sell_sim_result = chain.step_with_trace(sell_tx.clone()).await?;
    let sell_processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim_result, sell_block, 3)
        .await?;
    let can_sell = sell_sim_result.success;

    // Taxes and ETH
    let buy_tax = calculate_buy_tax_from_processed_transaction(
        &buy_processed,
        config.pool_address,
        config.buyer_address,
        config.token_address,
    );
    let sell_tax = calculate_sell_tax_from_processed_transaction(
        &sell_processed,
        config.pool_address,
        config.buyer_address,
    );
    let eth_received_u256 =
        extract_eth_received_from_processed_transaction(&sell_processed, config.buyer_address)
            .unwrap_or(U256::ZERO);

    let mut failure_reason = None;
    if !(can_buy && can_approve && can_sell) {
        let failing_step = if !can_buy {
            "BUY"
        } else if !can_approve {
            "APPROVE"
        } else {
            "SELL"
        };

        let mut message = format!("Trading failed at step: {}", failing_step);
        if !can_sell {
            if let Some(ref revert) = sell_sim_result.revert_reason {
                message.push_str(&format!(" (sell revert: {revert})"));
            }
        }
        failure_reason = Some(message);
    }

    Ok(PoolBuySellSimulationResult {
        pool_type: config.pool_type,
        pool_address: config.pool_address,
        token_address: config.token_address,
        can_buy,
        can_approve,
        can_sell,
        is_tradeable: can_buy && can_approve && can_sell,
        buy_tax_percent: if can_buy {
            buy_tax.as_percentage().unwrap_or(0.0)
        } else {
            -1.0
        },
        sell_tax_percent: if can_sell {
            sell_tax.as_percentage().unwrap_or(0.0)
        } else {
            -1.0
        },
        tokens_received,
        eth_spent: config.test_amount,
        eth_received: eth_received_u256,
        buy_transaction: buy_processed,
        sell_transaction: sell_processed,
        approve_transaction: approve_processed,
        prior_transaction: setup_tx_result,
        failure_reason,
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

fn extract_eth_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
) -> Option<U256> {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if let Some(&eth_amount) = balance_changes.currency_net.get("ETH") {
            if eth_amount > I256::ZERO {
                return Some(eth_amount.unsigned_abs());
            }
        }
    }
    None
}

fn apply_fee_policy(
    tx: &mut UnsignedTransaction,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
) {
    let effective_base = base_fee.or(config.gas_price).unwrap_or(20_000_000_000); // 20 gwei fallback pre-London
    let default_tip = std::cmp::max(effective_base / 10, 1_000_000_000); // 10% of base, min 1 gwei

    if tx.gas.is_none() {
        tx.gas = Some(config.buy_gas_limit);
    }

    if let Some(price) = tx.gas_price {
        if price == 0 && config.gas_price.is_some() {
            tx.gas_price = config.gas_price;
        }
        return;
    }

    if let Some(mut max_fee) = tx.max_fee_per_gas {
        let priority_fee = tx
            .max_priority_fee_per_gas
            .or(config.max_priority_fee_per_gas)
            .unwrap_or(default_tip);

        let min_required = effective_base.saturating_add(priority_fee);
        if max_fee < min_required {
            max_fee = min_required;
        }

        tx.max_priority_fee_per_gas = Some(priority_fee);
        tx.max_fee_per_gas = Some(max_fee);
        tx.gas_price = None;
        return;
    }

    let priority_fee = tx
        .max_priority_fee_per_gas
        .or(config.max_priority_fee_per_gas)
        .unwrap_or(default_tip);

    let mut max_fee = tx
        .max_fee_per_gas
        .or(config.max_fee_per_gas)
        .unwrap_or_else(|| {
            effective_base
                .saturating_mul(2)
                .saturating_add(priority_fee)
        });

    let min_required = effective_base.saturating_add(priority_fee);
    if max_fee < min_required {
        max_fee = min_required;
    }

    tx.gas_price = None;
    tx.max_priority_fee_per_gas = Some(priority_fee);
    tx.max_fee_per_gas = Some(max_fee);
}

async fn check_can_buy_sell_uniswap_v4(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    mut config: PoolBuySellParameters,
) -> Result<PoolBuySellSimulationResult> {
    if config.block_delay > 0 {
        return Err(eyre!(
            "Uniswap V4 pool simulation currently does not support block delays"
        ));
    }

    let v4_cfg = config
        .uniswap_v4_config
        .clone()
        .ok_or_else(|| eyre!("Uniswap V4 configuration must be provided"))?;

    let block_number = config.block_number.unwrap_or(simulator.get_latest_block()?);

    let (mut chain, base_fee) = match config.block_header.clone() {
        Some(block_header) => {
            let base_fee = block_header
                .header()
                .base_fee_per_gas
                .map(|fee| fee as u128);
            let chain = simulator
                .start_simulation_chain(None, Some(block_header))
                .await?;
            (chain, base_fee)
        }
        None => {
            let provider = simulator.provider_factory().provider()?;
            let header = provider
                .header_by_number(block_number)?
                .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;
            let base_fee = header.base_fee_per_gas.map(|fee| fee as u128);
            let sealed = SealedHeader::seal_slow(header);
            let chain = simulator.start_simulation_chain(None, Some(sealed)).await?;
            (chain, base_fee)
        }
    };

    let provider = simulator.provider_factory().provider()?;
    let deployer_nonce = provider
        .basic_account(&config.buyer_address)?
        .map(|acc| acc.nonce)
        .unwrap_or(0);

    let pool_key = BuilderV4PoolKey {
        currency0: v4_cfg.currency0,
        currency1: v4_cfg.currency1,
        fee: v4_cfg.fee,
        tick_spacing: v4_cfg.tick_spacing,
        hooks: v4_cfg.hooks,
    };

    let (buy_zero_for_one, buy_input_currency, _buy_output_currency) =
        determine_swap_direction(&pool_key, config.token_address)?;

    if buy_input_currency != Address::ZERO && buy_input_currency != config.weth_address {
        return Err(eyre!(
            "Uniswap V4 helper currently supports pools where the input currency is WETH or native ETH"
        ));
    }

    let mut deploy_tx = build_v4_router_deploy_tx(
        config.buyer_address,
        v4_cfg.pool_manager,
        config.weth_address,
    );
    apply_fee_policy(&mut deploy_tx, &config, base_fee);
    let deploy_result = chain.step_with_trace(deploy_tx.clone()).await?;
    let deploy_processed = tx_processor
        .process_transaction_from_simulation_result(&deploy_tx, &deploy_result, block_number, 0)
        .await?;
    if !deploy_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            Some(deploy_processed),
            None,
            None,
            None,
            format!(
                "Router deployment failed: {:?}",
                deploy_result.revert_reason
            ),
            false,
            false,
            false,
        ));
    }
    let mut setup_tx_result: Option<ProcessedTransaction> = Some(deploy_processed);
    let router_address = compute_v4_contract_address(config.buyer_address, deployer_nonce);

    if buy_input_currency == config.weth_address {
        let mut deposit_tx = build_v4_weth_deposit_tx(
            config.buyer_address,
            config.weth_address,
            config.test_amount,
        );
        deposit_tx.gas = Some(config.buy_gas_limit);
        apply_fee_policy(&mut deposit_tx, &config, base_fee);
        let deposit_result = chain.step_with_trace(deposit_tx.clone()).await?;
        let deposit_processed = tx_processor
            .process_transaction_from_simulation_result(
                &deposit_tx,
                &deposit_result,
                block_number,
                0,
            )
            .await?;
        if !deposit_result.success {
            return Ok(create_failed_result(
                config,
                block_number,
                Some(deposit_processed),
                None,
                None,
                None,
                format!("WETH deposit failed: {:?}", deposit_result.revert_reason),
                false,
                false,
                false,
            ));
        }
        setup_tx_result = Some(deposit_processed);
    }

    let mut buy_tx = build_v4_swap_tx(
        router_address,
        config.buyer_address,
        &pool_key,
        buy_zero_for_one,
        config.test_amount,
        U256::ZERO,
        config.buyer_address,
        false,
        &v4_cfg.hook_data,
    )?;
    buy_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut buy_tx, &config, base_fee);
    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(&buy_tx, &buy_result, block_number, 1)
        .await?;
    if !buy_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            setup_tx_result,
            Some(buy_processed),
            None,
            None,
            format!("Buy transaction failed: {:?}", buy_result.revert_reason),
            false,
            false,
            false,
        ));
    }

    let tokens_received = extract_tokens_received_from_processed_transaction(
        &buy_processed,
        config.buyer_address,
        config.token_address,
        config.token_decimals,
    );

    let mut approve_tx = build_v4_token_approval_tx(
        config.buyer_address,
        config.token_address,
        router_address,
        U256::MAX,
    );
    approve_tx.gas = Some(config.approve_gas_limit);
    apply_fee_policy(&mut approve_tx, &config, base_fee);
    let approve_result = chain.step_with_trace(approve_tx.clone()).await?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(&approve_tx, &approve_result, block_number, 2)
        .await?;
    if !approve_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            setup_tx_result,
            Some(buy_processed),
            Some(approve_processed),
            None,
            format!(
                "Approve transaction failed: {:?}",
                approve_result.revert_reason
            ),
            true,
            false,
            false,
        ));
    }

    let sell_zero_for_one = !buy_zero_for_one;
    let (_, sell_output_currency) = determine_swap_currencies(&pool_key, sell_zero_for_one);
    let unwrap_native =
        sell_output_currency == config.weth_address || sell_output_currency == Address::ZERO;

    let mut sell_tx = build_v4_swap_tx(
        router_address,
        config.buyer_address,
        &pool_key,
        sell_zero_for_one,
        tokens_received,
        U256::ZERO,
        config.buyer_address,
        unwrap_native,
        &v4_cfg.hook_data,
    )?;
    sell_tx.gas = Some(config.sell_gas_limit);
    apply_fee_policy(&mut sell_tx, &config, base_fee);
    let sell_result = chain.step_with_trace(sell_tx.clone()).await?;
    let sell_processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_result, block_number, 3)
        .await?;
    if !sell_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            setup_tx_result,
            Some(buy_processed),
            Some(approve_processed),
            Some(sell_processed),
            format!("Sell transaction failed: {:?}", sell_result.revert_reason),
            true,
            true,
            false,
        ));
    }

    let buy_tax = calculate_buy_tax_from_processed_transaction(
        &buy_processed,
        config.pool_address,
        config.buyer_address,
        config.token_address,
    );
    let sell_tax = calculate_sell_tax_from_processed_transaction(
        &sell_processed,
        config.pool_address,
        config.buyer_address,
    );

    let eth_received_u256 =
        extract_eth_received_from_processed_transaction(&sell_processed, config.buyer_address)
            .unwrap_or(U256::ZERO);

    Ok(PoolBuySellSimulationResult {
        pool_type: PoolType::UniswapV4,
        pool_address: config.pool_address,
        token_address: config.token_address,
        can_buy: true,
        can_approve: true,
        can_sell: true,
        is_tradeable: true,
        buy_tax_percent: buy_tax.as_percentage().unwrap_or(0.0),
        sell_tax_percent: sell_tax.as_percentage().unwrap_or(0.0),
        tokens_received,
        eth_spent: config.test_amount,
        eth_received: eth_received_u256,
        buy_transaction: buy_processed,
        sell_transaction: sell_processed,
        approve_transaction: approve_processed,
        prior_transaction: setup_tx_result,
        failure_reason: None,
        block_number,
    })
}

fn determine_swap_direction(
    key: &BuilderV4PoolKey,
    token_address: Address,
) -> Result<(bool, Address, Address)> {
    if token_address == key.currency1 {
        Ok((true, key.currency0, key.currency1))
    } else if token_address == key.currency0 {
        Ok((false, key.currency1, key.currency0))
    } else {
        Err(eyre!(
            "Token address {:x} is not part of the supplied Uniswap V4 pool key",
            token_address
        ))
    }
}

fn determine_swap_currencies(key: &BuilderV4PoolKey, zero_for_one: bool) -> (Address, Address) {
    if zero_for_one {
        (key.currency0, key.currency1)
    } else {
        (key.currency1, key.currency0)
    }
}

fn create_failed_result(
    config: PoolBuySellParameters,
    block_number: u64,
    setup_tx: Option<ProcessedTransaction>,
    buy_tx: Option<ProcessedTransaction>,
    approve_tx: Option<ProcessedTransaction>,
    sell_tx: Option<ProcessedTransaction>,
    failure_reason: String,
    can_buy: bool,
    can_approve: bool,
    can_sell: bool,
) -> PoolBuySellSimulationResult {
    let dummy_tx = ProcessedTransaction {
        hash: Default::default(),
        block_number,
        block_timestamp: 0,
        tx_index: 0,
        from_address: config.buyer_address,
        to_address: None,
        contract_address: None,
        value: U256::ZERO,
        status: "0".to_string(),
        nonce: 0,
        tx_type: "UNKNOWN".to_string(),
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
        struct_logs: None,
    };
    PoolBuySellSimulationResult {
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
