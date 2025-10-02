//! Canonical DEX pool type identifiers shared with Python utilities.

/// Canonical DEX pool type identifiers.
///
/// These must stay in sync with `eth_data.chain_utils.common_addresses.dex_pool_types`.
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
