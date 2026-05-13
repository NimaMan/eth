use alloy_primitives::{Bytes, U256};

/// Decode a uint256 value from contract method output bytes.
pub fn decode_uint256_from_contract_output(output: &Bytes) -> U256 {
    if output.len() >= 32 {
        U256::from_be_slice(&output[..32])
    } else {
        U256::ZERO
    }
}

/// Decode a uint8 value from contract method output bytes (e.g., decimals).
pub fn decode_uint8_from_contract_output(output: &Bytes) -> u8 {
    if output.len() >= 32 {
        output[31]
    } else {
        0
    }
}

/// Decode a string value from contract method output bytes (e.g., name, symbol).
pub fn decode_string_from_contract_output(output: &Bytes) -> String {
    if output.len() < 64 {
        return String::new();
    }

    let len_bytes = &output[32..64];
    let len = U256::from_be_slice(len_bytes).to::<usize>();

    if output.len() < 64 + len {
        return String::new();
    }

    let string_bytes = &output[64..64 + len];
    String::from_utf8_lossy(string_bytes).to_string()
}
