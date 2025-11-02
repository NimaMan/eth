use tx_simulator::UnsignedTransaction;

use crate::simulator::types::PoolBuySellParameters;
use crate::tx_processor::data_models::ProcessedTransaction;

pub(super) fn apply_fee_policy(
    tx: &mut UnsignedTransaction,
    config: &PoolBuySellParameters,
    base_fee: Option<u128>,
) {
    if tx.gas.is_none() {
        tx.gas = Some(config.buy_gas_limit);
    }

    if let Some(price) = tx.gas_price {
        if price == 0 {
            tx.gas_price = config.gas_price.or(base_fee);
        }
        return;
    }

    if let Some(mut max_fee) = tx.max_fee_per_gas {
        let priority_fee = tx
            .max_priority_fee_per_gas
            .or(config.max_priority_fee_per_gas)
            .unwrap_or(0);
        if let Some(base_for_min) = base_fee.or(config.gas_price) {
            let min_required = base_for_min.saturating_add(priority_fee);
            if max_fee < min_required {
                max_fee = min_required;
            }
        }
        tx.max_priority_fee_per_gas = Some(priority_fee);
        tx.max_fee_per_gas = Some(max_fee);
        tx.gas_price = None;
        return;
    }

    if let Some(price) = config.gas_price {
        tx.gas_price = Some(price);
        tx.max_fee_per_gas = None;
        tx.max_priority_fee_per_gas = None;
        return;
    }

    if let Some(base_for_min) = base_fee {
        let priority_fee = tx
            .max_priority_fee_per_gas
            .or(config.max_priority_fee_per_gas)
            .unwrap_or(0);

        let mut max_fee = config
            .max_fee_per_gas
            .unwrap_or(base_for_min.saturating_add(priority_fee));
        let min_required = base_for_min.saturating_add(priority_fee);
        if max_fee < min_required {
            max_fee = min_required;
        }

        tx.gas_price = None;
        tx.max_priority_fee_per_gas = Some(priority_fee);
        tx.max_fee_per_gas = Some(max_fee);
    }
}

pub(super) fn override_prior_gas_price_with_header(
    base_fee: Option<u128>,
    prior_tx: &ProcessedTransaction,
    unsigned_tx: &mut UnsignedTransaction,
) {
    if !matches!(prior_tx.raw_tx_type, 0 | 1) {
        return;
    }
    if let Some(base_fee) = base_fee {
        unsigned_tx.gas_price = Some(base_fee);
        // legacy transactions should not have EIP-1559 fields
        unsigned_tx.max_fee_per_gas = None;
        unsigned_tx.max_priority_fee_per_gas = None;
    }
}
