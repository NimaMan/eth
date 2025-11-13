//! Canonical DEX pool type identifiers shared across Rust/Python components.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Canonical DEX pool type identifiers.
pub const DEX_POOL_TYPES: &[&str] = &[
    "UNISWAP-V2",
    "UNISWAP-V3",
    "UNISWAP-V4",
    "SUSHI-SWAP",
    "CURVE",
    "BALANCER",
];

/// Convenience alias for the default V2-style pool type.
pub const DEFAULT_POOL_TYPE: &str = "UNISWAP-V2";

static DEX_POOL_TYPE_KEY_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("UNISWAPV2", "UNISWAP-V2");
    map.insert("UNIV2", "UNISWAP-V2");
    map.insert("V2", "UNISWAP-V2");
    map.insert("UNISWAPV3", "UNISWAP-V3");
    map.insert("UNIV3", "UNISWAP-V3");
    map.insert("V3", "UNISWAP-V3");
    map.insert("UNISWAPV4", "UNISWAP-V4");
    map.insert("UNIV4", "UNISWAP-V4");
    map.insert("V4", "UNISWAP-V4");
    map.insert("SUSHISWAP", "SUSHI-SWAP");
    map.insert("CURVE", "CURVE");
    map.insert("BALANCER", "BALANCER");
    map
});

fn normalize_key(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Canonicalize a DEX pool type identifier.
pub fn canonicalize_pool_type(name: &str) -> Option<&'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let key = normalize_key(trimmed);
    if let Some(value) = DEX_POOL_TYPE_KEY_MAP.get(key.as_str()) {
        return Some(*value);
    }

    let normalized = trimmed
        .to_uppercase()
        .replace('_', "-")
        .replace(' ', "-")
        .replace('/', "-");

    if let Some(value) = DEX_POOL_TYPES
        .iter()
        .copied()
        .find(|candidate| *candidate == normalized)
    {
        return Some(value);
    }

    let normalized_key = normalize_key(&normalized);
    DEX_POOL_TYPE_KEY_MAP.get(normalized_key.as_str()).copied()
}
