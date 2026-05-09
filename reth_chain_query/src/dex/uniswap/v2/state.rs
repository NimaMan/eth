//! Utilities for interacting with Uniswap V2-style pools.

use crate::dex::common::{compute_create2_address, sort_tokens};
use crate::dex::encoding::encode_two_addresses;
use crate::TxSimulator;
use alloy_primitives::{address, Address, Bytes};
use eyre::{eyre, Result};

/// Uniswap V2 factory address.
pub const UNISWAP_V2_FACTORY: Address = address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");

/// Uniswap V2 init code hash (used for CREATE2 address calculation).
pub const UNISWAP_V2_INIT_CODE_HASH: [u8; 32] =
    hex_literal::hex!("96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f");

const UNISWAP_V2_FACTORY_GET_PAIR: [u8; 4] = [0xe6, 0xa4, 0x39, 0x05];

/// Compute Uniswap V2 pool address deterministically.
pub fn compute_uniswap_v2_pool(token_a: Address, token_b: Address) -> Address {
    compute_v2_pool_address(
        UNISWAP_V2_FACTORY,
        UNISWAP_V2_INIT_CODE_HASH,
        token_a,
        token_b,
    )
}

/// Compute a V2-style pool address for an arbitrary factory/init-code pair.
pub fn compute_v2_pool_address(
    factory: Address,
    init_code_hash: [u8; 32],
    token_a: Address,
    token_b: Address,
) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(factory, token0, token1, init_code_hash)
}

/// Query a V2-style factory for a token pair using a view call.
pub async fn fetch_uniswap_v2_pair_address(
    simulator: &TxSimulator,
    factory: Address,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Address> {
    let params = encode_two_addresses(token_a, token_b);
    let mut call_data = Vec::with_capacity(4 + params.len());
    call_data.extend_from_slice(&UNISWAP_V2_FACTORY_GET_PAIR);
    call_data.extend_from_slice(&params);

    let response = simulator
        .simulate_view_function(factory, Bytes::from(call_data), block_number)
        .await?;

    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "V2 factory getPair reverted (factory={}, token_a={}, token_b={}, block={:?}; {})",
            factory,
            token_a,
            token_b,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "V2 factory getPair returned empty output (factory={}, token_a={}, token_b={}, block={:?})",
            factory,
            token_a,
            token_b,
            block_number
        ));
    }

    Ok(Address::from_slice(&response.output[12..32]))
}
