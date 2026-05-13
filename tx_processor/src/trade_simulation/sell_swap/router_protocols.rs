use alloy_primitives::U256;
use eyre::{eyre, Result};
use std::sync::Arc;
use tx_simulator::tx_builders::{self, amm_swap_route::AmmSwapRoute};
use tx_simulator::TxSimulator;

use crate::trade_simulation::types::PoolBuySellParameters;
use crate::tx_processor::TxProcessor;

use super::balance_setup::{log_token_balance_setup, prepare_seller_token_balance};
use super::common::{
    apply_sell_fee_policy, failed_sell_result, format_failure_with_revert, SELLER_ETH_FUND,
};
use super::denom_output::extract_denom_received;
use super::SellSwapResult;

pub(super) async fn simulate_router_protocol_sell(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
    tokens_to_sell: U256,
    block: u64,
) -> Result<SellSwapResult> {
    let seller_address = config.buyer_address;
    let route = if let Some(protocol) = config.pool_type.known_v2_protocol() {
        AmmSwapRoute::V2Router {
            pool: config.pool_address,
            router: protocol.router(),
        }
    } else if let (Some(protocol), Some(fee_tier)) = (
        config.pool_type.known_v3_protocol(),
        config.pool_type.v3_fee_tier(),
    ) {
        AmmSwapRoute::V3Router {
            pool: config.pool_address,
            router: protocol.router(),
            fee_tier,
        }
    } else {
        return Err(eyre!(
            "Pool type {:?} not yet supported for router sell simulation",
            config.pool_type
        ));
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
        "router-protocol chain-sim sell",
    );

    let mut approve_tx = tx_builders::build_approve_for_route(
        &route,
        seller_address,
        config.token_address,
        tokens_to_sell,
    );
    apply_sell_fee_policy(&mut approve_tx, config.approve_gas_limit, base_fee);
    let approve_sim = chain.step_with_trace(approve_tx.clone()).await?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(&approve_tx, &approve_sim, block, 0)
        .await?;
    if !approve_sim.success {
        return Ok(failed_sell_result(
            &config,
            tokens_to_sell,
            approve_processed,
            block,
            "Sell approve transaction failed",
            approve_sim.revert_reason.as_deref(),
        ));
    }

    let slippage_bps = (config.slippage_tolerance * 100.0).round() as u32;
    let mut sell_tx = tx_builders::build_sell_swap(
        &route,
        seller_address,
        config.token_address,
        tokens_to_sell,
        slippage_bps,
        u64::MAX,
    );
    apply_sell_fee_policy(&mut sell_tx, config.sell_gas_limit, base_fee);
    let sell_sim = chain.step_with_trace(sell_tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim, block, 1)
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
                "Sell transaction failed",
                sell_sim.revert_reason.as_deref(),
            ))
        },
    })
}
