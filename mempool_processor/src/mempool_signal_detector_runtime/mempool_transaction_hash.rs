use std::str::FromStr;

use alloy_primitives::B256;

pub(crate) fn parse_mempool_transaction_hash_or_zero(hash: &str) -> B256 {
    let trimmed = hash.trim();
    let normalized = if trimmed.starts_with("0x") {
        trimmed.to_string()
    } else {
        format!("0x{trimmed}")
    };
    B256::from_str(&normalized).unwrap_or(B256::ZERO)
}
