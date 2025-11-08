//! Utilities for interacting with Uniswap V3 pools.

use crate::dex::common::{compute_uniswap_v3_create2_address, sort_tokens};
use crate::dex::encoding::encode_function_call;
use crate::TxSimulator;
use alloy_primitives::{address, Address, U256};
use eyre::{eyre, Result};

/// Uniswap V3 factory address.
pub const UNISWAP_V3_FACTORY: Address = address!("1F98431c8aD98523631AE4a59f267346ea31F984");

/// Uniswap V3 init code hash.
pub const UNISWAP_V3_INIT_CODE_HASH: [u8; 32] =
    hex_literal::hex!("e34f199b19b2b4f47f68442619d555527d244f78a3297ea89325f843f87b8b54");

/// Standard fee tiers for Uniswap V3 (in basis points).
pub const V3_FEE_TIERS: [u32; 4] = [
    100,   // 0.01%
    500,   // 0.05%
    3000,  // 0.30%
    10000, // 1.00%
];

const UNISWAP_V3_FACTORY_GET_POOL: [u8; 4] = [0x16, 0x98, 0xee, 0x82];

/// Compute Uniswap V3 pool address for a specific fee tier.
pub fn compute_uniswap_v3_pool(token_a: Address, token_b: Address, fee_tier: u32) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_uniswap_v3_create2_address(
        UNISWAP_V3_FACTORY,
        token0,
        token1,
        fee_tier,
        UNISWAP_V3_INIT_CODE_HASH,
    )
}

/// Get all possible V3 pools for a token pair (all fee tiers).
pub fn get_all_v3_pools(token_a: Address, token_b: Address) -> Vec<(Address, u32)> {
    V3_FEE_TIERS
        .iter()
        .map(|&fee| (compute_uniswap_v3_pool(token_a, token_b, fee), fee))
        .collect()
}

/// Query Uniswap V3 factory for a token pair & fee tier using a view call.
pub async fn fetch_uniswap_v3_pool_address(
    simulator: &TxSimulator,
    factory: Address,
    token_a: Address,
    token_b: Address,
    fee_tier: u32,
    block_number: Option<u64>,
) -> Result<Address> {
    let params = crate::dex::encoding::encode_two_addresses_and_uint256(
        token_a,
        token_b,
        U256::from(fee_tier),
    );
    let call_data = encode_function_call(UNISWAP_V3_FACTORY_GET_POOL, &params);

    let response = simulator
        .simulate_view_function(factory, call_data, block_number, None)
        .await?;

    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "V3 factory getPool reverted (factory={}, token_a={}, token_b={}, fee={}, block={:?}; {})",
            factory,
            token_a,
            token_b,
            fee_tier,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "V3 factory getPool returned empty output (factory={}, token_a={}, token_b={}, fee={}, block={:?})",
            factory,
            token_a,
            token_b,
            fee_tier,
            block_number
        ));
    }

    Ok(Address::from_slice(&response.output[12..32]))
}
