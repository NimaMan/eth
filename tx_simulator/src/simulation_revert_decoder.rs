/// Simulation Revert Decoder
///
/// Decodes EVM revert data into human-readable error messages.
/// Handles standard Solidity reverts and common DeFi protocol errors.
use alloy_primitives::Bytes;
use std::convert::TryFrom;

use crate::types::RevertContext;

/// Decode revert data from EVM execution into a human-readable message
pub fn decode_revert_data(revert_data: &Bytes) -> String {
    if revert_data.is_empty() {
        return "Empty revert payload".to_string();
    }

    // Convert to hex string for processing
    let hex_str = format!("{:?}", revert_data);
    decode_revert_message(&hex_str)
}

/// Produce a revert reason string, using raw revert data when available and falling back to context.
pub fn decode_revert_reason(
    revert_data: Option<&Bytes>,
    context: Option<&RevertContext>,
) -> Option<String> {
    let base = revert_data.map(|data| {
        if data.is_empty() {
            "Empty revert payload".to_string()
        } else {
            decode_revert_data(data)
        }
    });

    if base.as_deref() != Some("Empty revert payload") {
        return base;
    }

    if let Some(ctx) = context {
        if !ctx.has_code {
            return Some(format!(
                "Target contract {} has no bytecode at the execution block",
                ctx.target
            ));
        }
        if ctx.calldata_len <= 4 {
            return Some(
                "Call data missing encoded arguments (only function selector provided)".to_string(),
            );
        }
        return Some(format!(
            "Empty revert payload from target {} (target_has_code=true, calldata_len={} bytes)",
            ctx.target, ctx.calldata_len
        ));
    }

    base
}

/// Decode a revert message from hex string representation
pub fn decode_revert_message(revert_data: &str) -> String {
    // Remove 0x prefix if present
    let data = if revert_data.starts_with("0x") {
        &revert_data[2..]
    } else {
        revert_data
    };

    // Need at least 4 bytes for selector
    if data.len() < 8 {
        return format!("Unknown revert: {}", revert_data);
    }

    let selector = &data[0..8];

    match selector {
        // Error(string) - most common Solidity revert
        "08c379a0" => decode_error_string(&data[8..]),

        // Panic(uint256) - Solidity panic codes
        "4e487b71" => decode_panic_code(&data[8..]),

        // Common UniswapV2 errors
        "dcee4fac" => "UniswapV2: INSUFFICIENT_OUTPUT_AMOUNT".to_string(),
        "bb55fd27" => "UniswapV2: INSUFFICIENT_LIQUIDITY".to_string(),
        "5b28fd91" => "UniswapV2: INSUFFICIENT_INPUT_AMOUNT".to_string(),
        "ddca3f43" => "UniswapV2: INVALID_PATH".to_string(),
        "25d0209c" => "UniswapV2: EXPIRED".to_string(),

        // UniswapV3 errors (short error codes)
        "773a6187" => "V3: AS".to_string(),  // Amount Slippage
        "904b3c4d" => "V3: LOK".to_string(), // Locked (reentrancy)
        "b4fa3fb3" => "V3: STF".to_string(), // SafeTransferFrom failed
        "b9ec1e96" => "V3: TF".to_string(),  // Transfer failed
        "025dbdd4" => "V3: SPL".to_string(), // Sqrt Price Limit

        // Common ERC20 errors
        "cc2e993e" => "TradingNotEnabled".to_string(),
        "02ce728f" => "TradingDisabled".to_string(),
        "8a81d3b3" => "TransferFailed".to_string(),
        "7939f424" => "TransferFromFailed".to_string(),
        "51e3f160" => "TransferHelper: TRANSFER_FROM_FAILED".to_string(),
        "90b8ec18" => "TransferHelper: TRANSFER_FAILED".to_string(),

        // Access control
        "82b42900" => "Unauthorized".to_string(),
        "8e4a23d6" => "Unauthorized()".to_string(),
        "65ed98f3" => "OnlyOwner".to_string(),

        // DEX Router errors
        "08ee9e42" => "InsufficientOutputAmount".to_string(),
        "e995f780" => "InsufficientInputAmount".to_string(),
        "ad3a8b9e" => "ExcessiveInputAmount".to_string(),
        "675cae38" => "InsufficientLiquidity".to_string(),
        "5c7c9124" => "InvalidPath".to_string(),
        "1ab7da6b" => "Expired".to_string(),

        // Safe math errors
        "50df29df" => "SafeMath: subtraction overflow".to_string(),
        "8995290f" => "SafeMath: addition overflow".to_string(),
        "ab143c06" => "SafeMath: multiplication overflow".to_string(),

        // Unknown selector - try to decode as string anyway
        _ => {
            // Some contracts use non-standard selectors, try to decode as string
            let decoded = decode_error_string(&data[8..]);
            if !decoded.starts_with("Failed to decode") {
                return decoded;
            }

            // Return selector for debugging
            format!("Unknown error (0x{})", selector)
        }
    }
}

/// Decode an Error(string) revert (selector: 0x08c379a0)
fn decode_error_string(data: &str) -> String {
    // Skip if not enough data
    if data.len() < 128 {
        return format!("Failed to decode Error(string): insufficient data");
    }

    // ABI encoding for dynamic string:
    // - First 32 bytes (64 hex chars): offset to string data (usually 0x20 = 32)
    // - Next 32 bytes: string length
    // - Remaining: actual string data (padded to 32 bytes)

    // Skip offset (first 64 chars) and get length
    let length_hex = &data[64..128];

    // Parse string length
    let length_u64 = match u64::from_str_radix(length_hex, 16) {
        Ok(len) => len,
        Err(_) => return format!("Failed to decode Error(string): invalid length"),
    };

    let length = match usize::try_from(length_u64) {
        Ok(len) => len,
        Err(_) => {
            return "Failed to decode Error(string): length exceeds platform capacity".to_string()
        }
    };

    // Extract string bytes (each byte is 2 hex chars)
    let string_start: usize = 128;
    let string_hex_len = match length.checked_mul(2) {
        Some(len) => len,
        None => return "Failed to decode Error(string): length too large".to_string(),
    };
    let string_end = match string_start.checked_add(string_hex_len) {
        Some(end) => end,
        None => return "Failed to decode Error(string): length too large".to_string(),
    };

    if data.len() < string_end {
        return format!("Failed to decode Error(string): string data truncated");
    }

    let string_hex = &data[string_start..string_end];

    // Convert hex to string
    match hex_to_string(string_hex) {
        Ok(s) => s,
        Err(_) => format!("Failed to decode Error(string): invalid string data"),
    }
}

/// Decode a Panic(uint256) revert (selector: 0x4e487b71)
fn decode_panic_code(data: &str) -> String {
    if data.len() < 64 {
        return "Panic: unknown code".to_string();
    }

    let code_hex = &data[0..64];
    let code = match u64::from_str_radix(code_hex, 16) {
        Ok(c) => c,
        Err(_) => return "Panic: invalid code".to_string(),
    };

    let panic_reason = match code {
        0x00 => "Generic compiler inserted panic",
        0x01 => "Assertion failed",
        0x11 => "Arithmetic overflow/underflow",
        0x12 => "Division by zero",
        0x21 => "Invalid enum value",
        0x22 => "Storage byte array incorrectly encoded",
        0x31 => "Pop on empty array",
        0x32 => "Array index out of bounds",
        0x41 => "Too much memory allocated",
        0x51 => "Called invalid internal function",
        _ => "Unknown panic code",
    };

    format!("Panic({}): {}", code, panic_reason)
}

/// Convert hex string to UTF-8 string
fn hex_to_string(hex: &str) -> Result<String, std::string::FromUtf8Error> {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect();

    String::from_utf8(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_error_string() {
        // "Insufficient balance" error
        let revert_data = "0x08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000014496e73756666696369656e742062616c616e6365000000000000000000000000";
        let decoded = decode_revert_message(revert_data);
        assert_eq!(decoded, "Insufficient balance");
    }

    #[test]
    fn test_decode_panic() {
        // Arithmetic overflow (0x11)
        let revert_data =
            "0x4e487b710000000000000000000000000000000000000000000000000000000000000011";
        let decoded = decode_revert_message(revert_data);
        assert_eq!(decoded, "Panic(17): Arithmetic overflow/underflow");
    }

    #[test]
    fn test_decode_uniswap_v3_error() {
        // STF error from V3
        let revert_data = "0xb4fa3fb3";
        let decoded = decode_revert_message(revert_data);
        assert_eq!(decoded, "V3: STF");
    }

    #[test]
    fn test_unknown_selector() {
        let revert_data = "0x12345678aabbccdd";
        let decoded = decode_revert_message(revert_data);
        assert!(decoded.contains("Unknown error"));
        assert!(decoded.contains("0x12345678"));
    }

    #[test]
    fn test_empty_revert() {
        let revert_data = Bytes::new();
        let decoded = decode_revert_data(&revert_data);
        assert_eq!(decoded, "Empty revert payload");
    }
}
