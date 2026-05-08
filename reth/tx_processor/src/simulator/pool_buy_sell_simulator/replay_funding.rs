use alloy_primitives::{Address, U256};
use eyre::Result;
use tx_simulator::{UnsignedTransaction, UnsignedTxChainSimulation};

pub(super) fn ensure_replay_sender_can_pay(
    chain: &mut UnsignedTxChainSimulation,
    tx: &UnsignedTransaction,
) -> Result<Option<ReplayFundingAdjustment>> {
    let Some(sender) = tx.from else {
        return Ok(None);
    };

    let required_balance = required_replay_sender_balance(tx, chain.block_base_fee());
    if required_balance.is_zero() {
        return Ok(None);
    }

    let current_balance = chain.eth_balance(sender)?;
    if current_balance >= required_balance {
        return Ok(None);
    }

    chain.set_eth_balance(sender, required_balance)?;
    Ok(Some(ReplayFundingAdjustment {
        sender,
        previous_balance: current_balance,
        replay_balance: required_balance,
    }))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct ReplayFundingAdjustment {
    pub sender: Address,
    pub previous_balance: U256,
    pub replay_balance: U256,
}

fn required_replay_sender_balance(tx: &UnsignedTransaction, block_base_fee: Option<u128>) -> U256 {
    let value = tx.value.unwrap_or(U256::ZERO);
    let gas_cost = match (tx.gas, fee_cap_per_gas(tx, block_base_fee)) {
        (Some(gas), Some(fee_cap)) => U256::from(gas)
            .checked_mul(U256::from(fee_cap))
            .unwrap_or(U256::MAX),
        _ => U256::ZERO,
    };

    value.checked_add(gas_cost).unwrap_or(U256::MAX)
}

fn fee_cap_per_gas(tx: &UnsignedTransaction, block_base_fee: Option<u128>) -> Option<u128> {
    let base_fee = block_base_fee.unwrap_or(0);
    let has_eip1559_fee = tx.max_fee_per_gas.is_some() || tx.max_priority_fee_per_gas.is_some();
    if has_eip1559_fee || block_base_fee.is_some() {
        let priority_fee = tx.max_priority_fee_per_gas.unwrap_or(0);
        let requested_max_fee = tx.max_fee_per_gas.unwrap_or(base_fee);
        return Some(requested_max_fee.max(base_fee).max(priority_fee));
    }

    tx.gas_price
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unsigned_tx() -> UnsignedTransaction {
        UnsignedTransaction {
            from: Some(Address::ZERO),
            to: Some(Address::ZERO),
            gas: Some(21_000),
            gas_price: None,
            max_fee_per_gas: Some(100),
            max_priority_fee_per_gas: Some(1),
            value: Some(U256::from(7)),
            data: None,
            nonce: Some(0),
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        }
    }

    #[test]
    fn required_replay_sender_balance_includes_value_and_fee_cap() {
        let tx = unsigned_tx();

        assert_eq!(
            required_replay_sender_balance(&tx, None),
            U256::from(2_100_007)
        );
    }

    #[test]
    fn required_replay_sender_balance_uses_block_base_fee_floor() {
        let tx = unsigned_tx();

        assert_eq!(
            required_replay_sender_balance(&tx, Some(110)),
            U256::from(2_310_007)
        );
    }

    #[test]
    fn required_replay_sender_balance_matches_eip1559_when_eip_fields_are_present() {
        let mut tx = unsigned_tx();
        tx.gas_price = Some(3);
        tx.max_fee_per_gas = Some(100);

        assert_eq!(
            required_replay_sender_balance(&tx, None),
            U256::from(2_100_007)
        );
    }

    #[test]
    fn required_replay_sender_balance_applies_base_fee_floor_to_legacy_replay() {
        let mut tx = unsigned_tx();
        tx.gas_price = Some(3);
        tx.max_fee_per_gas = None;
        tx.max_priority_fee_per_gas = None;

        assert_eq!(
            required_replay_sender_balance(&tx, Some(10)),
            U256::from(210_007)
        );
    }
}
