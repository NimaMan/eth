//! Curve pool discovery helpers.

use crate::dex::encoding::{encode_function_call, encode_two_addresses_and_uint256};
use crate::TxSimulator;
use alloy_primitives::{address, Address, U256};
use eyre::Result;

/// Curve Registry contract address (for dynamic pool discovery).
pub const CURVE_REGISTRY: Address = address!("90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5");

const CURVE_FIND_POOL_FOR_COINS: [u8; 4] = [0x69, 0x82, 0xeb, 0x0b];

/// Find Curve pool for token pair using registry contract.
/// Returns `Ok(Some(pool))` if a pool exists, `Ok(None)` otherwise.
pub async fn find_curve_pool_for_coins(
    simulator: &TxSimulator,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<Option<Address>> {
    let params = encode_two_addresses_and_uint256(token_a, token_b, U256::ZERO);
    let call_data = encode_function_call(CURVE_FIND_POOL_FOR_COINS, &params);

    let result = simulator
        .simulate_view_function(CURVE_REGISTRY, call_data, block_number)
        .await?;

    if result.success && result.output.len() >= 32 {
        let pool_address = Address::from_slice(&result.output[12..32]);
        if pool_address != Address::ZERO {
            return Ok(Some(pool_address));
        }
    }

    Ok(None)
}
