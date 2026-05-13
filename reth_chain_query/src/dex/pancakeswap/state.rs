//! Utilities for interacting with PancakeSwap V2/V3 pools on Ethereum.

use crate::dex::common::{
    compute_create2_address, compute_uniswap_v3_create2_address, sort_tokens,
};
use crate::dex::encoding::encode_function_call;
use crate::TxSimulator;
use alloy_primitives::{address, Address, U256};
use eyre::{eyre, Result};

// ─────────────────────────────────────────────────────────────────────────────
// PancakeSwap V2
// ─────────────────────────────────────────────────────────────────────────────

/// PancakeSwap V2 factory address on Ethereum mainnet.
pub const PANCAKESWAP_V2_FACTORY: Address = address!("1097053Fd2ea711dad45caCcc45EfF7548fCB362");

/// PancakeSwap V2 init code hash on Ethereum mainnet (CREATE2 salt for pool computation).
/// Verified against on-chain factory `INIT_CODE_PAIR_HASH` at
/// `0x1097053Fd2ea711dad45caCcc45EfF7548fCB362`.
pub const PANCAKESWAP_V2_INIT_CODE_HASH: [u8; 32] =
    hex_literal::hex!("57224589c67f3f30a6b0d7a1b54cf3153ab84563bc609ef41dfb34f8b2974d2d");

const PANCAKESWAP_V2_FACTORY_GET_PAIR: [u8; 4] = [0xe6, 0xa4, 0x39, 0x05];

/// Compute PancakeSwap V2 pool address deterministically.
pub fn compute_pancakeswap_v2_pool(token_a: Address, token_b: Address) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(
        PANCAKESWAP_V2_FACTORY,
        token0,
        token1,
        PANCAKESWAP_V2_INIT_CODE_HASH,
    )
}

/// Query PancakeSwap V2 factory for a token pair using a view call.
pub async fn fetch_pancakeswap_v2_pair_address(
    simulator: &TxSimulator,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Address> {
    let params = crate::dex::encoding::encode_two_addresses(token_a, token_b);
    let call_data = encode_function_call(PANCAKESWAP_V2_FACTORY_GET_PAIR, &params);

    let response = simulator
        .simulate_view_function(PANCAKESWAP_V2_FACTORY, call_data, block_number)
        .await?;

    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "PancakeSwap V2 factory getPair reverted (token_a={}, token_b={}, block={:?}; {})",
            token_a,
            token_b,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "PancakeSwap V2 factory getPair returned empty output (token_a={}, token_b={}, block={:?})",
            token_a,
            token_b,
            block_number
        ));
    }

    Ok(Address::from_slice(&response.output[12..32]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn computes_pancakeswap_v2_pool_for_chain_weth() {
        // Verified against on-chain PancakeSwap V2 pool on Ethereum mainnet:
        // https://dexscreener.com/ethereum/0x2997a394e02c46a2d00eb9a004d0145d79c242cc
        let chain = address!("0f9f5E9b76AA03e9Ab1dbd76223FC70A322b55Ad");
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let expected = address!("2997a394e02c46A2D00Eb9A004d0145d79c242cc");

        let computed = compute_pancakeswap_v2_pool(chain, weth);
        assert_eq!(
            computed, expected,
            "PancakeSwap V2 init code hash may be incorrect for Ethereum"
        );
    }

    #[test]
    fn computes_pancakeswap_v3_pool_for_usdc_weth() {
        // Verified against on-chain PancakeSwap V3 factory getPool(USDC, WETH, 500):
        // Factory: 0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865
        // Pool:    0x1ac1a8feaaea1900c4166deeed0c11cc10669d36
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let expected = address!("1ac1a8feaaea1900c4166deeed0c11cc10669d36");

        let computed = compute_pancakeswap_v3_pool(usdc, weth, 500);
        assert_eq!(
            computed, expected,
            "PancakeSwap V3 init code hash may be incorrect for Ethereum"
        );
    }

    #[test]
    fn computes_pancakeswap_v3_pool_for_hash_usdt() {
        // Verified against on-chain PancakeSwap V3 pool on Ethereum mainnet:
        // https://dexscreener.com/ethereum/0x1645ca2363ff04fddc6c8b0e8be1c3f773fe6a0d
        let hash_token = address!("AC7b5d06fa1e77D08aea40d46cb7C5923A87A0cc");
        let usdt = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
        let expected = address!("1645ca2363fF04fDDc6C8B0e8bE1C3f773Fe6A0d");

        let computed = compute_pancakeswap_v3_pool(hash_token, usdt, 10000);
        assert_eq!(
            computed, expected,
            "PancakeSwap V3 pool computation failed for HASH/USDT 1% tier"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PancakeSwap V3
// ─────────────────────────────────────────────────────────────────────────────

/// PancakeSwap V3 factory address on Ethereum mainnet.
pub const PANCAKESWAP_V3_FACTORY: Address = address!("0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865");

/// PancakeSwap V3 pool deployer address on Ethereum mainnet.
/// NOTE: Unlike Uniswap V3, PancakeSwap V3 delegates pool creation to a
/// separate deployer contract. The CREATE2 call is made from the deployer,
/// so pool address computation must use the deployer address, not the factory.
pub const PANCAKESWAP_V3_DEPLOYER: Address = address!("41ff9AA7e16B8B1a8a8dc4f0eFacd93D02d071c9");

/// PancakeSwap V3 pool init code hash.
pub const PANCAKESWAP_V3_INIT_CODE_HASH: [u8; 32] =
    hex_literal::hex!("6ce8eb472fa82df5469c6ab6d485f17c3ad13c8cd7af59b3d4a8026c5ce0f7e2");

/// Standard fee tiers for PancakeSwap V3 (in basis points).
pub const PANCAKESWAP_V3_FEE_TIERS: [u32; 4] = [
    100,   // 0.01%
    500,   // 0.05%
    2500,  // 0.25%
    10000, // 1.00%
];

const PANCAKESWAP_V3_FACTORY_GET_POOL: [u8; 4] = [0x16, 0x98, 0xee, 0x82];

/// Compute PancakeSwap V3 pool address for a specific fee tier.
pub fn compute_pancakeswap_v3_pool(token_a: Address, token_b: Address, fee_tier: u32) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_uniswap_v3_create2_address(
        PANCAKESWAP_V3_DEPLOYER,
        token0,
        token1,
        fee_tier,
        PANCAKESWAP_V3_INIT_CODE_HASH,
    )
}

/// Get all possible PancakeSwap V3 pools for a token pair (all fee tiers).
pub fn get_all_pancakeswap_v3_pools(token_a: Address, token_b: Address) -> Vec<(Address, u32)> {
    PANCAKESWAP_V3_FEE_TIERS
        .iter()
        .map(|&fee| (compute_pancakeswap_v3_pool(token_a, token_b, fee), fee))
        .collect()
}

/// Query PancakeSwap V3 factory for a token pair & fee tier using a view call.
pub async fn fetch_pancakeswap_v3_pool_address(
    simulator: &TxSimulator,
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
    let call_data = encode_function_call(PANCAKESWAP_V3_FACTORY_GET_POOL, &params);

    let response = simulator
        .simulate_view_function(PANCAKESWAP_V3_FACTORY, call_data, block_number)
        .await?;

    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "PancakeSwap V3 factory getPool reverted (token_a={}, token_b={}, fee={}, block={:?}; {})",
            token_a,
            token_b,
            fee_tier,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "PancakeSwap V3 factory getPool returned empty output (token_a={}, token_b={}, fee={}, block={:?})",
            token_a,
            token_b,
            fee_tier,
            block_number
        ));
    }

    Ok(Address::from_slice(&response.output[12..32]))
}
