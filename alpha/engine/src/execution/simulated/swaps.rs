use std::sync::{Arc, Mutex};

use alloy_primitives::U256;
use eth_alpha_core::{
    amount::Amount,
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::OrderId,
    market::PoolSnapshot,
    order::OrderIntent,
    portfolio::PortfolioState,
};
use tx_processor::tx_processor::TxProcessor;
use tx_processor::{
    simulate_buy_swap_with_params, simulate_buy_swap_with_params_and_chain,
    simulate_sell_swap_with_params, simulate_sell_swap_with_params_and_chain, BuySwapResult,
    SellSwapResult,
};
use tx_simulator::{LiveTxSimulator, TxSimulator};

use super::pool_context::{pool_simulation_parameters, pool_simulation_parameters_live};
use super::reports::{
    cancelled_report_at, failed_report_at, failed_report_at_with_gas, gas_cost_amount,
};
use crate::execution::sell_economics::uneconomic_sell_cancellation_reason;

/// Simulate a buy at a specific block. Returns the ExecutionReport with
/// `token_amount` populated from the EVM trace.
pub(super) async fn simulate_buy_at_block(
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
) -> Result<ExecutionReport> {
    let eth_amount = intent.amount.raw;
    let params =
        match pool_simulation_parameters(simulator, pool, intent.token_address, eth_amount, block)
            .await
        {
            Ok(params) => params,
            Err(error) => {
                return Ok(failed_report_at(
                    order_id,
                    format!("invalid pool parameters for chain simulation: {error}"),
                    block,
                ));
            }
        };

    let result: BuySwapResult = match simulate_buy_swap_with_params(
        simulator.clone(),
        tx_processor.clone(),
        params.clone(),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("chain buy simulation failed: {error}"),
                block,
            ));
        }
    };

    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("buy simulation failed".to_string()),
            block,
            Some(result.buy_transaction.fees.gas_used),
            Some(result.buy_transaction.fees.tx_fee),
        ));
    }

    let token_amount = Amount {
        raw: result.tokens_received,
        decimals: params.token_decimals,
    };

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(intent.amount),
        token_amount: Some(token_amount),
        gas_used: Some(result.buy_transaction.fees.gas_used),
        gas_cost: Some(gas_cost_amount(result.buy_transaction.fees.tx_fee)),
        mined_evidence: None,
        error: None,
    })
}

/// Simulate a sell at a specific block using a raw token amount and the same
/// swap simulation path used for exits and mark-to-market valuation.
pub(super) async fn simulate_sell_at_block(
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
    _portfolio: &Arc<Mutex<PortfolioState>>,
    skip_uneconomic_sell: bool,
) -> Result<ExecutionReport> {
    let tokens_to_sell = intent.amount.raw;

    if tokens_to_sell.is_zero() {
        return Ok(failed_report_at(
            order_id,
            "zero token amount; nothing to sell",
            block,
        ));
    }

    let params = match pool_simulation_parameters(
        simulator,
        pool,
        intent.token_address,
        tokens_to_sell,
        block,
    )
    .await
    {
        Ok(params) => params,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("invalid pool parameters for chain simulation: {error}"),
                block,
            ));
        }
    };

    let denom_decimals = params.denom_decimals;
    let result: SellSwapResult = match simulate_sell_swap_with_params(
        simulator.clone(),
        tx_processor.clone(),
        params,
        tokens_to_sell,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("chain sell simulation failed: {error}"),
                block,
            ));
        }
    };

    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("sell simulation failed".to_string()),
            block,
            Some(result.gas_used),
            Some(result.gas_cost),
        ));
    }

    let denom_received = Amount {
        raw: result.denom_received,
        decimals: denom_decimals,
    };

    if skip_uneconomic_sell {
        if let Some(reason) =
            uneconomic_sell_cancellation_reason(pool, result.denom_received, result.gas_cost)
        {
            return Ok(cancelled_report_at(order_id, reason, block));
        }
    }

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(denom_received),
        token_amount: None,
        gas_used: Some(result.gas_used),
        gas_cost: Some(gas_cost_amount(result.gas_cost)),
        mined_evidence: None,
        error: None,
    })
}

pub(super) async fn simulate_live_buy_at_block(
    live_simulator: &LiveTxSimulator,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
) -> Result<ExecutionReport> {
    let simulator = live_simulator.simulator();
    let eth_amount = intent.amount.raw;
    let mut chain = match live_simulator.start_chain_at(block).await {
        Ok(chain) => chain,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live simulator state unavailable for buy block {block}: {error}"),
                block,
            ));
        }
    };
    let params = match pool_simulation_parameters_live(
        &mut chain,
        pool,
        intent.token_address,
        eth_amount,
        block,
    ) {
        Ok(params) => params,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("invalid pool parameters for live chain simulation: {error}"),
                block,
            ));
        }
    };
    let token_decimals = params.token_decimals;
    let result: BuySwapResult = match simulate_buy_swap_with_params_and_chain(
        simulator,
        tx_processor.clone(),
        params,
        chain,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live chain buy simulation failed: {error}"),
                block,
            ));
        }
    };
    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("buy simulation failed".to_string()),
            block,
            Some(result.buy_transaction.fees.gas_used),
            Some(result.buy_transaction.fees.tx_fee),
        ));
    }

    let token_amount = Amount {
        raw: result.tokens_received,
        decimals: token_decimals,
    };

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(intent.amount),
        token_amount: Some(token_amount),
        gas_used: Some(result.buy_transaction.fees.gas_used),
        gas_cost: Some(gas_cost_amount(result.buy_transaction.fees.tx_fee)),
        mined_evidence: None,
        error: None,
    })
}

pub(super) async fn simulate_live_sell_at_block(
    live_simulator: &LiveTxSimulator,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
    _portfolio: &Arc<Mutex<PortfolioState>>,
    skip_uneconomic_sell: bool,
) -> Result<ExecutionReport> {
    let simulator = live_simulator.simulator();
    let tokens_to_sell = intent.amount.raw;

    if tokens_to_sell.is_zero() {
        return Ok(failed_report_at(
            order_id,
            "zero token amount; nothing to sell",
            block,
        ));
    }
    let mut chain = match live_simulator.start_chain_at(block).await {
        Ok(chain) => chain,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live simulator state unavailable for sell block {block}: {error}"),
                block,
            ));
        }
    };
    let params = match pool_simulation_parameters_live(
        &mut chain,
        pool,
        intent.token_address,
        U256::ZERO,
        block,
    ) {
        Ok(params) => params,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("invalid pool parameters for live chain simulation: {error}"),
                block,
            ));
        }
    };
    let denom_decimals = params.denom_decimals;
    let result: SellSwapResult = match simulate_sell_swap_with_params_and_chain(
        simulator,
        tx_processor.clone(),
        params,
        tokens_to_sell,
        chain,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live chain sell simulation failed: {error}"),
                block,
            ));
        }
    };
    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("sell simulation failed".to_string()),
            block,
            Some(result.gas_used),
            Some(result.gas_cost),
        ));
    }

    let denom_received = Amount {
        raw: result.denom_received,
        decimals: denom_decimals,
    };

    if skip_uneconomic_sell {
        if let Some(reason) =
            uneconomic_sell_cancellation_reason(pool, result.denom_received, result.gas_cost)
        {
            return Ok(cancelled_report_at(order_id, reason, block));
        }
    }

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(denom_received),
        token_amount: None,
        gas_used: Some(result.gas_used),
        gas_cost: Some(gas_cost_amount(result.gas_cost)),
        mined_evidence: None,
        error: None,
    })
}
