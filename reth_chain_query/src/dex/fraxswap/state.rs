//! Utilities for interacting with Fraxswap V2 pools on Ethereum.

use crate::dex::common::{compute_create2_address, sort_tokens};
use crate::dex::encoding::encode_function_call;
use crate::TxSimulator;
use alloy_primitives::{address, Address};
use eyre::{eyre, Result};

/// Fraxswap V2 factory address on Ethereum mainnet.
pub const FRAXSWAP_V2_FACTORY: Address = address!("43eC799eAdd63848443E2347C49f5f52e8Fe0F6f");

/// Fraxswap V2 init code hash on Ethereum mainnet.
/// Verified against on-chain factory `PairCreated` traces.
pub const FRAXSWAP_V2_INIT_CODE_HASH: [u8; 32] =
    hex_literal::hex!("4ce0b4ab368f39e4bd03ec712dfc405eb5a36cdb0294b3887b441cd1c743ced3");

const FRAXSWAP_V2_FACTORY_GET_PAIR: [u8; 4] = [0xe6, 0xa4, 0x39, 0x05];

/// Compute Fraxswap V2 pool address deterministically.
pub fn compute_fraxswap_v2_pool(token_a: Address, token_b: Address) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(
        FRAXSWAP_V2_FACTORY,
        token0,
        token1,
        FRAXSWAP_V2_INIT_CODE_HASH,
    )
}

/// Query Fraxswap V2 factory for a token pair using a view call.
pub async fn fetch_fraxswap_v2_pair_address(
    simulator: &TxSimulator,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Address> {
    let params = crate::dex::encoding::encode_two_addresses(token_a, token_b);
    let call_data = encode_function_call(FRAXSWAP_V2_FACTORY_GET_PAIR, &params);

    let response = simulator
        .simulate_view_function(FRAXSWAP_V2_FACTORY, call_data, block_number)
        .await?;

    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "Fraxswap V2 factory getPair reverted (token_a={}, token_b={}, block={:?}; {})",
            token_a,
            token_b,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "Fraxswap V2 factory getPair returned empty output (token_a={}, token_b={}, block={:?})",
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
    fn computes_fraxswap_v2_pool_for_weth_usdc() {
        // Verified against on-chain Fraxswap V2 pool on Ethereum mainnet.
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        let expected = address!("71fd63d6f70bfa901561c3c5240b3d999b899d27");

        let computed = compute_fraxswap_v2_pool(weth, usdc);
        assert_eq!(
            computed, expected,
            "Fraxswap V2 init code hash may be incorrect for Ethereum"
        );
    }
}
