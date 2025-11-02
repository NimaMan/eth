use std::sync::Arc;

use eyre::Result;
use tx_simulator::UnsignedTxChainSimulation;

use super::failure::format_failure_with_revert;
use super::fees::apply_fee_policy;
use super::results::create_failed_result;
use crate::simulator::types::{PoolBuySellParameters, PoolBuySellSimulationResult, PoolType};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;
use reth_chain_query::tx_builders::amm::build_weth_deposit_tx as build_v4_weth_deposit_tx;

pub(super) async fn prepare_buyer_account(
    chain: &mut UnsignedTxChainSimulation,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
    tx_processor: Arc<TxProcessor>,
    block_number: u64,
    prior_tx_results: &mut Vec<ProcessedTransaction>,
) -> Result<Option<PoolBuySellSimulationResult>> {
    if config.pool_type == PoolType::UniswapV4 || config.denom_address != config.weth_address {
        return Ok(None);
    }

    let mut deposit_tx = build_v4_weth_deposit_tx(
        config.buyer_address,
        config.weth_address,
        config.test_amount,
    );
    deposit_tx.gas = Some(config.buy_gas_limit);
    apply_fee_policy(&mut deposit_tx, config, base_fee);

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
