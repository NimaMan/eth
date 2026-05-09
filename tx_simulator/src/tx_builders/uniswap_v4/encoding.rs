use alloy_primitives::{Address, I256, U256};
use eyre::{eyre, Result};

use super::{UniswapV4PoolKey, MAX_SQRT_RATIO_X96, MIN_SQRT_RATIO_X96};

pub(super) fn encode_constructor_args(pool_manager: Address, weth: Address) -> [u8; 64] {
    let mut args = [0u8; 64];
    args[12..32].copy_from_slice(pool_manager.as_slice());
    args[44..64].copy_from_slice(weth.as_slice());
    args
}

pub(super) fn resolve_currencies(key: &UniswapV4PoolKey, zero_for_one: bool) -> (Address, Address) {
    if zero_for_one {
        (key.currency0, key.currency1)
    } else {
        (key.currency1, key.currency0)
    }
}

pub(super) fn pad_address(address: Address) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[12..].copy_from_slice(address.as_slice());
    buf
}

pub(super) fn pad_bool(value: bool) -> [u8; 32] {
    let mut buf = [0u8; 32];
    if value {
        buf[31] = 1;
    }
    buf
}

pub(super) fn pad_u32(value: u32) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[28..].copy_from_slice(&value.to_be_bytes());
    buf
}

pub(super) fn pad_u64(value: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..].copy_from_slice(&value.to_be_bytes());
    buf
}

pub(super) fn pad_i32(value: i32) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = value.to_be_bytes();
    if value >= 0 {
        buf[28..].copy_from_slice(&bytes);
    } else {
        buf.fill(0xff);
        buf[28..].copy_from_slice(&bytes);
    }
    buf
}

pub(super) fn default_sqrt_price_limit(zero_for_one: bool) -> U256 {
    if zero_for_one {
        MIN_SQRT_RATIO_X96 + U256::from(1u8)
    } else {
        MAX_SQRT_RATIO_X96 - U256::from(1u8)
    }
}

pub(super) fn pad_u128(value: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&value.to_be_bytes());
    buf
}

pub(super) fn pad_i128(value: i128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = value.to_be_bytes();
    if value >= 0 {
        buf[16..].copy_from_slice(&bytes);
    } else {
        buf.fill(0xff);
        buf[16..].copy_from_slice(&bytes);
    }
    buf
}

pub(super) fn pad_i256(value: I256) -> [u8; 32] {
    value.to_be_bytes()
}

pub(super) fn pad_u256(value: U256) -> [u8; 32] {
    value.to_be_bytes::<32>()
}

pub(super) fn u256_to_i128(value: U256) -> Result<i128> {
    let bytes = value.to_be_bytes::<32>();
    if bytes[..16].iter().any(|&b| b != 0) {
        return Err(eyre!("value {value:#x} exceeds i128 range"));
    }
    let mut lower_bytes = [0u8; 16];
    lower_bytes.copy_from_slice(&bytes[16..]);
    let lower = u128::from_be_bytes(lower_bytes);
    if lower > i128::MAX as u128 {
        return Err(eyre!("value {value:#x} exceeds i128 range"));
    }
    Ok(lower as i128)
}

pub(super) fn pad_bytes(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::from(data);
    while out.len() % 32 != 0 {
        out.push(0);
    }
    out
}

/// Compute the CREATE contract address for the deployer + nonce pair.
pub fn compute_contract_address(deployer: Address, nonce: u64) -> Address {
    deployer.create(nonce)
}
