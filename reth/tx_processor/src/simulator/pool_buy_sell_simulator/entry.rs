use crate::tx_processor::TxProcessor;
use alloy_primitives::{Address, I256, U256};
use eyre::{eyre, Result, WrapErr};
use reth_primitives::{Header as _, SealedHeader};
use reth_provider::{AccountReader, HeaderProvider};
use std::sync::Arc;
use tx_simulator::{TxSimulator, UnsignedTransaction};

use super::balance_deltas::{
    extract_denom_received_from_processed_transaction, extract_token_balance_delta,
    extract_tokens_received_from_processed_transaction,
};
use super::buyer_setup::prepare_buyer_account;
use super::failure::{enrich_failure_reason_with_trace, format_failure_with_revert};
use super::fees::{apply_fee_policy, override_prior_gas_price_with_header};
use super::results::create_failed_result;
use super::uniswap_v4::check_can_buy_sell_uniswap_v4;
use super::validation::validate_pool_registration;
use crate::simulator::types::{PoolBuySellParameters, PoolBuySellSimulationResult, PoolType};
use crate::simulator::unsigned_tx_builder::UnsignedTxBuilder;
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction,
};
use reth_chain_query::dex::{
    fetch_uniswap_v2_pair_address, fetch_uniswap_v3_pool_address, SUSHISWAP_FACTORY,
    UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY,
};
use reth_chain_query::tx_builders::amm::uniswap_v2::{
    build_approve_v2, build_token_to_token_swap_v2, Router as UniswapV2Router,
};
use reth_chain_query::tx_builders::amm::uniswap_v3::{
    build_approve_v3, build_token_to_token_swap_v3,
};
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
        setup_call.nonce = Some(prior_tx.nonce);
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

    if config.prior_txs.is_empty() {
        validate_pool_registration(simulator.clone(), &config, block_number).await?;
    } else {
        let pool_has_code = chain
            .account_has_code(config.pool_address)
            .wrap_err_with(|| {
                format!(
                    "Failed to inspect pool {:#x} after prior transaction replay",
                    config.pool_address
                )
            })?;
        if !pool_has_code {
            return Err(eyre!(
                "Pool contract {:#x} not present in simulated state after applying prior transactions at block {}",
                config.pool_address,
                block_number
            ));
        }
    }

    if let Some(failure) = prepare_buyer_account(
        &mut chain,
        &config,
        base_fee,
        tx_processor.clone(),
        block_number,
        &mut prior_tx_results,
    )
    .await?
    {
        return Ok(failure);
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
