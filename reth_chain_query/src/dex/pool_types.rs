//! Canonical DEX pool type identifiers shared across Rust/Python components.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Canonical DEX pool type identifiers.
pub const DEX_POOL_TYPES: &[&str] = &[
    "UNISWAP-V2",
    "UNISWAP-V3",
    "UNISWAP-V4",
    "SUSHISWAP-V2",
    "PANCAKESWAP-V2",
    "SHIBASWAP-V2",
    "FRAXSWAP-V2",
    "CURVE",
    "BALANCER",
];

/// Convenience alias for the default V2-style pool type.
pub const DEFAULT_POOL_TYPE: &str = "UNISWAP-V2";

const DEX_POOL_TYPE_ALIASES: &[(&str, &str)] = &[
    ("UNISWAPV2", "UNISWAP-V2"),
    ("UNIV2", "UNISWAP-V2"),
    ("V2", "UNISWAP-V2"),
    ("UNISWAPV3", "UNISWAP-V3"),
    ("UNIV3", "UNISWAP-V3"),
    ("V3", "UNISWAP-V3"),
    ("UNISWAPV4", "UNISWAP-V4"),
    ("UNIV4", "UNISWAP-V4"),
    ("V4", "UNISWAP-V4"),
    ("SUSHISWAP", "SUSHISWAP-V2"),
    ("SUSHISWAPV2", "SUSHISWAP-V2"),
    ("SUSHISWAP2", "SUSHISWAP-V2"),
    ("PANCAKESWAP", "PANCAKESWAP-V2"),
    ("PANCAKESWAPV2", "PANCAKESWAP-V2"),
    ("PANCAKEV2", "PANCAKESWAP-V2"),
    ("SHIBASWAP", "SHIBASWAP-V2"),
    ("SHIBASWAPV2", "SHIBASWAP-V2"),
    ("FRAXSWAP", "FRAXSWAP-V2"),
    ("FRAXSWAPV2", "FRAXSWAP-V2"),
    ("CURVE", "CURVE"),
    ("BALANCER", "BALANCER"),
];

static DEX_POOL_TYPE_KEY_MAP: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| DEX_POOL_TYPE_ALIASES.iter().copied().collect());

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn alias_keys_are_unique() {
        let mut seen = HashSet::new();

        for (alias, _) in DEX_POOL_TYPE_ALIASES {
            assert!(seen.insert(*alias), "duplicate pool type alias: {alias}");
        }
    }

    #[test]
    fn canonicalizes_known_v2_pool_types() {
        assert_eq!(canonicalize_pool_type("sushi-swap"), Some("SUSHISWAP-V2"));
        assert_eq!(
            canonicalize_pool_type("pancakeswap_v2"),
            Some("PANCAKESWAP-V2")
        );
        assert_eq!(canonicalize_pool_type("shibaswap"), Some("SHIBASWAP-V2"));
        assert_eq!(canonicalize_pool_type("fraxswap"), Some("FRAXSWAP-V2"));
    }
}
