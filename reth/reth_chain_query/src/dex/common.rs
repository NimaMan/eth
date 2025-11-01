//! Shared helpers for DEX modules.

use alloy_primitives::{keccak256, Address};

/// Sort two token addresses to match the `token0`/`token1` ordering used by
/// V2/V3-style AMMs.
pub fn sort_tokens(token_a: Address, token_b: Address) -> (Address, Address) {
    if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    }
}

/// Generic CREATE2 address calculation for V2-style AMMs.
pub fn compute_create2_address(
    factory: Address,
    token0: Address,
    token1: Address,
    init_code_hash: [u8; 32],
) -> Address {
    let mut salt_input = Vec::with_capacity(40);
    salt_input.extend_from_slice(token0.as_slice());
    salt_input.extend_from_slice(token1.as_slice());
    let salt = keccak256(&salt_input);

    let mut input = Vec::with_capacity(85);
    input.push(0xff);
    input.extend_from_slice(factory.as_slice());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(&init_code_hash);

    let hash = keccak256(&input);
    Address::from_slice(&hash[12..])
}

/// CREATE2 address calculation for Uniswap V3 pools (fee included in salt).
pub fn compute_uniswap_v3_create2_address(
    factory: Address,
    token0: Address,
    token1: Address,
    fee: u32,
    init_code_hash: [u8; 32],
) -> Address {
    let mut salt_input = Vec::with_capacity(96);
    salt_input.extend_from_slice(&[0u8; 12]);
    salt_input.extend_from_slice(token0.as_slice());
    salt_input.extend_from_slice(&[0u8; 12]);
    salt_input.extend_from_slice(token1.as_slice());
    salt_input.extend_from_slice(&[0u8; 28]);
    salt_input.extend_from_slice(&fee.to_be_bytes());

    let salt = keccak256(&salt_input);

    let mut input = Vec::with_capacity(85);
    input.push(0xff);
    input.extend_from_slice(factory.as_slice());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(&init_code_hash);

    let hash = keccak256(&input);
    Address::from_slice(&hash[12..])
}
