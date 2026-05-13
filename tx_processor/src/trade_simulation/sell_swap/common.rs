use alloy_primitives::{address, Address, U256};
use eyre::{eyre, Result};
use tx_simulator::UnsignedTransaction;

use crate::trade_simulation::types::PoolBuySellParameters;
use crate::tx_processor::data_models::ProcessedTransaction;

use super::SellSwapResult;

pub(super) const SELLER_ETH_FUND: u128 = 1_000_000_000_000_000_000;
pub(super) const PERMIT2: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");
pub(super) const PERMIT2_EXPIRATION: u64 = (1_u64 << 48) - 1;

pub(super) fn apply_sell_fee_policy(
    tx: &mut UnsignedTransaction,
    gas_limit: u64,
    base_fee: Option<u128>,
) {
    tx.gas = Some(gas_limit);
    if let Some(base_fee) = base_fee {
        tx.gas_price = None;
        tx.max_fee_per_gas = Some(base_fee);
        tx.max_priority_fee_per_gas = Some(0);
    } else if tx.gas_price.is_none() && tx.max_fee_per_gas.is_none() {
        tx.gas_price = Some(1);
    }
}

pub(super) fn format_failure_with_revert(prefix: &str, revert_reason: Option<&str>) -> String {
    match revert_reason.filter(|reason| !reason.trim().is_empty()) {
        Some(reason) => format!("{prefix}: {reason}"),
        None => prefix.to_string(),
    }
}

pub(super) fn failed_sell_result(
    config: &PoolBuySellParameters,
    tokens_to_sell: U256,
    processed: ProcessedTransaction,
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
        block_number: block,
        failure_reason: Some(format_failure_with_revert(message, revert_reason)),
    }
}

pub(super) fn currency_matches_denom(currency: Address, denom: Address, weth: Address) -> bool {
    currency == denom || (currency.is_zero() && (denom.is_zero() || denom == weth))
}

pub(super) fn permit2_amount(amount: U256) -> Result<U256> {
    let max = (U256::from(1_u8) << 160) - U256::from(1_u8);
    if amount > max {
        return Err(eyre!("Permit2 allowance amount must fit uint160"));
    }
    Ok(amount)
}
