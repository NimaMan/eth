use alloy_primitives::{address, Address, U256};
use eyre::{eyre, Result};
use std::sync::Arc;
use tx_simulator::tx_builders::{
    permit2::build_permit2_approve_tx,
    uniswap_v3::{build_universal_router_v3_exact_input_tx, UniversalRouterV3ExactInputRequest},
    uniswap_v4::build_token_approval_tx,
};
use tx_simulator::TxSimulator;

use crate::trade_simulation::types::PoolBuySellParameters;
use crate::tx_processor::TxProcessor;

use super::balance_setup::{log_token_balance_setup, prepare_seller_token_balance};
use super::common::{
    apply_sell_fee_policy, failed_sell_result, failed_sell_result_with_fees, fee_totals,
    format_failure_with_revert, permit2_amount, PERMIT2, PERMIT2_EXPIRATION, SELLER_ETH_FUND,
};
use super::denom_output::extract_denom_received;
use super::SellSwapResult;

const UNIVERSAL_ROUTER_V3: Address = address!("4C82D1fBFe28C977cBB58D8C7FF8FCF9F70a2cCA");

pub(super) async fn simulate_universal_router_v3_sell(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
    tokens_to_sell: U256,
    fee_tier: u32,
    block: u64,
) -> Result<SellSwapResult> {
    let seller_address = config.buyer_address;
    let token_out = if config.denom_address.is_zero() {
        config.weth_address
    } else {
        config.denom_address
    };

    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    let base_fee = chain.block_base_fee();
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
        "V3 Universal Router chain-sim sell",
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
        UNIVERSAL_ROUTER_V3,
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
        let (gas_used, gas_cost) =
            fee_totals(&[&token_approve_processed, &permit2_approve_processed]);
        return Ok(failed_sell_result_with_fees(
            &config,
            tokens_to_sell,
            permit2_approve_processed,
            gas_used,
            gas_cost,
            block,
            "Permit2 approval for Universal Router failed",
            permit2_approve_sim.revert_reason.as_deref(),
        ));
    }

    let mut sell_tx =
        build_universal_router_v3_exact_input_tx(&UniversalRouterV3ExactInputRequest {
            universal_router: UNIVERSAL_ROUTER_V3,
            caller: seller_address,
            recipient: seller_address,
            token_in: config.token_address,
            token_out,
            fee: fee_tier,
            amount_in: tokens_to_sell,
            min_amount_out: U256::ZERO,
            deadline: U256::from(u64::MAX),
            payer_is_user: true,
            unwrap_weth_to: Some(seller_address),
        })?;
    apply_sell_fee_policy(&mut sell_tx, config.sell_gas_limit, base_fee);
    let sell_sim = chain.step_with_trace(sell_tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim, block, 2)
        .await?;
    let (gas_used, gas_cost) = fee_totals(&[
        &token_approve_processed,
        &permit2_approve_processed,
        &processed,
    ]);
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
        gas_used,
        gas_cost,
        block_number: block,
        failure_reason: if success {
            None
        } else {
            Some(format_failure_with_revert(
                "Universal Router V3 sell transaction failed",
                sell_sim.revert_reason.as_deref(),
            ))
        },
    })
}
