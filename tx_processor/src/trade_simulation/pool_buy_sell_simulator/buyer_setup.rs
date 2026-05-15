use std::sync::Arc;

use alloy_primitives::{Address, Bytes, U256};
use eyre::{eyre, Result};
use reth_chain_query::dex::{
    encoding::{encode_function_call, encode_two_addresses},
    UNISWAP_V2_FACTORY,
};
use tx_simulator::{
    tx_builders::{
        amm_swap_route::AmmSwapRoute, build_approve_for_route, build_denom_to_token_swap,
        uniswap_v4::build_weth_deposit_tx as build_v4_weth_deposit_tx,
    },
    UnsignedTxChainSimulation,
};

use super::failure::{format_failure_with_full_trace, format_failure_with_revert};
use super::fees::apply_fee_policy;
use super::results::create_failed_result;
use crate::trade_simulation::types::{
    PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;

const GET_RESERVES_SELECTOR: [u8; 4] = [0x09, 0x02, 0xf1, 0xac];
const UNISWAP_V2_FACTORY_GET_PAIR: [u8; 4] = [0xe6, 0xa4, 0x39, 0x05];
const FEE_NUMERATOR: u128 = 997;
const FEE_DENOMINATOR: u128 = 1000;
const PREFUND_BUFFER_BPS: u128 = 105; // 5% buffer

fn decode_address_response(output: &[u8], context: &str) -> Result<Address> {
    if output.len() < 32 {
        return Err(eyre!("{context} returned output shorter than 32 bytes"));
    }
    Ok(Address::from_slice(&output[12..32]))
}

fn decode_reserves(output: &[u8]) -> Result<(U256, U256)> {
    if output.len() < 64 {
        return Err(eyre!(
            "getReserves return data too short ({} bytes)",
            output.len()
        ));
    }
    let reserve0 = U256::from_be_slice(&output[0..32]);
    let reserve1 = U256::from_be_slice(&output[32..64]);
    Ok((reserve0, reserve1))
}

fn quote_in_amount(amount_out: U256, reserve_in: U256, reserve_out: U256) -> Result<U256> {
    if amount_out.is_zero() {
        return Err(eyre!("desired output amount is zero"));
    }
    if amount_out >= reserve_out {
        return Err(eyre!(
            "desired output {} exceeds pool reserve {}",
            amount_out,
            reserve_out
        ));
    }
    let fee_num = U256::from(FEE_NUMERATOR);
    let fee_den = U256::from(FEE_DENOMINATOR);

    let numerator = reserve_in
        .checked_mul(amount_out)
        .and_then(|n| n.checked_mul(fee_den))
        .ok_or_else(|| eyre!("overflow computing numerator"))?;
    let denominator = reserve_out
        .checked_sub(amount_out)
        .and_then(|d| d.checked_mul(fee_num))
        .ok_or_else(|| eyre!("overflow computing denominator"))?;
    if denominator.is_zero() {
        return Err(eyre!("denominator zero while quoting input amount"));
    }
    numerator
        .checked_div(denominator)
        .and_then(|v| v.checked_add(U256::from(1u64)))
        .ok_or_else(|| eyre!("overflow adding safety margin"))
}

fn apply_buffer(amount: U256, bps: u128) -> Result<U256> {
    let multiplier = U256::from(bps);
    amount
        .checked_mul(multiplier)
        .and_then(|v| v.checked_div(U256::from(100u64)))
        .ok_or_else(|| eyre!("overflow applying buffer"))
}

fn denom_prefund_unavailable_result(
    config: &PoolBuySellParameters,
    block_number: u64,
    prior_tx_results: &[ProcessedTransaction],
    reason: impl Into<String>,
) -> PoolBuySellSimulationResult {
    create_failed_result(
        config.clone(),
        block_number,
        prior_tx_results.to_vec(),
        None,
        None,
        None,
        format!("Denomination top-up unavailable: {}", reason.into()),
        false,
        false,
        false,
    )
}

fn fetch_uniswap_v2_pair_address_on_chain(
    chain: &mut UnsignedTxChainSimulation,
    token_a: Address,
    token_b: Address,
    block_number: u64,
) -> Result<Address> {
    let params = encode_two_addresses(token_a, token_b);
    let call_data = encode_function_call(UNISWAP_V2_FACTORY_GET_PAIR, &params);
    let response = chain
        .simulate_view_call(UNISWAP_V2_FACTORY, call_data)
        .map_err(|err| {
            eyre!(
                "Uniswap V2 factory getPair view failed at block {} for token_a={} token_b={}: {}",
                block_number,
                token_a,
                token_b,
                err
            )
        })?;

    if !response.success {
        return Err(eyre!(
            "Uniswap V2 factory getPair reverted at block {} for token_a={} token_b={}",
            block_number,
            token_a,
            token_b
        ));
    }

    decode_address_response(&response.output, "Uniswap V2 factory getPair")
}

fn fetch_uniswap_v2_reserves_on_chain(
    chain: &mut UnsignedTxChainSimulation,
    pair_address: Address,
    block_number: u64,
) -> Result<(U256, U256)> {
    let reserve_response = chain
        .simulate_view_call(pair_address, Bytes::from(GET_RESERVES_SELECTOR.to_vec()))
        .map_err(|err| {
            eyre!(
                "failed to fetch reserves for pair {:#x} at block {}: {}",
                pair_address,
                block_number,
                err
            )
        })?;

    if !reserve_response.success {
        return Err(eyre!(
            "getReserves reverted for pair {:#x} at block {} (calldata len {})",
            pair_address,
            block_number,
            reserve_response.output.len()
        ));
    }

    decode_reserves(&reserve_response.output)
}

async fn execute_weth_deposit(
    chain: &mut UnsignedTxChainSimulation,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
    tx_processor: Arc<TxProcessor>,
    block_number: u64,
    prior_tx_results: &mut Vec<ProcessedTransaction>,
    amount: U256,
) -> Result<Option<PoolBuySellSimulationResult>> {
    if amount.is_zero() {
        return Ok(None);
    }
    let mut deposit_tx =
        build_v4_weth_deposit_tx(config.buyer_address, config.weth_address, amount);
    deposit_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut deposit_tx, config, base_fee);

    let deposit_result = chain.step_with_trace(deposit_tx.clone()).await.map_err(|err| {
        let context = format!(
            "while simulating WETH deposit with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
            deposit_tx.gas,
            deposit_tx.gas_price,
            deposit_tx.max_fee_per_gas,
            deposit_tx.max_priority_fee_per_gas
        );
        tracing::warn!(
            target: "pool_buy_sell_sim",
            step = "weth_deposit",
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
        let failure = create_failed_result(
            config.clone(),
            block_number,
            prior_tx_results.clone(),
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
        );
        return Ok(Some(failure));
    }

    Ok(None)
}

#[allow(clippy::too_many_arguments)]
async fn prefund_denom_via_weth(
    chain: &mut UnsignedTxChainSimulation,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
    tx_processor: Arc<TxProcessor>,
    block_number: u64,
    prior_tx_results: &mut Vec<ProcessedTransaction>,
) -> Result<Option<PoolBuySellSimulationResult>> {
    let pair_address = fetch_uniswap_v2_pair_address_on_chain(
        chain,
        config.denom_address,
        config.weth_address,
        block_number,
    )?;

    if pair_address.is_zero() {
        return Ok(None);
    }

    let (reserve0, reserve1) =
        fetch_uniswap_v2_reserves_on_chain(chain, pair_address, block_number)?;
    let denom_first = config.denom_address < config.weth_address;

    let (reserve_denom, reserve_weth) = if denom_first {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    if reserve_denom.is_zero() || reserve_weth.is_zero() {
        return Ok(Some(denom_prefund_unavailable_result(
            config,
            block_number,
            prior_tx_results,
            format!(
                "pool {:#x} reserves exhausted (denom {}, weth {})",
                pair_address, reserve_denom, reserve_weth
            ),
        )));
    }

    let desired_out = config.test_amount;
    let weth_needed = match quote_in_amount(desired_out, reserve_weth, reserve_denom) {
        Ok(value) => value,
        Err(error) => {
            return Ok(Some(denom_prefund_unavailable_result(
                config,
                block_number,
                prior_tx_results,
                error.to_string(),
            )));
        }
    };
    let weth_buffered = apply_buffer(weth_needed, PREFUND_BUFFER_BPS)?;

    if let Some(failure) = execute_weth_deposit(
        chain,
        config,
        base_fee,
        tx_processor.clone(),
        block_number,
        prior_tx_results,
        weth_buffered,
    )
    .await?
    {
        return Ok(Some(failure));
    }

    let route = AmmSwapRoute::UniswapV2 { pool: pair_address };
    let mut approve_tx =
        build_approve_for_route(&route, config.buyer_address, config.weth_address, U256::MAX);
    approve_tx.gas = Some(config.approve_gas_limit);
    apply_fee_policy(&mut approve_tx, config, base_fee);
    let approve_result = chain
        .step_with_trace(approve_tx.clone())
        .await
        .map_err(|err| {
            let context = "while approving WETH for denomination top-up".to_string();
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "denom_prefund_approve",
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
            prior_tx_results.len() as u64,
        )
        .await?;
    prior_tx_results.push(approve_processed.clone());
    if !approve_result.success {
        let failure = create_failed_result(
            config.clone(),
            block_number,
            prior_tx_results.clone(),
            None,
            None,
            None,
            format_failure_with_revert(
                "Denomination top-up approval failed",
                approve_result.revert_reason.as_deref(),
            ),
            false,
            false,
            false,
        );
        return Ok(Some(failure));
    }

    let mut swap_tx = build_denom_to_token_swap(
        &route,
        config.buyer_address,
        config.weth_address,
        config.denom_address,
        weth_buffered,
        0,
        u64::MAX,
    );
    swap_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut swap_tx, config, base_fee);
    let swap_result = chain.step_with_trace(swap_tx.clone()).await.map_err(|err| {
        let context = format!(
            "while executing denomination top-up swap with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
            swap_tx.gas,
            swap_tx.gas_price,
            swap_tx.max_fee_per_gas,
            swap_tx.max_priority_fee_per_gas
        );
        tracing::warn!(
            target: "pool_buy_sell_sim",
            step = "denom_prefund_swap",
            %context,
            block = block_number,
            error = %err
        );
        err.wrap_err(context)
    })?;
    let swap_processed = tx_processor
        .process_transaction_from_simulation_result(
            &swap_tx,
            &swap_result,
            block_number,
            prior_tx_results.len() as u64,
        )
        .await?;
    prior_tx_results.push(swap_processed.clone());

    if !swap_result.success {
        let failure_message =
            format_failure_with_full_trace("Denomination top-up swap failed", &swap_result);
        let failure = create_failed_result(
            config.clone(),
            block_number,
            prior_tx_results.clone(),
            None,
            None,
            None,
            failure_message,
            false,
            false,
            false,
        );
        return Ok(Some(failure));
    }

    Ok(None)
}

pub(super) async fn prepare_buyer_account(
    chain: &mut UnsignedTxChainSimulation,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
    tx_processor: Arc<TxProcessor>,
    block_number: u64,
    prior_tx_results: &mut Vec<ProcessedTransaction>,
) -> Result<Option<PoolBuySellSimulationResult>> {
    if config.pool_type != PoolType::UniswapV4 && config.denom_address == config.weth_address {
        if let Some(failure) = execute_weth_deposit(
            chain,
            config,
            base_fee,
            tx_processor.clone(),
            block_number,
            prior_tx_results,
            config.test_amount,
        )
        .await?
        {
            return Ok(Some(failure));
        }
        return Ok(None);
    }

    if config.denom_address != config.weth_address {
        if let Some(failure) = prefund_denom_via_weth(
            chain,
            config,
            base_fee,
            tx_processor,
            block_number,
            prior_tx_results,
        )
        .await?
        {
            return Ok(Some(failure));
        }
    }

    Ok(None)
}
