use alloy_primitives::{hex, Address, U256};

const SELECTOR_ERROR_STRING: [u8; 4] = [0x08, 0xc3, 0x79, 0xa0];
const SELECTOR_PANIC: [u8; 4] = [0x4e, 0x48, 0x7b, 0x71];
const SELECTOR_WRAPPED_ERROR: [u8; 4] = [0x90, 0xbf, 0xb8, 0x65];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DecodedRevert {
    pub selector: [u8; 4],
    pub signature: Option<&'static str>,
    pub summary: String,
}

pub fn describe_revert_output(data: &[u8]) -> Option<String> {
    decode_revert_output(data).map(|decoded| decoded.summary)
}

pub fn decode_revert_output(data: &[u8]) -> Option<DecodedRevert> {
    let selector = read_selector(data)?;
    match selector {
        SELECTOR_ERROR_STRING => decode_error_string(data),
        SELECTOR_PANIC => Some(decode_panic(data)),
        SELECTOR_WRAPPED_ERROR => decode_wrapped_error(data),
        _ => Some(DecodedRevert {
            selector,
            signature: known_error_signature(selector),
            summary: describe_unknown_selector(selector),
        }),
    }
}

pub fn known_error_signature(selector: [u8; 4]) -> Option<&'static str> {
    match selector {
        SELECTOR_ERROR_STRING => Some("Error(string)"),
        SELECTOR_PANIC => Some("Panic(uint256)"),
        SELECTOR_WRAPPED_ERROR => Some("WrappedError(address,bytes4,bytes,bytes)"),
        _ => None,
    }
}

pub fn selector_hex(selector: [u8; 4]) -> String {
    format!("0x{}", hex::encode(selector))
}

fn read_selector(data: &[u8]) -> Option<[u8; 4]> {
    data.get(0..4)?.try_into().ok()
}

fn decode_error_string(data: &[u8]) -> Option<DecodedRevert> {
    let message = decode_dynamic_string_argument(data, 0)?;
    Some(DecodedRevert {
        selector: SELECTOR_ERROR_STRING,
        signature: known_error_signature(SELECTOR_ERROR_STRING),
        summary: message,
    })
}

fn decode_panic(data: &[u8]) -> DecodedRevert {
    let code = read_word(data, 0).map(|word| decode_u256(&word));
    let summary = match code {
        None => "panic (no code)".to_string(),
        Some(code) if code == U256::from(0x01u8) => "panic: assertion failed".to_string(),
        Some(code) if code == U256::from(0x11u8) => {
            "panic: arithmetic overflow/underflow".to_string()
        }
        Some(code) if code == U256::from(0x12u8) => "panic: division by zero".to_string(),
        Some(code) if code == U256::from(0x21u8) => "panic: invalid enum value".to_string(),
        Some(code) if code == U256::from(0x31u8) => {
            "panic: storage byte array out-of-bounds".to_string()
        }
        Some(code) if code == U256::from(0x32u8) => "panic: array out-of-bounds".to_string(),
        Some(code) if code == U256::from(0x41u8) => "panic: memory overflow".to_string(),
        Some(code) if code == U256::from(0x51u8) => "panic: pop from empty array".to_string(),
        Some(code) => format!("panic code 0x{code:x}"),
    };
    DecodedRevert {
        selector: SELECTOR_PANIC,
        signature: known_error_signature(SELECTOR_PANIC),
        summary,
    }
}

fn decode_wrapped_error(data: &[u8]) -> Option<DecodedRevert> {
    let target = decode_address(&read_word(data, 0)?);
    let wrapped_selector = decode_bytes4(&read_word(data, 1)?);
    let reason = decode_dynamic_bytes_argument(data, 2).unwrap_or_default();
    let details = decode_dynamic_bytes_argument(data, 3).unwrap_or_default();

    let reason_summary = describe_revert_bytes(&reason);
    let details_summary = describe_revert_bytes(&details);
    let selector_summary = describe_selector(wrapped_selector);

    Some(DecodedRevert {
        selector: SELECTOR_WRAPPED_ERROR,
        signature: known_error_signature(SELECTOR_WRAPPED_ERROR),
        summary: format!(
            "WrappedError(target={target:#x}, selector={selector_summary}, reason={reason_summary}, details={details_summary})"
        ),
    })
}

fn describe_revert_bytes(data: &[u8]) -> String {
    if data.is_empty() {
        return "empty".to_string();
    }
    if let Some(decoded) = decode_revert_output(data) {
        return decoded.summary;
    }
    if let Some(selector) = read_selector(data) {
        return describe_unknown_selector(selector);
    }
    format!("0x{}", hex::encode(data))
}

fn describe_selector(selector: [u8; 4]) -> String {
    match known_error_signature(selector) {
        Some(signature) => format!("{} ({signature})", selector_hex(selector)),
        None => selector_hex(selector),
    }
}

fn describe_unknown_selector(selector: [u8; 4]) -> String {
    format!("unknown custom error selector {}", selector_hex(selector))
}

fn decode_dynamic_string_argument(data: &[u8], arg_index: usize) -> Option<String> {
    let bytes = decode_dynamic_bytes_argument(data, arg_index)?;
    Some(String::from_utf8_lossy(&bytes).to_string())
}

fn decode_dynamic_bytes_argument(data: &[u8], arg_index: usize) -> Option<Vec<u8>> {
    let offset_word = read_word(data, arg_index)?;
    let offset = word_to_usize(&offset_word)?;
    decode_dynamic_bytes_at(data, 4 + offset)
}

fn decode_dynamic_bytes_at(data: &[u8], start: usize) -> Option<Vec<u8>> {
    let len_word: [u8; 32] = data.get(start..start + 32)?.try_into().ok()?;
    let len = word_to_usize(&len_word)?;
    let bytes_start = start + 32;
    let bytes_end = bytes_start.checked_add(len)?;
    Some(data.get(bytes_start..bytes_end)?.to_vec())
}

fn read_word(data: &[u8], index: usize) -> Option<[u8; 32]> {
    let start = 4 + (index * 32);
    let end = start + 32;
    data.get(start..end)?.try_into().ok()
}

fn decode_address(word: &[u8; 32]) -> Address {
    Address::from_slice(&word[12..])
}

fn decode_bytes4(word: &[u8; 32]) -> [u8; 4] {
    word[0..4].try_into().unwrap_or([0u8; 4])
}

fn decode_u256(word: &[u8; 32]) -> U256 {
    U256::from_be_bytes(*word)
}

fn word_to_usize(word: &[u8; 32]) -> Option<usize> {
    let value = U256::from_be_bytes(*word);
    usize::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    fn word_u256(value: usize) -> [u8; 32] {
        let mut word = [0u8; 32];
        let value = value as u128;
        word[16..32].copy_from_slice(&value.to_be_bytes());
        word
    }

    fn word_address(address: Address) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[12..32].copy_from_slice(address.as_slice());
        word
    }

    fn word_bytes4(selector: [u8; 4]) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[0..4].copy_from_slice(&selector);
        word
    }

    fn dynamic_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&word_u256(bytes.len()));
        out.extend_from_slice(bytes);
        let padding = (32 - (bytes.len() % 32)) % 32;
        out.extend(std::iter::repeat(0).take(padding));
        out
    }

    fn error_string(message: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&SELECTOR_ERROR_STRING);
        out.extend_from_slice(&word_u256(32));
        out.extend_from_slice(&dynamic_bytes(message.as_bytes()));
        out
    }

    fn wrapped_error(target: Address, selector: [u8; 4], reason: &[u8], details: &[u8]) -> Vec<u8> {
        let reason_tail = dynamic_bytes(reason);
        let details_tail = dynamic_bytes(details);
        let details_offset = 4 * 32 + reason_tail.len();

        let mut out = Vec::new();
        out.extend_from_slice(&SELECTOR_WRAPPED_ERROR);
        out.extend_from_slice(&word_address(target));
        out.extend_from_slice(&word_bytes4(selector));
        out.extend_from_slice(&word_u256(4 * 32));
        out.extend_from_slice(&word_u256(details_offset));
        out.extend_from_slice(&reason_tail);
        out.extend_from_slice(&details_tail);
        out
    }

    #[test]
    fn decodes_standard_error_string() {
        let decoded = decode_revert_output(&error_string("hook blocked")).unwrap();
        assert_eq!(decoded.signature, Some("Error(string)"));
        assert_eq!(decoded.summary, "hook blocked");
    }

    #[test]
    fn decodes_wrapped_error_with_nested_reason() {
        let target = address!("298a86cc43af878cb78ca20e80ab0de0a59a0444");
        let reason = error_string("afterSwap rejected");
        let decoded = decode_revert_output(&wrapped_error(
            target,
            [0x12, 0x34, 0x56, 0x78],
            &reason,
            &[],
        ))
        .unwrap();

        assert_eq!(
            decoded.signature,
            Some("WrappedError(address,bytes4,bytes,bytes)")
        );
        assert!(decoded.summary.contains("WrappedError"));
        assert!(decoded.summary.contains("0x12345678"));
        assert!(decoded.summary.contains("afterSwap rejected"));
        assert!(decoded.summary.contains("details=empty"));
    }

    #[test]
    fn labels_unknown_custom_error_selector() {
        let decoded = decode_revert_output(&[0xde, 0xad, 0xbe, 0xef]).unwrap();
        assert_eq!(decoded.summary, "unknown custom error selector 0xdeadbeef");
    }
}
