use alloy_primitives::{address, Address, U256};
use eyre::{eyre, Result};
use tx_simulator::UnsignedTransaction;

use crate::trade_simulation::types::PoolBuySellParameters;
use crate::tx_processor::data_models::ProcessedTransaction;

use crate::trade_simulation::sell_swap::SellSwapResult;

pub(in crate::trade_simulation::sell_swap) const SELLER_ETH_FUND: u128 = 1_000_000_000_000_000_000;
pub(in crate::trade_simulation::sell_swap) const PERMIT2: Address =
    address!("000000000022D473030F116dDEE9F6B43aC78BA3");
pub(in crate::trade_simulation::sell_swap) const PERMIT2_EXPIRATION: u64 = (1_u64 << 48) - 1;

pub(in crate::trade_simulation::sell_swap) fn apply_sell_fee_policy(
    tx: &mut UnsignedTransaction,
    gas_limit: u64,
    base_fee: Option<u128>,
) {
    apply_sell_fee_policy_with_overrides(tx, gas_limit, base_fee, None, None, None);
}

pub(in crate::trade_simulation::sell_swap) fn apply_configured_sell_fee_policy(
    tx: &mut UnsignedTransaction,
    config: &PoolBuySellParameters,
    gas_limit: u64,
    base_fee: Option<u128>,
) {
    apply_sell_fee_policy_with_overrides(
        tx,
        gas_limit,
        base_fee,
        config.gas_price,
        config.max_fee_per_gas,
        config.max_priority_fee_per_gas,
    );
}

fn apply_sell_fee_policy_with_overrides(
    tx: &mut UnsignedTransaction,
    gas_limit: u64,
    base_fee: Option<u128>,
    gas_price: Option<u128>,
    max_fee_per_gas: Option<u128>,
    max_priority_fee_per_gas: Option<u128>,
) {
    tx.gas = Some(gas_limit);
    if let Some(gas_price) = gas_price {
        tx.gas_price = Some(gas_price);
        tx.max_fee_per_gas = None;
        tx.max_priority_fee_per_gas = None;
    } else if base_fee.is_some() || max_fee_per_gas.is_some() || max_priority_fee_per_gas.is_some()
    {
        let base_fee = base_fee.unwrap_or(0);
        let priority_fee = max_priority_fee_per_gas.unwrap_or(0);
        let min_required = base_fee.saturating_add(priority_fee);
        let max_fee = max_fee_per_gas.unwrap_or(min_required).max(min_required);
        tx.gas_price = None;
        tx.max_fee_per_gas = Some(max_fee);
        tx.max_priority_fee_per_gas = Some(priority_fee);
    } else if tx.gas_price.is_none() && tx.max_fee_per_gas.is_none() {
        tx.gas_price = Some(1);
    }
}

pub(in crate::trade_simulation::sell_swap) fn format_failure_with_revert(
    prefix: &str,
    revert_reason: Option<&str>,
) -> String {
    match revert_reason.filter(|reason| !reason.trim().is_empty()) {
        Some(reason) => format!("{prefix}: {reason}"),
        None => prefix.to_string(),
    }
}

pub(in crate::trade_simulation::sell_swap) fn fee_totals(
    transactions: &[&ProcessedTransaction],
) -> (u64, U256) {
    transactions
        .iter()
        .fold((0_u64, U256::ZERO), |(gas_used, gas_cost), transaction| {
            (
                gas_used.saturating_add(transaction.fees.gas_used),
                gas_cost.saturating_add(transaction.fees.tx_fee),
            )
        })
}

pub(in crate::trade_simulation::sell_swap) fn failed_sell_result(
    config: &PoolBuySellParameters,
    tokens_to_sell: U256,
    processed: ProcessedTransaction,
    block: u64,
    message: &str,
    revert_reason: Option<&str>,
) -> SellSwapResult {
    let (gas_used, gas_cost) = fee_totals(&[&processed]);
    failed_sell_result_with_fees(
        config,
        tokens_to_sell,
        processed,
        gas_used,
        gas_cost,
        block,
        message,
        revert_reason,
    )
}

pub(in crate::trade_simulation::sell_swap) fn failed_sell_result_with_fees(
    config: &PoolBuySellParameters,
    tokens_to_sell: U256,
    processed: ProcessedTransaction,
    gas_used: u64,
    gas_cost: U256,
    block: u64,
    message: &str,
    revert_reason: Option<&str>,
) -> SellSwapResult {
    SellSwapResult {
        success: false,
        seller_address: config.buyer_address,
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
        tokens_sold: tokens_to_sell,
        denom_received: U256::ZERO,
        sell_transaction: processed,
        gas_used,
        gas_cost,
        block_number: block,
        failure_reason: Some(format_failure_with_revert(message, revert_reason)),
    }
}

pub(in crate::trade_simulation::sell_swap) fn currency_matches_denom(
    currency: Address,
    denom: Address,
    weth: Address,
) -> bool {
    currency == denom || (currency.is_zero() && (denom.is_zero() || denom == weth))
}

pub(in crate::trade_simulation::sell_swap) fn permit2_amount(amount: U256) -> Result<U256> {
    let max = (U256::from(1_u8) << 160) - U256::from(1_u8);
    if amount > max {
        return Err(eyre!("Permit2 allowance amount must fit uint160"));
    }
    Ok(amount)
}
