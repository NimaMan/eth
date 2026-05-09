use crate::UnsignedTransaction;
use alloy_primitives::{Address, Bytes, U256};
use eyre::{eyre, Result};

const WORD_BYTES: usize = 32;
const PERMIT2_APPROVE_SELECTOR: [u8; 4] = [0x87, 0x51, 0x7c, 0x45];

/// Build a Permit2 `approve(address token,address spender,uint160 amount,uint48 expiration)` tx.
pub fn build_permit2_approve_tx(
    owner: Address,
    permit2: Address,
    token: Address,
    spender: Address,
    amount: U256,
    expiration: u64,
) -> Result<UnsignedTransaction> {
    if amount > max_uint160() {
        return Err(eyre!("Permit2 allowance amount must fit uint160"));
    }
    if expiration > max_uint48() {
        return Err(eyre!("Permit2 expiration must fit uint48"));
    }

    let mut data = Vec::with_capacity(4 + WORD_BYTES * 4);
    data.extend_from_slice(&PERMIT2_APPROVE_SELECTOR);
    data.extend_from_slice(&pad_address(token));
    data.extend_from_slice(&pad_address(spender));
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(expiration).to_be_bytes::<32>());

    Ok(UnsignedTransaction {
        from: Some(owner),
        to: Some(permit2),
        gas: Some(120_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
    })
}

fn pad_address(address: Address) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[12..].copy_from_slice(address.as_slice());
    buf
}

fn max_uint160() -> U256 {
    (U256::from(1u8) << 160) - U256::from(1u8)
}

fn max_uint48() -> u64 {
    (1u64 << 48) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permit2_approve_tx_uses_allowance_transfer_selector() {
        let tx = build_permit2_approve_tx(
            Address::with_last_byte(1),
            Address::with_last_byte(2),
            Address::with_last_byte(3),
            Address::with_last_byte(4),
            U256::from(100),
            1234,
        )
        .unwrap();
        let data = tx.data.unwrap();

        assert_eq!(&data[..4], PERMIT2_APPROVE_SELECTOR);
        assert_eq!(tx.value, Some(U256::ZERO));
    }
}
