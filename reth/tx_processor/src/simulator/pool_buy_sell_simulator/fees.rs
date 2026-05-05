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

pub(super) fn normalize_prior_fees_with_header(
    base_fee: Option<u128>,
    prior_tx: &ProcessedTransaction,
    unsigned_tx: &mut UnsignedTransaction,
) {
    let Some(base_fee) = base_fee else {
        return;
    };

    if matches!(prior_tx.raw_tx_type, 0 | 1) {
        unsigned_tx.gas_price = Some(base_fee);
        unsigned_tx.max_fee_per_gas = None;
        unsigned_tx.max_priority_fee_per_gas = None;
        return;
    }

    let priority_fee = unsigned_tx.max_priority_fee_per_gas.unwrap_or(0);
    let min_required = base_fee.saturating_add(priority_fee);
    let max_fee = unsigned_tx.max_fee_per_gas.unwrap_or(min_required);

    unsigned_tx.gas_price = None;
    unsigned_tx.max_priority_fee_per_gas = Some(priority_fee);
    unsigned_tx.max_fee_per_gas = Some(max_fee.max(min_required));
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256, U256};

    fn processed_tx(raw_tx_type: u8) -> ProcessedTransaction {
        ProcessedTransaction::new(
            B256::ZERO,
            0,
            0,
            0,
            Address::ZERO,
            Some(Address::ZERO),
            U256::ZERO,
            true,
            0,
            raw_tx_type,
            Vec::new(),
        )
    }

    fn unsigned_tx() -> UnsignedTransaction {
        UnsignedTransaction {
            from: Some(Address::ZERO),
            to: Some(Address::ZERO),
            value: Some(U256::ZERO),
            data: None,
            gas: Some(100_000),
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: Some(0),
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        }
    }

    #[test]
    fn legacy_prior_replay_uses_replay_block_base_fee() {
        let prior_tx = processed_tx(0);
        let mut tx = unsigned_tx();
        tx.gas_price = Some(40);

        normalize_prior_fees_with_header(Some(100), &prior_tx, &mut tx);

        assert_eq!(tx.gas_price, Some(100));
        assert_eq!(tx.max_fee_per_gas, None);
        assert_eq!(tx.max_priority_fee_per_gas, None);
    }

    #[test]
    fn eip1559_prior_replay_clamps_max_fee_to_base_plus_priority() {
        let prior_tx = processed_tx(2);
        let mut tx = unsigned_tx();
        tx.max_fee_per_gas = Some(50);
        tx.max_priority_fee_per_gas = Some(3);

        normalize_prior_fees_with_header(Some(100), &prior_tx, &mut tx);

        assert_eq!(tx.gas_price, None);
        assert_eq!(tx.max_fee_per_gas, Some(103));
        assert_eq!(tx.max_priority_fee_per_gas, Some(3));
    }

    #[test]
    fn eip1559_prior_replay_fills_missing_fee_fields() {
        let prior_tx = processed_tx(2);
        let mut tx = unsigned_tx();

        normalize_prior_fees_with_header(Some(100), &prior_tx, &mut tx);

        assert_eq!(tx.gas_price, None);
        assert_eq!(tx.max_fee_per_gas, Some(100));
        assert_eq!(tx.max_priority_fee_per_gas, Some(0));
    }
}
