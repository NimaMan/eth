use crate::tx_processor::TxProcessor;
use alloy_consensus::{Header as AlloyHeader, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};
use alloy_primitives::{Address, Bloom, Bytes, B256, B64, I256, U256};
use eyre::{eyre, Result, WrapErr};
use reth_chain_query::provider::BlockHeader;
use reth_primitives_traits::SealedHeader;
use std::collections::HashMap;
use std::sync::Arc;
use tx_simulator::{TxSimulator, UnsignedTransaction};

use super::balance_deltas::{
    extract_denom_received_from_processed_transaction, extract_token_balance_delta,
    extract_tokens_received_from_processed_transaction,
};
use super::buyer_setup::prepare_buyer_account;
use super::failure::{
    enrich_failure_reason_with_trace, format_failure_with_full_trace, format_failure_with_revert,
};
use super::fees::{apply_fee_policy, normalize_prior_fees_with_header};
use super::replay_funding::ensure_replay_sender_can_pay;
use super::results::create_failed_result;
use super::uniswap_v4::check_can_buy_sell_uniswap_v4;
use super::validation::validate_pool_registration;
use crate::simulator::types::{PoolBuySellParameters, PoolBuySellSimulationResult, PoolType};
use crate::tx_builder::UnsignedTxBuilder;
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction,
};
use tx_simulator::tx_builders::{
    amm_swap_route::AmmSwapRoute,
    build_approve_for_route, build_denom_to_token_swap, build_token_to_denom_swap,
    uniswap_v2::{build_approve_v2, Router as UniswapV2Router},
    uniswap_v3::build_approve_v3,
};
pub async fn check_can_buy_sell_pool(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
) -> Result<PoolBuySellSimulationResult> {
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
        None => simulator.get_latest_block()?,
    };

    let route = match config.pool_type {
        PoolType::UniswapV2 => AmmSwapRoute::UniswapV2 {
            pool: config.pool_address,
        },
        PoolType::SushiSwap => AmmSwapRoute::SushiswapV2 {
            pool: config.pool_address,
        },
        PoolType::UniswapV3 { fee_tier } => AmmSwapRoute::UniswapV3 {
            pool: config.pool_address,
            fee_tier,
        },
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
            ));
        }
    };

    let header_hint = block_header_hint(&config, block_number)?;
    let header = match header_hint.clone() {
        Some(header) => header,
        None => {
            simulator
                .block_context_loader()
                .load_block_header(block_number, None)
                .await?
        }
    };
    let base_fee = header.header().base_fee_per_gas.map(|fee| fee as u128);
    let mut chain = match header_hint {
        Some(header) => {
            simulator
                .start_simulation_chain_with_header(block_number, header)
                .await?
        }
        None => simulator.start_simulation_chain(Some(block_number)).await?,
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
        normalize_prior_fees_with_header(base_fee, prior_tx, &mut setup_call);

        let has_explicit_fee = setup_call.gas_price.is_some()
            || setup_call.max_fee_per_gas.is_some()
            || setup_call.max_priority_fee_per_gas.is_some();
        if !has_explicit_fee {
            apply_fee_policy(&mut setup_call, &config, base_fee);
        }

        let prior_hash = format!("{:#x}", prior_tx.hash);
        let prior_nonce = prior_tx.nonce;
        let previous_nonce =
            chain.set_account_nonce_for_replay(prior_tx.from_address, prior_nonce)?;
        if previous_nonce != prior_nonce {
            tracing::debug!(
                target: "pool_buy_sell_sim",
                step = "prior_replay_nonce_normalization",
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
                step = "prior_replay_sender_funding",
                tx_hash = %prior_hash,
                sender = %adjustment.sender,
                previous_balance = %adjustment.previous_balance,
                replay_balance = %adjustment.replay_balance,
                "funding selected prior transaction sender for replay validation"
            );
        }
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
            let prior_to = prior_tx
                .to_address
                .map(|address| format!("{address:#x}"))
                .unwrap_or_else(|| "contract creation".to_string());
            let base_message = format!(
                "Setup transaction replay failed tx={} mined_block={} tx_index={} nonce={} from={:#x} to={} mined_status={} simulation_base_block={}",
                prior_hash,
                prior_tx.block_number,
                prior_tx.tx_index,
                prior_nonce,
                prior_tx.from_address,
                prior_to,
                prior_tx.status,
                block_number
            );
            let failure_message = format!(
                "{}{}",
                format_failure_with_full_trace(&base_message, &setup_sim_result),
                if prior_tx.status {
                    "; mined receipt succeeded, so this is a setup replay mismatch rather than an on-chain transaction failure"
                } else {
                    ""
                }
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "setup_replay_failed",
                tx_hash = %prior_hash,
                mined_block = prior_tx.block_number,
                tx_index = prior_tx.tx_index,
                nonce = prior_nonce,
                from = %prior_tx.from_address,
                to = %prior_to,
                mined_status = prior_tx.status,
                simulation_base_block = block_number,
                failure = %failure_message
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

    if !config.prior_txs.is_empty() {
        let mut next_nonces: HashMap<Address, u64> = HashMap::new();
        for prior_tx in &config.prior_txs {
            let next_nonce = prior_tx.nonce.saturating_add(1);
            next_nonces
                .entry(prior_tx.from_address)
                .and_modify(|tracked| {
                    if next_nonce > *tracked {
                        *tracked = next_nonce;
                    }
                })
                .or_insert(next_nonce);
        }
        for (address, next_nonce) in next_nonces {
            chain.override_account_nonce(address, next_nonce);
        }
    }

    if config.prior_txs.is_empty() {
        validate_pool_registration(&mut chain, &config, block_number)?;
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
        simulator.clone(),
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
    let mut buy_tx = build_denom_to_token_swap(
        &route,
        config.buyer_address,
        config.denom_address,
        config.token_address,
        config.test_amount,
        slippage_bps,
        deadline,
    );
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
    if tokens_received == U256::ZERO {
        let failure_message = if denom_spent_u256 > U256::ZERO {
            format!(
                "Buy transaction succeeded but buyer received zero tokens (denom_spent={denom_spent_u256})"
            )
        } else {
            "Buy transaction succeeded but buyer received zero tokens and spent no denomination"
                .to_string()
        };
        tracing::warn!(
            target: "pool_buy_sell_sim",
            step = "buy",
            block = block_number,
            token_address = %config.token_address,
            pool_address = %config.pool_address,
            denom_spent = %denom_spent_u256,
            "simulated buy produced zero output tokens"
        );
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
    let mut approve_tx = build_approve_for_route(
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
        let latest = simulator
            .live_latest_block_number()
            .await?
            .unwrap_or(simulator.get_latest_block()?);
        let latest = latest.max(block_number);
        let sell_block = if requested_sell_block > latest {
            latest
        } else {
            requested_sell_block
        };
        chain = simulator.start_simulation_chain(Some(sell_block)).await?;
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
    let mut sell_tx = build_token_to_denom_swap(
        &route,
        config.buyer_address,
        config.token_address,
        config.denom_address,
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

pub(super) fn block_header_hint(
    config: &PoolBuySellParameters,
    block_number: u64,
) -> Result<Option<SealedHeader>> {
    let Some(header) = &config.block_header else {
        return Ok(None);
    };
    if header.number != block_number {
        return Err(eyre!(
            "block header hint mismatch: header={}, requested={}",
            header.number,
            block_number
        ));
    }
    Ok(Some(sealed_header_from_block_header(header)))
}

fn sealed_header_from_block_header(header: &BlockHeader) -> SealedHeader {
    let sparse_header = AlloyHeader {
        parent_hash: header.parent_hash,
        ommers_hash: EMPTY_OMMER_ROOT_HASH,
        beneficiary: Address::ZERO,
        state_root: EMPTY_ROOT_HASH,
        transactions_root: EMPTY_ROOT_HASH,
        receipts_root: EMPTY_ROOT_HASH,
        logs_bloom: Bloom::ZERO,
        difficulty: U256::ZERO,
        number: header.number,
        gas_limit: header.gas_limit,
        gas_used: header.gas_used,
        timestamp: header.timestamp,
        extra_data: Bytes::default(),
        mix_hash: B256::ZERO,
        nonce: B64::ZERO,
        base_fee_per_gas: header.base_fee_per_gas,
        withdrawals_root: header.withdrawals_root,
        blob_gas_used: header.blob_gas_used,
        excess_blob_gas: header.excess_blob_gas,
        parent_beacon_block_root: header.parent_beacon_block_root,
        requests_hash: header.requests_hash,
        block_access_list_hash: header.block_access_list_hash,
        slot_number: header.slot_number,
    };
    SealedHeader::new(sparse_header, header.hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processed_block_header(number: u64) -> BlockHeader {
        BlockHeader {
            number,
            hash: B256::from([1_u8; 32]),
            parent_hash: B256::from([2_u8; 32]),
            timestamp: 1_777_000_000,
            gas_limit: 60_000_000,
            gas_used: 21_000_000,
            base_fee_per_gas: Some(123_456_789),
            withdrawals_root: Some(B256::from([3_u8; 32])),
            blob_gas_used: Some(393_216),
            excess_blob_gas: Some(1_179_648),
            parent_beacon_block_root: Some(B256::from([4_u8; 32])),
            requests_hash: Some(B256::from([5_u8; 32])),
            block_access_list_hash: Some(B256::from([6_u8; 32])),
            slot_number: Some(42),
        }
    }

    #[test]
    fn block_header_hint_uses_processed_block_header() {
        let config =
            PoolBuySellParameters::default().with_block_header(processed_block_header(100));

        let header = block_header_hint(&config, 100)
            .expect("header hint should parse")
            .expect("header should be present");

        assert_eq!(header.number, 100);
        assert_eq!(header.hash(), B256::from([1_u8; 32]));
        assert_eq!(header.parent_hash, B256::from([2_u8; 32]));
        assert_eq!(header.timestamp, 1_777_000_000);
        assert_eq!(header.base_fee_per_gas, Some(123_456_789));
        assert_eq!(header.gas_limit, 60_000_000);
        assert_eq!(header.gas_used, 21_000_000);
    }

    #[test]
    fn block_header_hint_rejects_wrong_block() {
        let config =
            PoolBuySellParameters::default().with_block_header(processed_block_header(100));

        let error = block_header_hint(&config, 101).expect_err("mismatch should fail");

        assert!(error.to_string().contains("block header hint mismatch"));
    }
}
