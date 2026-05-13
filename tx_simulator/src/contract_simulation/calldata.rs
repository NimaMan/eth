use alloy_primitives::{Address, Bytes};

/// Encode a contract read-only call with no arguments (just 4-byte selector).
pub fn encode_contract_read_call_no_args(selector: [u8; 4]) -> Bytes {
    Bytes::from(selector.to_vec())
}

/// Encode a contract read-only call with a single address argument (e.g., balanceOf).
pub fn encode_contract_read_call_with_address_arg(selector: [u8; 4], address: Address) -> Bytes {
    let mut data = selector.to_vec();
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(address.as_ref());
    data.extend_from_slice(&padded);
    Bytes::from(data)
}
