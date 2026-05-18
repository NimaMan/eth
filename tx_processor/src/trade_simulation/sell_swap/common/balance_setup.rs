use alloy_primitives::{address, keccak256, Address, Bytes, U256};
use eyre::Result;
use std::collections::HashSet;
use tx_simulator::{UnsignedTransaction, UnsignedTxChainSimulation};

use super::core::{apply_sell_fee_policy, SELLER_ETH_FUND};

const BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];
const TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];
const ERC20_BALANCE_SLOT_SEARCH_LIMIT: u64 = 64;
const TRANSFER_DIFF_DUMMY_RECIPIENT: Address = address!("000000000000000000000000000000000000bEEF");

#[derive(Debug, Clone)]
pub(in crate::trade_simulation::sell_swap) enum TokenBalanceSetup {
    StandardSlot {
        slot: u64,
    },
    RecipientTransferDiff {
        slots_applied: usize,
        observed_balance: U256,
    },
}

pub(in crate::trade_simulation::sell_swap) async fn prepare_seller_token_balance(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
    funding_source: Address,
    seller: Address,
    amount: U256,
    gas_limit: u64,
    base_fee: Option<u128>,
) -> Result<Option<TokenBalanceSetup>> {
    if let Some(slot) = inject_standard_erc20_balance(chain, token_address, seller, amount)? {
        return Ok(Some(TokenBalanceSetup::StandardSlot { slot }));
    }

    inject_recipient_transfer_diff_balance(
        chain,
        token_address,
        funding_source,
        seller,
        amount,
        gas_limit,
        base_fee,
    )
    .await
}

async fn inject_recipient_transfer_diff_balance(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
    funding_source: Address,
    seller: Address,
    amount: U256,
    gas_limit: u64,
    base_fee: Option<u128>,
) -> Result<Option<TokenBalanceSetup>> {
    for transfer_amount in recipient_transfer_gross_amounts(amount) {
        let mut candidate = chain.clone();
        if let Some(setup) = try_recipient_transfer_diff_balance(
            &mut candidate,
            token_address,
            funding_source,
            seller,
            amount,
            transfer_amount,
            gas_limit,
            base_fee,
        )
        .await?
        {
            *chain = candidate;
            return Ok(Some(setup));
        }
    }

    Ok(None)
}

async fn try_recipient_transfer_diff_balance(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
    funding_source: Address,
    seller: Address,
    required_balance: U256,
    transfer_amount: U256,
    gas_limit: u64,
    base_fee: Option<u128>,
) -> Result<Option<TokenBalanceSetup>> {
    let mut seller_probe = chain.clone();
    let mut dummy_probe = chain.clone();
    let gas_fund = U256::from(SELLER_ETH_FUND);
    seller_probe.set_eth_balance(funding_source, gas_fund)?;
    dummy_probe.set_eth_balance(funding_source, gas_fund)?;
    seller_probe.set_account_as_eoa_for_replay(funding_source)?;
    dummy_probe.set_account_as_eoa_for_replay(funding_source)?;

    let seller_transfer = build_erc20_transfer_tx(
        funding_source,
        token_address,
        seller,
        transfer_amount,
        gas_limit,
        base_fee,
    );
    let seller_result = seller_probe.step_with_trace(seller_transfer).await?;
    if !seller_result.success {
        tracing::debug!(
            token = %token_address,
            source = %funding_source,
            seller = %seller,
            transfer_amount = %transfer_amount,
            revert = ?seller_result.revert_reason,
            "recipient-transfer balance setup failed for seller probe"
        );
        return Ok(None);
    }

    let dummy_transfer = build_erc20_transfer_tx(
        funding_source,
        token_address,
        TRANSFER_DIFF_DUMMY_RECIPIENT,
        transfer_amount,
        gas_limit,
        base_fee,
    );
    let dummy_result = dummy_probe.step_with_trace(dummy_transfer).await?;
    if !dummy_result.success {
        tracing::debug!(
            token = %token_address,
            source = %funding_source,
            dummy = %TRANSFER_DIFF_DUMMY_RECIPIENT,
            transfer_amount = %transfer_amount,
            revert = ?dummy_result.revert_reason,
            "recipient-transfer balance setup failed for dummy probe"
        );
        return Ok(None);
    }

    let dummy_slots = dummy_probe
        .cached_account_storage(token_address)
        .into_iter()
        .map(|(slot, _)| slot)
        .collect::<HashSet<_>>();

    let mut applied_slots = Vec::new();
    for (slot, value) in seller_probe.cached_account_storage(token_address) {
        let original = chain.account_storage(token_address, slot)?;
        if original == value {
            continue;
        }
        chain.set_account_storage(token_address, slot, value)?;
        applied_slots.push((slot, original, value, dummy_slots.contains(&slot), true));
    }

    if applied_slots.is_empty() {
        return Ok(None);
    }

    for (slot, original, value, common_with_dummy, keep_applied) in &mut applied_slots {
        if !*common_with_dummy {
            continue;
        }
        chain.set_account_storage(token_address, *slot, *original)?;
        let observed_balance = read_erc20_balance(chain, token_address, seller)?;
        if observed_balance >= required_balance {
            *keep_applied = false;
        } else {
            chain.set_account_storage(token_address, *slot, *value)?;
        }
    }

    let observed_balance = read_erc20_balance(chain, token_address, seller)?;
    if observed_balance < required_balance {
        let slots_applied = applied_slots
            .iter()
            .filter(|(_, _, _, _, keep)| *keep)
            .count();
        tracing::debug!(
            token = %token_address,
            seller = %seller,
            required_balance = %required_balance,
            transfer_amount = %transfer_amount,
            observed_balance = %observed_balance,
            slots_applied,
            "recipient-transfer balance setup did not prove requested seller balance"
        );
        return Ok(None);
    }

    let slots_applied = applied_slots
        .iter()
        .filter(|(_, _, _, _, keep)| *keep)
        .count();
    Ok(Some(TokenBalanceSetup::RecipientTransferDiff {
        slots_applied,
        observed_balance,
    }))
}

fn recipient_transfer_gross_amounts(amount: U256) -> Vec<U256> {
    [
        (1u64, 1u64),
        (101, 100),
        (105, 100),
        (110, 100),
        (125, 100),
        (150, 100),
        (2, 1),
    ]
    .into_iter()
    .map(|(numerator, denominator)| {
        amount.saturating_mul(U256::from(numerator)) / U256::from(denominator)
    })
    .collect()
}

fn build_erc20_transfer_tx(
    from: Address,
    token_address: Address,
    to: Address,
    amount: U256,
    gas_limit: u64,
    base_fee: Option<u128>,
) -> UnsignedTransaction {
    let mut calldata = Vec::with_capacity(68);
    calldata.extend_from_slice(&TRANSFER_SELECTOR);
    calldata.extend_from_slice(to.into_word().as_slice());
    calldata.extend_from_slice(&amount.to_be_bytes::<32>());
    let mut tx = UnsignedTransaction {
        from: Some(from),
        to: Some(token_address),
        gas: Some(gas_limit),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(calldata)),
        nonce: None,
        ..Default::default()
    };
    apply_sell_fee_policy(&mut tx, gas_limit, base_fee);
    tx
}

pub(in crate::trade_simulation::sell_swap) fn log_token_balance_setup(
    setup: &TokenBalanceSetup,
    token: Address,
    seller: Address,
    amount: U256,
    context: &'static str,
) {
    match setup {
        TokenBalanceSetup::StandardSlot { slot } => {
            tracing::debug!(
                token = %token,
                seller = %seller,
                balance_slot = *slot,
                amount = %amount,
                context,
                "injected synthetic ERC20 balance"
            );
        }
        TokenBalanceSetup::RecipientTransferDiff {
            slots_applied,
            observed_balance,
        } => {
            tracing::debug!(
                token = %token,
                seller = %seller,
                slots_applied,
                amount = %amount,
                observed_balance = %observed_balance,
                context,
                "injected ERC20 balance from recipient transfer storage diff"
            );
        }
    }
}

fn inject_standard_erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
    owner: Address,
    amount: U256,
) -> Result<Option<u64>> {
    for slot in 0..ERC20_BALANCE_SLOT_SEARCH_LIMIT {
        let storage_key = standard_erc20_balance_storage_key(owner, slot);
        let previous = chain.set_account_storage(token_address, storage_key, amount)?;
        let observed = read_erc20_balance(chain, token_address, owner)?;
        if observed == amount {
            return Ok(Some(slot));
        }
        chain.set_account_storage(token_address, storage_key, previous)?;
    }
    Ok(None)
}

fn standard_erc20_balance_storage_key(owner: Address, slot: u64) -> U256 {
    let mut input = [0u8; 64];
    input[..32].copy_from_slice(owner.into_word().as_slice());
    input[32..].copy_from_slice(&U256::from(slot).to_be_bytes::<32>());
    U256::from_be_slice(keccak256(input).as_slice())
}

fn read_erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
    owner: Address,
) -> Result<U256> {
    let mut calldata = Vec::with_capacity(36);
    calldata.extend_from_slice(&BALANCE_OF_SELECTOR);
    calldata.extend_from_slice(owner.into_word().as_slice());
    let result = chain.simulate_view_call(token_address, Bytes::from(calldata))?;
    if !result.success || result.output.len() < 32 {
        return Ok(U256::ZERO);
    }
    Ok(U256::from_be_slice(&result.output[..32]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_transfer_gross_amounts_include_tax_buffers() {
        let amount = U256::from(1_000_000u64);
        let amounts = recipient_transfer_gross_amounts(amount);

        assert_eq!(amounts[0], amount);
        assert!(amounts.contains(&U256::from(1_050_000u64)));
        assert_eq!(amounts.last().copied(), Some(U256::from(2_000_000u64)));
        assert!(amounts.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
