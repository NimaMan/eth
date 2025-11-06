"""Canonical DEX pool type identifiers used across the codebase."""

# Canonical pool type identifiers. These align with Rust constants in
# `reth_chain_query::dex::pool_types`.
DEX_POOL_TYPES = (
    "UNISWAP-V2",
    "UNISWAP-V3",
    "UNISWAP-V4",
    "SUSHI-SWAP",
    "CURVE",
    "BALANCER",
)


DEX_POOL_TYPE_SET = set(DEX_POOL_TYPES)
