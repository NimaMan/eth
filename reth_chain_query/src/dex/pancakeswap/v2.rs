//! PancakeSwap V2 pool utilities for Ethereum mainnet.

use crate::dex::common::{compute_create2_address, sort_tokens};
use crate::dex::encoding::encode_function_call;
use crate::TxSimulator;
use alloy_primitives::{address, Address};
use eyre::{eyre, Result};

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
}
