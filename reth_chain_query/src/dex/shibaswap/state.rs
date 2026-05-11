//! Utilities for interacting with ShibaSwap V2 pools on Ethereum.

use crate::dex::common::{compute_create2_address, sort_tokens};
use crate::dex::encoding::encode_function_call;
use crate::TxSimulator;
use alloy_primitives::{address, Address};
use eyre::{eyre, Result};

/// ShibaSwap V2 factory address on Ethereum mainnet.
pub const SHIBASWAP_V2_FACTORY: Address = address!("115934131916C8b277DD010Ee02de363c09d037c");

/// ShibaSwap V2 init code hash on Ethereum mainnet.
/// Verified against on-chain factory `PairCreated` traces.
pub const SHIBASWAP_V2_INIT_CODE_HASH: [u8; 32] =
    hex_literal::hex!("65d1a3b1e46c6e4f1be1ad5f99ef14dc488ae0549dc97db9b30afe2241ce1c7a");

const SHIBASWAP_V2_FACTORY_GET_PAIR: [u8; 4] = [0xe6, 0xa4, 0x39, 0x05];

/// Compute ShibaSwap V2 pool address deterministically.
pub fn compute_shibaswap_v2_pool(token_a: Address, token_b: Address) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(SHIBASWAP_V2_FACTORY, token0, token1, SHIBASWAP_V2_INIT_CODE_HASH)
}

/// Query ShibaSwap V2 factory for a token pair using a view call.
pub async fn fetch_shibaswap_v2_pair_address(
    simulator: &TxSimulator,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Address> {
    let params = crate::dex::encoding::encode_two_addresses(token_a, token_b);
    let call_data = encode_function_call(SHIBASWAP_V2_FACTORY_GET_PAIR, &params);

    let response = simulator
        .simulate_view_function(SHIBASWAP_V2_FACTORY, call_data, block_number)
        .await?;

    if !response.success {
        let revert_data = if response.output.is_empty() {
            "no revert data".to_string()
        } else {
            format!("revert data 0x{}", hex::encode(&response.output))
        };
        return Err(eyre!(
            "ShibaSwap V2 factory getPair reverted (token_a={}, token_b={}, block={:?}; {})",
            token_a,
            token_b,
            block_number,
            revert_data
        ));
    }

    if response.output.len() < 32 {
        return Err(eyre!(
            "ShibaSwap V2 factory getPair returned empty output (token_a={}, token_b={}, block={:?})",
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
    fn computes_shibaswap_v2_pool_for_weth_usdc() {
        // Verified against on-chain ShibaSwap V2 pool on Ethereum mainnet.
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        let expected = address!("20e95253e54490d8d30ea41574b24f741ee70201");

        let computed = compute_shibaswap_v2_pool(weth, usdc);
        assert_eq!(
            computed, expected,
            "ShibaSwap V2 init code hash may be incorrect for Ethereum"
        );
    }
}
