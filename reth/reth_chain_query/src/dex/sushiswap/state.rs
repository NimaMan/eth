//! Utilities for interacting with SushiSwap V2-style pools.

use crate::dex::common::{compute_create2_address, sort_tokens};
use crate::dex::encoding::encode_two_addresses;
use crate::TxSimulator;
use alloy_primitives::{address, Address, Bytes};
use eyre::{eyre, Result};

/// SushiSwap V2 factory address.
pub const SUSHISWAP_FACTORY: Address = address!("C0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac");

/// SushiSwap init code hash (CREATE2 salt for pool computation).
pub const SUSHISWAP_INIT_CODE_HASH: [u8; 32] =
    hex_literal::hex!("e18a34eb0e04b04f7a0ac29a6e80748dca96319b42c54d679cb821dca90c6303");

const SUSHISWAP_FACTORY_GET_PAIR: [u8; 4] = [0xe6, 0xa4, 0x39, 0x05];

/// Compute SushiSwap pool address (same algorithm as Uniswap V2).
pub fn compute_sushiswap_pool(token_a: Address, token_b: Address) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(SUSHISWAP_FACTORY, token0, token1, SUSHISWAP_INIT_CODE_HASH)
}

/// Query SushiSwap factory for a token pair using a view call.
pub async fn fetch_sushiswap_pair_address(
    simulator: &TxSimulator,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Address> {
    let params = encode_two_addresses(token_a, token_b);
    let mut call_data = Vec::with_capacity(4 + params.len());
    call_data.extend_from_slice(&SUSHISWAP_FACTORY_GET_PAIR);
    call_data.extend_from_slice(&params);

    let response = simulator
        .simulate_view_function(SUSHISWAP_FACTORY, Bytes::from(call_data), block_number)
        .await?;

    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "SushiSwap factory getPair reverted (token_a={}, token_b={}, block={:?}; {})",
            token_a,
            token_b,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "SushiSwap factory getPair returned empty output (token_a={}, token_b={}, block={:?})",
            token_a,
            token_b,
            block_number
        ));
    }

    Ok(Address::from_slice(&response.output[12..32]))
}
