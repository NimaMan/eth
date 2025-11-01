use crate::tx_processor::TxProcessor;
use alloy_primitives::{Address, I256, U256};
use eyre::{eyre, Result, WrapErr};
use reth_primitives::{Header as _, SealedHeader};
use reth_provider::{AccountReader, HeaderProvider};
use std::convert::TryInto;
use std::sync::Arc;
use tx_simulator::types::CallFrame;
use tx_simulator::{TxSimulator, UnsignedTransaction};

use super::unsigned_tx_builder::UnsignedTxBuilder;

use super::types::{PoolBuySellParameters, PoolBuySellSimulationResult, PoolType};
use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
use crate::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction,
};

use crate::tx_processor::data_models::ProcessedTransaction;
use reth_chain_query::dex::{
    fetch_uniswap_v2_pair_address, fetch_uniswap_v3_pool_address, SUSHISWAP_FACTORY,
    UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY,
};
use reth_chain_query::to_checksum_address;
use reth_chain_query::tx_builders::amm::uniswap_v2::{
    build_approve_v2, build_token_to_token_swap_v2, Router as UniswapV2Router,
};
use reth_chain_query::tx_builders::amm::uniswap_v3::{
    build_approve_v3, build_token_to_token_swap_v3,
};
use reth_chain_query::tx_builders::amm::{
    build_baygus_router_deploy_tx, build_baygus_router_multihop_tx,
    build_baygus_single_hop_exact_input_call,
    build_token_approval_tx as build_v4_token_approval_tx,
    build_weth_deposit_tx as build_v4_weth_deposit_tx,
    build_weth_withdraw_tx as build_v4_weth_withdraw_tx,
    compute_contract_address as compute_v4_contract_address,
    infer_orientation_from_input as infer_v4_orientation_from_input,
    infer_orientation_from_output as infer_v4_orientation_from_output,
    UniswapV4BaygusSingleHopRequest as BuilderV4SingleHopRequest,
    UniswapV4PoolKey as BuilderV4PoolKey,
};

const WETH_DECIMALS: u8 = 18;
pub async fn check_can_buy_sell_pool(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
) -> Result<PoolBuySellSimulationResult> {
    let latest_available_block = simulator.get_latest_block()?;

    if let Some(explicit_block) = config.block_number {
        if latest_available_block < explicit_block {
            return Err(eyre!(
                "State for block {} not yet available (latest persisted block {})",
                explicit_block,
                latest_available_block
            ));
        }
    }

    if config.token_decimals == 0 {
        return Err(eyre!("token_decimals must be provided (non-zero)"));
    }

    if config.pool_address.is_zero() {
        return Err(eyre!(
            "Pool address must be non-zero for {:?} pools",
            config.pool_type
        ));
    }

    if matches!(
        config.pool_type,
        PoolType::UniswapV2 | PoolType::SushiSwap | PoolType::UniswapV3 { .. }
    ) && config.denom_address.is_zero()
    {
        return Err(eyre!(
            "denom_address must be provided via PoolBuySellParameters::with_denom_address() for {:?} pools",
            config.pool_type
        ));
    }

    if config.denom_decimals == 0 {
        return Err(eyre!(
            "denom_decimals must be provided via PoolBuySellParameters::with_denom_decimals()"
        ));
    }

    if matches!(config.pool_type, PoolType::UniswapV4) {
        return check_can_buy_sell_uniswap_v4(simulator, tx_processor, config).await;
    }

    let block_number = match config.block_number {
        Some(b) => b,
        None => latest_available_block,
    };

    validate_pool_registration(simulator.clone(), &config, block_number).await?;

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
                Vec::new(),
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
    let mut prior_tx_results: Vec<ProcessedTransaction> =
        Vec::with_capacity(config.prior_txs.len());

    for (idx, prior_tx) in config.prior_txs.iter().enumerate() {
        let mut setup_call = UnsignedTxBuilder::build_unsigned_from_processed_tx(prior_tx);
        if setup_call.gas.is_none() {
            setup_call.gas = if prior_tx.fees.gas_limit > 0 {
                Some(prior_tx.fees.gas_limit)
            } else if prior_tx.fees.gas_used > 0 {
                Some(prior_tx.fees.gas_used)
            } else {
                None
            };
        }
        override_prior_gas_price_with_header(base_fee, prior_tx, &mut setup_call);

        let has_explicit_fee = setup_call.gas_price.is_some()
            || setup_call.max_fee_per_gas.is_some()
            || setup_call.max_priority_fee_per_gas.is_some();
        if !has_explicit_fee {
            apply_fee_policy(&mut setup_call, &config, base_fee);
        }

        let prior_hash = format!("{:#x}", prior_tx.hash);
        let prior_nonce = prior_tx.nonce;
        let setup_sim_result = chain
            .step_with_trace(setup_call.clone())
            .await
            .map_err(|err| {
                let context = format!(
                    "while replaying prior tx {prior_hash} (index {idx}, nonce {prior_nonce}) with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                    setup_call.gas,
                    setup_call.gas_price,
                    setup_call.max_fee_per_gas,
                    setup_call.max_priority_fee_per_gas
                );
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "prior_replay",
                    %context,
                    block = block_number,
                    error = %err
                );
                eyre!("{}: {}", context, err)
            })?;
        let setup_processed = tx_processor
            .process_transaction_from_simulation_result(
                &setup_call,
                &setup_sim_result,
                block_number,
                idx as u64,
            )
            .await?;
        let succeeded = setup_sim_result.success;
        prior_tx_results.push(setup_processed);
        if !succeeded {
            let revert_reason = setup_sim_result
                .revert_reason
                .clone()
                .unwrap_or_else(|| "Unknown revert".to_string());
            let failure_message = format!(
                "Prior transaction {} (nonce {}) failed: {}",
                prior_hash, prior_nonce, revert_reason
            );
            return Ok(create_failed_result(
                config,
                block_number,
                prior_tx_results,
                None,
                None,
                None,
                failure_message,
                false,
                false,
                false,
            ));
        }
    }

    let mut denom_approve_tx_for_delay: Option<UnsignedTransaction> = None;
    if let Some(mut denom_approve_tx) = match config.pool_type {
        PoolType::UniswapV2 => Some(build_approve_v2(
            UniswapV2Router::UniswapV2,
            config.buyer_address,
            config.denom_address,
            config.test_amount,
        )),
        PoolType::SushiSwap => Some(build_approve_v2(
            UniswapV2Router::SushiswapV2,
            config.buyer_address,
            config.denom_address,
            config.test_amount,
        )),
        PoolType::UniswapV3 { .. } => Some(build_approve_v3(
            config.buyer_address,
            config.denom_address,
            config.test_amount,
        )),
        _ => None,
    } {
        denom_approve_tx.gas = Some(config.approve_gas_limit);
        apply_fee_policy(&mut denom_approve_tx, &config, base_fee);
        let denom_approve_sim = chain
            .step_with_trace(denom_approve_tx.clone())
            .await
            .map_err(|err| {
                let context = format!(
                    "while executing denom approve with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                    denom_approve_tx.gas,
                    denom_approve_tx.gas_price,
                    denom_approve_tx.max_fee_per_gas,
                    denom_approve_tx.max_priority_fee_per_gas
                );
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "denom_approve",
                    %context,
                    block = block_number,
                    error = %err
                );
                err.wrap_err(context)
            })?;
        let denom_approve_processed = tx_processor
            .process_transaction_from_simulation_result(
                &denom_approve_tx,
                &denom_approve_sim,
                block_number,
                prior_tx_results.len() as u64,
            )
            .await?;
        prior_tx_results.push(denom_approve_processed);
        if !denom_approve_sim.success {
            let failure_message = format_failure_with_revert(
                "Denomination token approval failed",
                denom_approve_sim.revert_reason.as_deref(),
            );
            return Ok(create_failed_result(
                config,
                block_number,
                prior_tx_results,
                None,
                None,
                None,
                failure_message,
                false,
                false,
                false,
            ));
        }
        denom_approve_tx_for_delay = Some(denom_approve_tx);
    }

    let prior_step_count = prior_tx_results.len() as u64;

    // BUY
    let slippage_bps = (config.slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let mut buy_tx = match config.pool_type {
        PoolType::UniswapV2 => build_token_to_token_swap_v2(
            UniswapV2Router::UniswapV2,
            config.buyer_address,
            config.denom_address,
            config.token_address,
            config.test_amount,
            slippage_bps,
            deadline,
        ),
        PoolType::SushiSwap => build_token_to_token_swap_v2(
            UniswapV2Router::SushiswapV2,
            config.buyer_address,
            config.denom_address,
            config.token_address,
            config.test_amount,
            slippage_bps,
            deadline,
        ),
        PoolType::UniswapV3 { fee_tier } => build_token_to_token_swap_v3(
            config.buyer_address,
            config.denom_address,
            config.token_address,
            config.test_amount,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        other => {
            return Err(eyre::eyre!(
                "Pool type {:?} not yet implemented for denomination token swaps",
                other
            ));
        }
    };
    buy_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut buy_tx, &config, base_fee);
    let buy_sim_result = chain
        .step_with_trace(buy_tx.clone())
        .await
        .map_err(|err| {
            let context = format!(
                "while executing simulated buy with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                buy_tx.gas,
                buy_tx.gas_price,
                buy_tx.max_fee_per_gas,
                buy_tx.max_priority_fee_per_gas
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "buy",
                %context,
                block = block_number,
                error = %err
            );
            err.wrap_err(context)
        })?;
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(
            &buy_tx,
            &buy_sim_result,
            block_number,
            prior_step_count,
        )
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
    let denom_spent_u256 = if can_buy {
        let delta =
            extract_token_balance_delta(&buy_processed, config.buyer_address, config.denom_address);
        if delta < I256::ZERO {
            delta.unsigned_abs()
        } else {
            U256::ZERO
        }
    } else {
        U256::ZERO
    };
    if !can_buy {
        let failure_message = enrich_failure_reason_with_trace(
            &simulator,
            &buy_tx,
            block_number,
            "Buy transaction failed",
            buy_sim_result.revert_reason.as_deref(),
        )
        .await;

        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            None,
            None,
            failure_message,
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
    let approve_sim_result = chain
        .step_with_trace(approve_tx.clone())
        .await
        .map_err(|err| {
            let context = format!(
                "while executing simulated approve with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                approve_tx.gas,
                approve_tx.gas_price,
                approve_tx.max_fee_per_gas,
                approve_tx.max_priority_fee_per_gas
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "approve",
                %context,
                block = block_number,
                error = %err
            );
            err.wrap_err(context)
        })?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(
            &approve_tx,
            &approve_sim_result,
            block_number,
            prior_step_count + 1,
        )
        .await?;
    let can_approve = approve_sim_result.success;
    if !can_approve {
        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            Some(approve_processed),
            None,
            format_failure_with_revert(
                "Approve transaction failed",
                approve_sim_result.revert_reason.as_deref(),
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
        if let Some(denom_tx) = denom_approve_tx_for_delay.clone() {
            chain.step_with_trace(denom_tx).await.map_err(|err| {
                let context = "while reapplying denom approve before delayed sell".to_string();
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "reapply_denom_approve",
                    %context,
                    block = sell_block,
                    error = %err
                );
                err.wrap_err(context)
            })?;
        }
        chain.step_with_trace(buy_tx).await.map_err(|err| {
            let context = "while reapplying simulated buy before delayed sell".to_string();
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "reapply_buy",
                %context,
                block = sell_block,
                error = %err
            );
            err.wrap_err(context)
        })?;
        chain.step_with_trace(approve_tx).await.map_err(|err| {
            let context = "while reapplying simulated approve before delayed sell".to_string();
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "reapply_approve",
                %context,
                block = sell_block,
                error = %err
            );
            err.wrap_err(context)
        })?;
    }
    let mut sell_tx = match config.pool_type {
        PoolType::UniswapV2 => build_token_to_token_swap_v2(
            UniswapV2Router::UniswapV2,
            config.buyer_address,
            config.token_address,
            config.denom_address,
            tokens_received,
            slippage_bps,
            deadline,
        ),
        PoolType::SushiSwap => build_token_to_token_swap_v2(
            UniswapV2Router::SushiswapV2,
            config.buyer_address,
            config.token_address,
            config.denom_address,
            tokens_received,
            slippage_bps,
            deadline,
        ),
        PoolType::UniswapV3 { fee_tier } => build_token_to_token_swap_v3(
            config.buyer_address,
            config.token_address,
            config.denom_address,
            tokens_received,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        other => {
            return Err(eyre::eyre!(
                "Pool type {:?} not yet implemented for denomination token swaps",
                other
            ));
        }
    };
    sell_tx.gas = Some(config.sell_gas_limit);
    apply_fee_policy(&mut sell_tx, &config, base_fee);
    let sell_block = if config.block_delay > 0 {
        block_number + config.block_delay
    } else {
        block_number
    };
    let sell_sim_result = chain
        .step_with_trace(sell_tx.clone())
        .await
        .map_err(|err| {
            let context = format!(
                "while executing simulated sell with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                sell_tx.gas,
                sell_tx.gas_price,
                sell_tx.max_fee_per_gas,
                sell_tx.max_priority_fee_per_gas
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "sell",
                %context,
                block = sell_block,
                error = %err
            );
            err.wrap_err(context)
        })?;
    let sell_processed = tx_processor
        .process_transaction_from_simulation_result(
            &sell_tx,
            &sell_sim_result,
            sell_block,
            prior_step_count + 2,
        )
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
    let denom_received_u256 = extract_denom_received_from_processed_transaction(
        &sell_processed,
        config.buyer_address,
        config.denom_address,
    )
    .unwrap_or(U256::ZERO);

    let failure_reason = if !(can_buy && can_approve && can_sell) {
        if !can_sell {
            Some(
                enrich_failure_reason_with_trace(
                    &simulator,
                    &sell_tx,
                    sell_block,
                    "Simulation failed at step SELL",
                    sell_sim_result.revert_reason.as_deref(),
                )
                .await,
            )
        } else if !can_approve {
            let prefix = "Simulation failed at step APPROVE";
            Some(format_failure_with_revert(
                prefix,
                approve_sim_result.revert_reason.as_deref(),
            ))
        } else {
            let prefix = "Simulation failed at step BUY";
            Some(format_failure_with_revert(
                prefix,
                buy_sim_result.revert_reason.as_deref(),
            ))
        }
    } else {
        None
    };

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
        denom_spent: denom_spent_u256,
        denom_received: denom_received_u256,
        buy_transaction: buy_processed,
        sell_transaction: sell_processed,
        approve_transaction: approve_processed,
        prior_transactions: prior_tx_results,
        failure_reason,
        block_number,
    })
}

async fn validate_pool_registration(
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

fn extract_token_balance_delta(
    processed_tx: &ProcessedTransaction,
    account: Address,
    token_address: Address,
) -> I256 {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&account) {
        if let Some(symbol) = get_token_symbol(&token_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                return amount;
            }
        }
        let token_key = to_checksum_address(&token_address);
        if let Some(&amount) = balance_changes.token_net.get(&token_key) {
            return amount;
        }
    }
    I256::ZERO
}

fn extract_tokens_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    token_address: Address,
    _token_decimals: u8,
) -> U256 {
    let delta = extract_token_balance_delta(processed_tx, recipient_address, token_address);
    if delta > I256::ZERO {
        delta.unsigned_abs()
    } else {
        U256::ZERO
    }
}

fn extract_denom_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    denom_address: Address,
) -> Option<U256> {
    let delta = extract_token_balance_delta(processed_tx, recipient_address, denom_address);
    if delta > I256::ZERO {
        Some(delta.unsigned_abs())
    } else {
        None
    }
}

fn apply_fee_policy(
    tx: &mut UnsignedTransaction,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
) {
    if tx.gas.is_none() {
        tx.gas = Some(config.buy_gas_limit);
    }

    if let Some(price) = tx.gas_price {
        if price == 0 {
            tx.gas_price = config.gas_price.or(base_fee);
        }
        return;
    }

    if let Some(mut max_fee) = tx.max_fee_per_gas {
        let priority_fee = tx
            .max_priority_fee_per_gas
            .or(config.max_priority_fee_per_gas)
            .unwrap_or(0);
        if let Some(base_for_min) = base_fee.or(config.gas_price) {
            let min_required = base_for_min.saturating_add(priority_fee);
            if max_fee < min_required {
                max_fee = min_required;
            }
        }
        tx.max_priority_fee_per_gas = Some(priority_fee);
        tx.max_fee_per_gas = Some(max_fee);
        tx.gas_price = None;
        return;
    }

    if let Some(price) = config.gas_price {
        tx.gas_price = Some(price);
        tx.max_fee_per_gas = None;
        tx.max_priority_fee_per_gas = None;
        return;
    }

    if let Some(base_for_min) = base_fee {
        let priority_fee = tx
            .max_priority_fee_per_gas
            .or(config.max_priority_fee_per_gas)
            .unwrap_or(0);

        let mut max_fee = config
            .max_fee_per_gas
            .unwrap_or(base_for_min.saturating_add(priority_fee));
        let min_required = base_for_min.saturating_add(priority_fee);
        if max_fee < min_required {
            max_fee = min_required;
        }

        tx.gas_price = None;
        tx.max_priority_fee_per_gas = Some(priority_fee);
        tx.max_fee_per_gas = Some(max_fee);
    }
}

fn format_failure_with_revert(prefix: &str, revert: Option<&str>) -> String {
    let reason = revert.map(|s| s.trim()).filter(|s| !s.is_empty());
    match reason {
        Some(reason) => format!("{prefix} (revert: {reason})"),
        None => format!("{prefix} (revert reason unknown)"),
    }
}

async fn enrich_failure_reason_with_trace(
    simulator: &Arc<TxSimulator>,
    tx: &UnsignedTransaction,
    block_number: u64,
    base_message: &str,
    revert_reason: Option<&str>,
) -> String {
    let mut reason_opt = revert_reason.map(str::to_string);
    let needs_trace = reason_opt
        .as_deref()
        .map(|s| {
            let trimmed = s.trim();
            trimmed.is_empty()
                || trimmed.contains("without returning data")
                || trimmed.contains("UniswapV2:")
                || trimmed.eq_ignore_ascii_case("execution reverted")
        })
        .unwrap_or(true);

    if needs_trace {
        match simulator
            .simulate_unsigned_transaction_with_full_trace_at_block(tx.clone(), block_number)
            .await
        {
            Ok(full) => {
                if let Some(reason) = full.revert_reason.as_ref() {
                    if !reason.is_empty() {
                        reason_opt = Some(reason.clone());
                    }
                }

                if reason_opt.is_none() {
                    if let Some(reason) = extract_reason_from_call_trace(&full.call_trace) {
                        reason_opt = Some(reason);
                    }
                }

                if reason_opt.is_none() {
                    if let Some(ctx) = full.revert_context.as_ref() {
                        reason_opt = Some(format!(
                            "execution reverted at {} (calldata {} bytes)",
                            ctx.target, ctx.calldata_len
                        ));
                    }
                }
            }
            Err(err) => {
                let base = format_failure_with_revert(base_message, revert_reason);
                return format!("{base} (trace failed: {err})");
            }
        }
    }

    format_failure_with_revert(base_message, reason_opt.as_deref())
}

fn extract_reason_from_call_trace(frame: &CallFrame) -> Option<String> {
    if let Some(output) = &frame.output {
        if let Some(reason) = decode_revert_output(output.as_ref()) {
            if !reason.is_empty() {
                return Some(reason);
            }
        }
    }

    let mut fallback: Option<String> = None;
    if let Some(error) = &frame.error {
        let trimmed = error.trim();
        if !trimmed.is_empty() && trimmed != "execution reverted" {
            if trimmed.contains("UniswapV2:") || trimmed.eq_ignore_ascii_case("execution reverted")
            {
                fallback = Some(trimmed.to_string());
            } else {
                return Some(trimmed.to_string());
            }
        }
    }

    for child in &frame.calls {
        if let Some(reason) = extract_reason_from_call_trace(child) {
            return Some(reason);
        }
    }

    fallback.or_else(|| {
        frame.error.as_ref().and_then(|err| {
            let trimmed = err.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    })
}

fn decode_revert_output(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }

    let selector = &data[..4];
    if selector == [0x08, 0xc3, 0x79, 0xa0] {
        if data.len() < 68 {
            return None;
        }
        let len_bytes: [u8; 8] = data[60..68].try_into().ok()?;
        let str_len = u64::from_be_bytes(len_bytes) as usize;
        let start = 68;
        if data.len() < start + str_len {
            return None;
        }
        let string_bytes = &data[start..start + str_len];
        return Some(String::from_utf8_lossy(string_bytes).to_string());
    }

    if selector == [0x4e, 0x48, 0x7b, 0x71] {
        if data.len() < 36 {
            return Some("panic (no code)".to_string());
        }
        let code_bytes: [u8; 8] = data[28..36].try_into().ok()?;
        let code = u64::from_be_bytes(code_bytes);
        let description = match code {
            0x01 => "panic: assertion failed",
            0x11 => "panic: arithmetic overflow/underflow",
            0x12 => "panic: division by zero",
            0x21 => "panic: invalid enum value",
            0x31 => "panic: storage byte array out-of-bounds",
            0x32 => "panic: array out-of-bounds",
            0x41 => "panic: memory overflow",
            0x51 => "panic: pop from empty array",
            other => return Some(format!("panic code 0x{other:x}")),
        };
        return Some(description.to_string());
    }

    None
}

fn override_prior_gas_price_with_header(
    base_fee: Option<u128>,
    prior_tx: &ProcessedTransaction,
    unsigned_tx: &mut UnsignedTransaction,
) {
    if !matches!(prior_tx.raw_tx_type, 0 | 1) {
        return;
    }
    if let Some(base_fee) = base_fee {
        unsigned_tx.gas_price = Some(base_fee);
        // legacy transactions should not have EIP-1559 fields
        unsigned_tx.max_fee_per_gas = None;
        unsigned_tx.max_priority_fee_per_gas = None;
    }
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

    let buy_orientation = infer_v4_orientation_from_output(&pool_key, config.token_address)?;

    if buy_orientation.input_currency != Address::ZERO
        && buy_orientation.input_currency != config.weth_address
    {
        return Err(eyre!(
            "Uniswap V4 helper currently supports pools where the input currency is WETH or native ETH"
        ));
    }

    let router_address = compute_v4_contract_address(config.buyer_address, deployer_nonce);
    let mut prior_tx_results: Vec<ProcessedTransaction> =
        Vec::with_capacity(config.prior_txs.len() + 4);

    for (idx, prior_tx) in config.prior_txs.iter().enumerate() {
        let prior_tx_gas_limit = if prior_tx.fees.gas_limit > 0 {
            Some(prior_tx.fees.gas_limit)
        } else if prior_tx.fees.gas_used > 0 {
            Some(prior_tx.fees.gas_used)
        } else {
            None
        };

        let prior_max_fee = prior_tx
            .fees
            .max_fee_per_gas
            .and_then(|v| u128::try_from(v).ok())
            .or(config.max_fee_per_gas);
        let prior_max_priority = prior_tx
            .fees
            .max_priority_fee
            .and_then(|v| u128::try_from(v).ok())
            .or(config.max_priority_fee_per_gas);

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
            gas: prior_tx_gas_limit,
            gas_price: prior_gas_price,
            max_fee_per_gas: prior_max_fee,
            max_priority_fee_per_gas: prior_max_priority,
            nonce: None,
        };
        override_prior_gas_price_with_header(base_fee, prior_tx, &mut setup_call);
        let has_explicit_fee = setup_call.gas_price.is_some()
            || setup_call.max_fee_per_gas.is_some()
            || setup_call.max_priority_fee_per_gas.is_some();
        if !has_explicit_fee {
            apply_fee_policy(&mut setup_call, &config, base_fee);
        } else if setup_call.gas.is_none() && prior_tx_gas_limit.is_some() {
            setup_call.gas = prior_tx_gas_limit;
        }

        let prior_hash = format!("{:#x}", prior_tx.hash);
        let prior_nonce = prior_tx.nonce;
        let setup_result = chain.step_with_trace(setup_call.clone()).await.map_err(|err| {
            let context = format!(
                "while replaying prior tx {prior_hash} (index {idx}, nonce {prior_nonce}) with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                setup_call.gas,
                setup_call.gas_price,
                setup_call.max_fee_per_gas,
                setup_call.max_priority_fee_per_gas
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "v4_prior_replay",
                %context,
                block = block_number,
                error = %err
            );
            eyre!("{}: {}", context, err)
        })?;
        let processed = tx_processor
            .process_transaction_from_simulation_result(
                &setup_call,
                &setup_result,
                block_number,
                idx as u64,
            )
            .await?;
        prior_tx_results.push(processed);
    }
    let router_exists_on_chain = provider
        .basic_account(&router_address)?
        .map(|acc| acc.has_bytecode())
        .unwrap_or(false);
    let router_exists_in_chain = chain.account_has_code(router_address)?;

    if !router_exists_on_chain && !router_exists_in_chain {
        let mut deploy_tx =
            build_baygus_router_deploy_tx(config.buyer_address, v4_cfg.pool_manager);
        apply_fee_policy(&mut deploy_tx, &config, base_fee);
        let deploy_result = chain
            .step_with_trace(deploy_tx.clone())
            .await
            .map_err(|err| {
                let context = format!(
                    "while deploying temporary router with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                    deploy_tx.gas,
                    deploy_tx.gas_price,
                    deploy_tx.max_fee_per_gas,
                    deploy_tx.max_priority_fee_per_gas
                );
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "v4_deploy",
                    %context,
                    block = block_number,
                    error = %err
                );
                err.wrap_err(context)
            })?;
        let deploy_processed = tx_processor
            .process_transaction_from_simulation_result(
                &deploy_tx,
                &deploy_result,
                block_number,
                prior_tx_results.len() as u64,
            )
            .await?;
        prior_tx_results.push(deploy_processed.clone());
        if !deploy_result.success {
            return Ok(create_failed_result(
                config,
                block_number,
                prior_tx_results,
                None,
                None,
                None,
                format_failure_with_revert(
                    "Baygus router deployment failed",
                    deploy_result.revert_reason.as_deref(),
                ),
                false,
                false,
                false,
            ));
        }
        if !chain.account_has_code(router_address)? {
            return Ok(create_failed_result(
                config,
                block_number,
                prior_tx_results,
                None,
                None,
                None,
                "Baygus router deployment succeeded but bytecode not visible in simulation state"
                    .to_string(),
                false,
                false,
                false,
            ));
        }
    }

    if buy_orientation.input_currency == config.weth_address {
        let mut deposit_tx = build_v4_weth_deposit_tx(
            config.buyer_address,
            config.weth_address,
            config.test_amount,
        );
        deposit_tx.gas = Some(config.buy_gas_limit);
        apply_fee_policy(&mut deposit_tx, &config, base_fee);
        let deposit_result = chain
            .step_with_trace(deposit_tx.clone())
            .await
            .map_err(|err| {
                let context = format!(
                    "while simulating WETH deposit with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                    deposit_tx.gas,
                    deposit_tx.gas_price,
                    deposit_tx.max_fee_per_gas,
                    deposit_tx.max_priority_fee_per_gas
                );
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "v4_deposit",
                    %context,
                    block = block_number,
                    error = %err
                );
                err.wrap_err(context)
            })?;
        let deposit_processed = tx_processor
            .process_transaction_from_simulation_result(
                &deposit_tx,
                &deposit_result,
                block_number,
                prior_tx_results.len() as u64,
            )
            .await?;
        prior_tx_results.push(deposit_processed.clone());
        if !deposit_result.success {
            return Ok(create_failed_result(
                config,
                block_number,
                prior_tx_results,
                None,
                None,
                None,
                format_failure_with_revert(
                    "WETH deposit failed",
                    deposit_result.revert_reason.as_deref(),
                ),
                false,
                false,
                false,
            ));
        }

        let mut weth_approve_tx = build_v4_token_approval_tx(
            config.buyer_address,
            config.weth_address,
            router_address,
            config.test_amount,
        );
        weth_approve_tx.gas = Some(config.approve_gas_limit);
        apply_fee_policy(&mut weth_approve_tx, &config, base_fee);
        let weth_approve_result = chain
            .step_with_trace(weth_approve_tx.clone())
            .await
            .map_err(|err| {
                let context = format!(
                    "while simulating WETH approval with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                    weth_approve_tx.gas,
                    weth_approve_tx.gas_price,
                    weth_approve_tx.max_fee_per_gas,
                    weth_approve_tx.max_priority_fee_per_gas
                );
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "v4_weth_approve",
                    %context,
                    block = block_number,
                    error = %err
                );
                err.wrap_err(context)
            })?;
        let weth_approve_processed = tx_processor
            .process_transaction_from_simulation_result(
                &weth_approve_tx,
                &weth_approve_result,
                block_number,
                prior_tx_results.len() as u64,
            )
            .await?;
        prior_tx_results.push(weth_approve_processed.clone());
        if !weth_approve_result.success {
            return Ok(create_failed_result(
                config,
                block_number,
                prior_tx_results,
                None,
                None,
                None,
                format_failure_with_revert(
                    "WETH approval for Baygus router failed",
                    weth_approve_result.revert_reason.as_deref(),
                ),
                false,
                false,
                false,
            ));
        }
    }

    let buy_request = BuilderV4SingleHopRequest {
        pool_key: pool_key.clone(),
        token_in: buy_orientation.input_currency,
        token_out: buy_orientation.output_currency,
        amount_in: config.test_amount,
        recipient: config.buyer_address,
        min_output: None,
        hook_adapter: Address::ZERO,
        hook_data: v4_cfg.hook_data.clone(),
        sqrt_price_limit_x96: None,
    };
    let buy_call = build_baygus_single_hop_exact_input_call(&buy_request)?;
    let mut buy_tx = build_baygus_router_multihop_tx(
        router_address,
        config.buyer_address,
        &buy_call.params,
        buy_call.eth_value,
    )?;
    buy_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut buy_tx, &config, base_fee);
    let prior_step_count = prior_tx_results.len() as u64;
    let buy_result = chain
        .step_with_trace(buy_tx.clone())
        .await
        .map_err(|err| {
            let context = format!(
                "while executing Uniswap V4 buy via Baygus router with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                buy_tx.gas,
                buy_tx.gas_price,
                buy_tx.max_fee_per_gas,
                buy_tx.max_priority_fee_per_gas
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "v4_buy",
                %context,
                block = block_number,
                error = %err
            );
            err.wrap_err(context)
        })?;
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(
            &buy_tx,
            &buy_result,
            block_number,
            prior_step_count,
        )
        .await?;
    if !buy_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            None,
            None,
            format_failure_with_revert(
                "Baygus router buy transaction failed",
                buy_result.revert_reason.as_deref(),
            ),
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
    let approve_result = chain
        .step_with_trace(approve_tx.clone())
        .await
        .map_err(|err| {
            let context = format!(
                "while executing Uniswap V4 token approval with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                approve_tx.gas,
                approve_tx.gas_price,
                approve_tx.max_fee_per_gas,
                approve_tx.max_priority_fee_per_gas
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "v4_approve",
                %context,
                block = block_number,
                error = %err
            );
            err.wrap_err(context)
        })?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(
            &approve_tx,
            &approve_result,
            block_number,
            prior_step_count + 1,
        )
        .await?;
    if !approve_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            Some(approve_processed),
            None,
            format_failure_with_revert(
                "Token approval for Baygus router failed",
                approve_result.revert_reason.as_deref(),
            ),
            true,
            false,
            false,
        ));
    }

    let sell_orientation = infer_v4_orientation_from_input(&pool_key, config.token_address)?;
    let sell_request = BuilderV4SingleHopRequest {
        pool_key: pool_key.clone(),
        token_in: config.token_address,
        token_out: sell_orientation.output_currency,
        amount_in: tokens_received,
        recipient: config.buyer_address,
        min_output: None,
        hook_adapter: Address::ZERO,
        hook_data: v4_cfg.hook_data.clone(),
        sqrt_price_limit_x96: None,
    };
    let sell_call = build_baygus_single_hop_exact_input_call(&sell_request)?;

    let mut sell_tx = build_baygus_router_multihop_tx(
        router_address,
        config.buyer_address,
        &sell_call.params,
        sell_call.eth_value,
    )?;
    sell_tx.gas = Some(config.sell_gas_limit);
    apply_fee_policy(&mut sell_tx, &config, base_fee);
    let sell_result = chain
        .step_with_trace(sell_tx.clone())
        .await
        .map_err(|err| {
            let context = format!(
                "while executing Uniswap V4 sell via Baygus router with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                sell_tx.gas,
                sell_tx.gas_price,
                sell_tx.max_fee_per_gas,
                sell_tx.max_priority_fee_per_gas
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "v4_sell",
                %context,
                block = block_number,
                error = %err
            );
            err.wrap_err(context)
        })?;
    let sell_processed = tx_processor
        .process_transaction_from_simulation_result(
            &sell_tx,
            &sell_result,
            block_number,
            prior_step_count + 2,
        )
        .await?;
    if !sell_result.success {
        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            Some(approve_processed),
            Some(sell_processed),
            format_failure_with_revert(
                "Baygus router sell transaction failed",
                sell_result.revert_reason.as_deref(),
            ),
            true,
            true,
            false,
        ));
    }

    let mut unwrap_tx_processed: Option<ProcessedTransaction> = None;
    if sell_call.orientation.output_currency == config.weth_address {
        let wdenom_received = extract_tokens_received_from_processed_transaction(
            &sell_processed,
            config.buyer_address,
            config.weth_address,
            WETH_DECIMALS,
        );
        if wdenom_received > U256::ZERO {
            let mut withdraw_tx = build_v4_weth_withdraw_tx(
                config.buyer_address,
                config.weth_address,
                wdenom_received,
            );
            withdraw_tx.gas = Some(config.sell_gas_limit);
            apply_fee_policy(&mut withdraw_tx, &config, base_fee);
            let withdraw_result = chain
                .step_with_trace(withdraw_tx.clone())
                .await
                .map_err(|err| {
                    let context = format!(
                        "while executing WETH unwrap with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                        withdraw_tx.gas,
                        withdraw_tx.gas_price,
                        withdraw_tx.max_fee_per_gas,
                        withdraw_tx.max_priority_fee_per_gas
                    );
                    tracing::warn!(
                        target: "pool_buy_sell_sim",
                        step = "v4_unwrap",
                        %context,
                        block = block_number,
                        error = %err
                    );
                    err.wrap_err(context)
                })?;
            let withdraw_processed = tx_processor
                .process_transaction_from_simulation_result(
                    &withdraw_tx,
                    &withdraw_result,
                    block_number,
                    prior_step_count + 3,
                )
                .await?;
            prior_tx_results.push(withdraw_processed.clone());
            if !withdraw_result.success {
                return Ok(create_failed_result(
                    config,
                    block_number,
                    prior_tx_results,
                    Some(buy_processed),
                    Some(approve_processed),
                    Some(withdraw_processed),
                    format_failure_with_revert(
                        "WETH unwrap transaction failed",
                        withdraw_result.revert_reason.as_deref(),
                    ),
                    true,
                    true,
                    false,
                ));
            }
            unwrap_tx_processed = Some(withdraw_processed);
        }
    }

    let final_sell_processed = unwrap_tx_processed
        .clone()
        .unwrap_or_else(|| sell_processed.clone());

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

    let eth_transfer_source = unwrap_tx_processed.as_ref().unwrap_or(&sell_processed);
    let denom_received_u256 = extract_denom_received_from_processed_transaction(
        eth_transfer_source,
        config.buyer_address,
        config.denom_address,
    )
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
        denom_spent: config.test_amount,
        denom_received: denom_received_u256,
        buy_transaction: buy_processed,
        sell_transaction: final_sell_processed,
        approve_transaction: approve_processed,
        prior_transactions: prior_tx_results,
        failure_reason: None,
        block_number,
    })
}

fn create_failed_result(
    config: PoolBuySellParameters,
    block_number: u64,
    prior_txs: Vec<ProcessedTransaction>,
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
        status: false,
        nonce: 0,
        raw_tx_type: 0,
        tx_type: "UNKNOWN".to_string(),
        actions: vec![],
        fees: Default::default(),
        bribe_amount: U256::ZERO,
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
        erc20_approval_events: vec![],
        erc721_approval_events: vec![],
        uniswap_v2_mints: vec![],
        uniswap_v2_burns: vec![],
        deposit_events: vec![],
        withdraw_events: vec![],
        uniswap_v2_pair_created_events: vec![],
        ownership_transferred_events: vec![],
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
        denom_spent: config.test_amount,
        denom_received: U256::ZERO,
        buy_transaction: buy_tx.unwrap_or_else(|| dummy_tx.clone()),
        sell_transaction: sell_tx.unwrap_or_else(|| dummy_tx.clone()),
        approve_transaction: approve_tx.unwrap_or_else(|| dummy_tx.clone()),
        prior_transactions: prior_txs,
        failure_reason: Some(failure_reason),
        block_number,
    }
}
