use std::sync::Arc;

use alloy_eips::eip2930::AccessListItem;
use alloy_primitives::{Address, U256};
use eyre::{eyre, Result};
use reth_provider::AccountReader;
use tx_simulator::{TxSimulator, UnsignedTransaction};

use super::balance_deltas::{
    extract_denom_received_from_processed_transaction,
    extract_tokens_received_from_processed_transaction,
};
use super::failure::{enrich_failure_reason_with_trace, format_failure_with_revert};
use super::fees::{apply_fee_policy, normalize_prior_fees_with_header};
use super::replay_funding::ensure_replay_sender_can_pay;
use super::results::create_failed_result;
use super::WETH_DECIMALS;
use crate::simulator::types::{PoolBuySellParameters, PoolBuySellSimulationResult, PoolType};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction,
};
use crate::tx_processor::TxProcessor;
use tx_simulator::tx_builders::uniswap_v4::{
    build_baygus_executor_deploy_tx, build_baygus_executor_multihop_tx,
    build_baygus_executor_single_hop_exact_input_call,
    build_token_approval_tx as build_v4_token_approval_tx,
    build_weth_deposit_tx as build_v4_weth_deposit_tx,
    build_weth_withdraw_tx as build_v4_weth_withdraw_tx,
    compute_contract_address as compute_v4_contract_address,
    infer_orientation_from_input as infer_v4_orientation_from_input,
    infer_orientation_from_output as infer_v4_orientation_from_output,
    UniswapV4BaygusSingleHopRequest as BuilderV4SingleHopRequest,
    UniswapV4PoolKey as BuilderV4PoolKey,
};
pub(super) async fn check_can_buy_sell_uniswap_v4(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
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

    let header = simulator
        .block_context_loader()
        .load_block_header(block_number, None)
        .await?;
    let base_fee = header.header().base_fee_per_gas.map(|fee| fee as u128);
    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;

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

        let access_list: Vec<AccessListItem> = prior_tx
            .access_list
            .iter()
            .map(|item| AccessListItem {
                address: item.address,
                storage_keys: item.storage_keys.clone(),
            })
            .collect();
        let max_fee_per_blob_gas = prior_tx
            .fees
            .max_fee_per_blob_gas
            .and_then(|v| u128::try_from(v).ok());

        let mut setup_call = UnsignedTransaction {
            from: Some(prior_tx.from_address),
            to: prior_tx.to_address,
            value: Some(prior_tx.value),
            data: Some(prior_tx.input.clone().into()),
            gas: prior_tx_gas_limit,
            gas_price: prior_gas_price,
            max_fee_per_gas: prior_max_fee,
            max_priority_fee_per_gas: prior_max_priority,
            nonce: Some(prior_tx.nonce),
            access_list,
            blob_versioned_hashes: prior_tx.blob_versioned_hashes.clone(),
            max_fee_per_blob_gas,
            signed_authorizations: prior_tx.signed_authorizations.clone(),
        };
        normalize_prior_fees_with_header(base_fee, prior_tx, &mut setup_call);
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
        let previous_nonce =
            chain.set_account_nonce_for_replay(prior_tx.from_address, prior_nonce)?;
        if previous_nonce != prior_nonce {
            tracing::debug!(
                target: "pool_buy_sell_sim",
                step = "v4_prior_replay_nonce_normalization",
                tx_hash = %prior_hash,
                sender = %prior_tx.from_address,
                previous_nonce,
                replay_nonce = prior_nonce,
                "normalizing sender nonce for selected prior transaction replay"
            );
        }
        if let Some(adjustment) = ensure_replay_sender_can_pay(&mut chain, &setup_call)? {
            tracing::debug!(
                target: "pool_buy_sell_sim",
                step = "v4_prior_replay_sender_funding",
                tx_hash = %prior_hash,
                sender = %adjustment.sender,
                previous_balance = %adjustment.previous_balance,
                replay_balance = %adjustment.replay_balance,
                "funding selected prior transaction sender for replay validation"
            );
        }
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
            build_baygus_executor_deploy_tx(config.buyer_address, v4_cfg.pool_manager)?;
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
                    "Baygus executor deployment failed",
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
                "Baygus executor deployment succeeded but bytecode not visible in simulation state"
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
                    "WETH approval for Baygus executor failed",
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
    let buy_call = build_baygus_executor_single_hop_exact_input_call(&buy_request)?;
    let mut buy_tx = build_baygus_executor_multihop_tx(
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
                "while executing Uniswap V4 buy via Baygus executor with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
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
        let failure_message = enrich_failure_reason_with_trace(
            &simulator,
            &buy_tx,
            block_number,
            "Baygus executor buy transaction failed",
            buy_result.revert_reason.as_deref(),
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
                "Token approval for Baygus executor failed",
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
    let sell_call = build_baygus_executor_single_hop_exact_input_call(&sell_request)?;

    let mut sell_tx = build_baygus_executor_multihop_tx(
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
                "while executing Uniswap V4 sell via Baygus executor with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
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
        let failure_message = enrich_failure_reason_with_trace(
            &simulator,
            &sell_tx,
            block_number,
            "Baygus executor sell transaction failed",
            sell_result.revert_reason.as_deref(),
        )
        .await;

        return Ok(create_failed_result(
            config,
            block_number,
            prior_tx_results,
            Some(buy_processed),
            Some(approve_processed),
            Some(sell_processed),
            failure_message,
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
