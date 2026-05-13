use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use serde_json::Value;
use tracing::debug;

use crate::RethQueryProvider;
use tx_simulator::{
    block_context::processed_tx_json::build_unsigned_transaction_from_processed_tx_json,
    UnsignedTransaction, UnsignedTxChainSimulation,
};

/// Replays pending transactions on top of the resolved block so metadata view
/// calls can observe live state even when MDBX has not advanced yet.
pub(super) async fn prepare_state_for_metadata(
    provider: &RethQueryProvider,
    block_number: Option<u64>,
    pending_tx_hashes: &[B256],
) -> Result<Option<UnsignedTxChainSimulation>> {
    if pending_tx_hashes.is_empty() {
        return Ok(None);
    }

    let simulator = provider.simulator();
    let Some(cache) = simulator.live_chain_cache() else {
        return Ok(None);
    };

    let resolved_block = block_number.unwrap_or(simulator.get_latest_block()?);

    let mut chain = simulator
        .start_simulation_chain(Some(resolved_block))
        .await?;

    for hash in pending_tx_hashes {
        let hash_hex = format!("0x{}", hex::encode(hash.as_slice()));
        let Some((candidate_block, tx_value)) =
            find_pending_transaction(&cache, resolved_block, &hash_hex).await?
        else {
            debug!(
                target: "reth_chain_query::erc20",
                block = resolved_block,
                pending_block = resolved_block + 1,
                hash = hash_hex,
                "pending transaction not found in live cache"
            );
            continue;
        };

        let unsigned_tx = build_unsigned_transaction_from_processed_tx_json(&tx_value)?;
        if let (Some(from), Some(nonce)) = (unsigned_tx.from, unsigned_tx.nonce) {
            let previous_nonce = chain.set_account_nonce_for_replay(from, nonce)?;
            debug!(
                target: "reth_chain_query::erc20",
                block = resolved_block,
                pending_block = candidate_block,
                hash = hash_hex,
                %from,
                previous_nonce,
                replay_nonce = nonce,
                "Set account nonce before pending metadata replay"
            );
        }
        debug!(
            target: "reth_chain_query::erc20",
            block = resolved_block,
            pending_block = candidate_block,
            hash = hash_hex,
            "Replaying pending tx before fetching token metadata"
        );
        if let Some(adjustment) = ensure_replay_sender_can_pay(&mut chain, &unsigned_tx)? {
            debug!(
                target: "reth_chain_query::erc20",
                block = resolved_block,
                pending_block = candidate_block,
                hash = hash_hex,
                sender = %adjustment.sender,
                previous_balance = %adjustment.previous_balance,
                replay_balance = %adjustment.replay_balance,
                "funding pending metadata replay sender for validation"
            );
        }
        chain.step(unsigned_tx).await?;
    }

    Ok(Some(chain))
}

async fn find_pending_transaction(
    cache: &tx_simulator::LiveChainCache,
    base_block: u64,
    hash: &str,
) -> Result<Option<(u64, Value)>> {
    let latest_live = cache
        .latest_block_number()
        .await?
        .unwrap_or(base_block.saturating_add(1));

    for candidate in (base_block + 1)..=latest_live {
        if let Some(tx) = cache.find_processed_transaction(candidate, hash).await? {
            return Ok(Some((candidate, tx)));
        }
    }

    Ok(None)
}

fn ensure_replay_sender_can_pay(
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
struct ReplayFundingAdjustment {
    sender: Address,
    previous_balance: U256,
    replay_balance: U256,
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
            gas: Some(30_000),
            gas_price: None,
            max_fee_per_gas: Some(50),
            max_priority_fee_per_gas: Some(2),
            value: Some(U256::from(11)),
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
        assert_eq!(
            required_replay_sender_balance(&unsigned_tx(), None),
            U256::from(1_500_011)
        );
    }

    #[test]
    fn required_replay_sender_balance_uses_block_base_fee_floor() {
        assert_eq!(
            required_replay_sender_balance(&unsigned_tx(), Some(60)),
            U256::from(1_800_011)
        );
    }

    #[test]
    fn required_replay_sender_balance_matches_eip1559_when_eip_fields_are_present() {
        let mut tx = unsigned_tx();
        tx.gas_price = Some(3);
        tx.max_fee_per_gas = Some(50);

        assert_eq!(
            required_replay_sender_balance(&tx, None),
            U256::from(1_500_011)
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
            U256::from(300_011)
        );
    }
}
