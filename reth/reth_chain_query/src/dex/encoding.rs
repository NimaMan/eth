//! Lightweight ABI encoding helpers shared by DEX modules.

use alloy_primitives::{Address, Bytes, U256};

/// Simple ABI encoding helper for function calls.
pub fn encode_function_call(selector: [u8; 4], params: &[u8]) -> Bytes {
    let mut encoded = Vec::with_capacity(4 + params.len());
    encoded.extend_from_slice(&selector);
    encoded.extend_from_slice(params);
    Bytes::from(encoded)
}

/// Encode two addresses as ABI parameters (each padded to 32 bytes).
pub fn encode_two_addresses(addr1: Address, addr2: Address) -> Vec<u8> {
    let mut params = Vec::with_capacity(64);
    params.extend_from_slice(&[0u8; 12]);
    params.extend_from_slice(addr1.as_slice());
    params.extend_from_slice(&[0u8; 12]);
    params.extend_from_slice(addr2.as_slice());
    params
}

/// Encode two addresses and a uint256 value as ABI parameters.
pub fn encode_two_addresses_and_uint256(addr1: Address, addr2: Address, value: U256) -> Vec<u8> {
    let mut params = Vec::with_capacity(96);
    params.extend_from_slice(&[0u8; 12]);
    params.extend_from_slice(addr1.as_slice());
    params.extend_from_slice(&[0u8; 12]);
    params.extend_from_slice(addr2.as_slice());
    let value_bytes = value.to_be_bytes::<32>();
    params.extend_from_slice(&value_bytes);
    params
}

/// Encode a bytes32 value as ABI parameter.
pub fn encode_bytes32(value: [u8; 32]) -> Vec<u8> {
    value.to_vec()
}
