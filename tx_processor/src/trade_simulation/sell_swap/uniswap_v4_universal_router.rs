use alloy_primitives::{address, Address, U256};
use eyre::{eyre, Result};
use std::sync::Arc;
use tx_simulator::tx_builders::{
    permit2::build_permit2_approve_tx,
    uniswap_v4::{
        build_token_approval_tx, build_universal_router_v4_exact_input_single_tx,
        infer_orientation_from_input, UniswapV4PoolKey as BuilderV4PoolKey,
        UniversalRouterV4ExactInputSingleRequest, UniversalRouterV4InputPayment,
    },
};
use tx_simulator::TxSimulator;

use crate::trade_simulation::types::PoolBuySellParameters;
use crate::tx_processor::TxProcessor;

use super::balance_setup::{log_token_balance_setup, prepare_seller_token_balance};
use super::common::{
    apply_sell_fee_policy, currency_matches_denom, failed_sell_result, format_failure_with_revert,
    permit2_amount, PERMIT2, PERMIT2_EXPIRATION, SELLER_ETH_FUND,
};
use super::denom_output::extract_denom_received;
use super::SellSwapResult;

const UNIVERSAL_ROUTER_V4: Address = address!("66a9893cC07D91D95644AEDD05D03f95e1dBA8Af");

pub(super) async fn simulate_universal_router_v4_sell(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
    tokens_to_sell: U256,
    block: u64,
) -> Result<SellSwapResult> {
    let v4_cfg = config
        .uniswap_v4_config
        .clone()
        .ok_or_else(|| eyre!("Uniswap V4 configuration must be provided"))?;
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    let base_fee = chain.block_base_fee();
    let seller_address = config.buyer_address;

    if !chain.account_has_code(v4_cfg.pool_manager)? {
        return Err(eyre!(
            "Uniswap V4 PoolManager {:#x} has no bytecode at block {}",
            v4_cfg.pool_manager,
            block
        ));
    }

    let pool_key = BuilderV4PoolKey {
        currency0: v4_cfg.currency0,
        currency1: v4_cfg.currency1,
        fee: v4_cfg.fee,
        tick_spacing: v4_cfg.tick_spacing,
        hooks: v4_cfg.hooks,
    };
    let sell_orientation = infer_orientation_from_input(&pool_key, config.token_address)?;
    if !currency_matches_denom(
        sell_orientation.output_currency,
        config.denom_address,
        config.weth_address,
    ) {
        return Err(eyre!(
            "Uniswap V4 sell output currency {:#x} does not match configured denom {:#x}",
            sell_orientation.output_currency,
            config.denom_address
        ));
    }

    chain.set_eth_balance(seller_address, U256::from(SELLER_ETH_FUND))?;
    let balance_setup = prepare_seller_token_balance(
        &mut chain,
        config.token_address,
        config.pool_address,
        seller_address,
        tokens_to_sell,
        config.sell_gas_limit,
        base_fee,
    )
    .await?;
    let Some(balance_setup) = balance_setup else {
        return Err(eyre!(
            "unable to inject synthetic ERC20 balance for token {:#x}; unsupported balance storage layout",
            config.token_address
        ));
    };
    log_token_balance_setup(
        &balance_setup,
        config.token_address,
        seller_address,
        tokens_to_sell,
        "V4 chain-sim sell",
    );

    let mut token_approve =
        build_token_approval_tx(seller_address, config.token_address, PERMIT2, U256::MAX);
    apply_sell_fee_policy(&mut token_approve, config.approve_gas_limit, base_fee);
    let token_approve_sim = chain.step_with_trace(token_approve.clone()).await?;
    let token_approve_processed = tx_processor
        .process_transaction_from_simulation_result(&token_approve, &token_approve_sim, block, 0)
        .await?;
    if !token_approve_sim.success {
        return Ok(failed_sell_result(
            &config,
            tokens_to_sell,
            token_approve_processed,
            block,
            "Token approval for Permit2 failed",
            token_approve_sim.revert_reason.as_deref(),
        ));
    }

    let mut permit2_approve = build_permit2_approve_tx(
        seller_address,
        PERMIT2,
        config.token_address,
        UNIVERSAL_ROUTER_V4,
        permit2_amount(tokens_to_sell)?,
        PERMIT2_EXPIRATION,
    )?;
    apply_sell_fee_policy(&mut permit2_approve, config.approve_gas_limit, base_fee);
    let permit2_approve_sim = chain.step_with_trace(permit2_approve.clone()).await?;
    let permit2_approve_processed = tx_processor
        .process_transaction_from_simulation_result(
            &permit2_approve,
            &permit2_approve_sim,
            block,
            1,
        )
        .await?;
    if !permit2_approve_sim.success {
        return Ok(failed_sell_result(
            &config,
            tokens_to_sell,
            permit2_approve_processed,
            block,
            "Permit2 approval for Universal Router failed",
            permit2_approve_sim.revert_reason.as_deref(),
        ));
    }

    let mut sell_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: UNIVERSAL_ROUTER_V4,
            caller: seller_address,
            pool_key,
            token_in: config.token_address,
            token_out: sell_orientation.output_currency,
            amount_in: tokens_to_sell,
            min_amount_out: U256::ZERO,
            deadline: U256::from(u64::MAX),
            hook_data: v4_cfg.hook_data,
            input_payment: UniversalRouterV4InputPayment::Permit2User,
        },
    )?;
    sell_tx.gas = Some(config.sell_gas_limit);
    apply_sell_fee_policy(&mut sell_tx, config.sell_gas_limit, base_fee);
    let sell_sim = chain.step_with_trace(sell_tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim, block, 2)
        .await?;
    let denom_received = extract_denom_received(
        &processed,
        seller_address,
        config.pool_address,
        config.denom_address,
        config.weth_address,
    );
    let success = sell_sim.success;

    Ok(SellSwapResult {
        success,
        seller_address,
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
        tokens_sold: tokens_to_sell,
        denom_received,
        sell_transaction: processed,
        block_number: block,
        failure_reason: if success {
            None
        } else {
            Some(format_failure_with_revert(
                "Universal Router V4 sell transaction failed",
                sell_sim.revert_reason.as_deref(),
            ))
        },
    })
}
