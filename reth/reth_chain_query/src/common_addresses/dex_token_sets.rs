//! Canonical token sets used across Uniswap pool simulations.
//!
//! This module centralises token metadata (address, decimals, default
//! denomination and, for V3, fee tier) so that higher level crates can
//! build scenario-specific expectations without duplicating addresses.

use alloy_primitives::Address;
use once_cell::sync::Lazy;

use crate::common_addresses::denom_tokens::{get_token_decimals, token_address};

/// Minimal metadata required to describe a Uniswap V2 token/denom pair.
#[derive(Debug, Clone, Copy)]
pub struct UniswapV2TokenInfo {
    pub symbol: &'static str,
    pub token_address: Address,
    pub denom_address: Address,
    pub decimals: u8,
}

/// Minimal metadata required to describe a Uniswap V3 token/denom pair.
#[derive(Debug, Clone, Copy)]
pub struct UniswapV3TokenInfo {
    pub symbol: &'static str,
    pub token_address: Address,
    pub denom_address: Address,
    pub fee_tier: u32,
    pub decimals: u8,
}

fn resolve_token(symbol: &str) -> Address {
    token_address(symbol).unwrap_or_else(|| panic!("Token address for {} not configured", symbol))
}

fn resolve_decimals(symbol: &str) -> u8 {
    get_token_decimals(symbol)
        .unwrap_or_else(|| panic!("Token decimals for {} not configured", symbol))
}

static UNISWAP_V2_TOKEN_SET: Lazy<Vec<UniswapV2TokenInfo>> = Lazy::new(|| {
    let weth = resolve_token("WETH");
    let entries: &[(&str, &str)] = &[
        // Stablecoins / blue chips
        ("USDC", "WETH"),
        ("USDT", "WETH"),
        ("DAI", "WETH"),
        // Wrapped assets
        ("WBTC", "WETH"),
        ("stETH", "WETH"),
        // DeFi blue chips
        ("UNI", "WETH"),
        ("LINK", "WETH"),
        ("AAVE", "WETH"),
        ("MKR", "WETH"),
        ("CRV", "WETH"),
        // Layer 2 / staking tokens
        ("MATIC", "WETH"),
        ("LDO", "WETH"),
        // Meme / higher risk tokens
        ("PEPE", "WETH"),
        ("SHIB", "WETH"),
        ("DOGE", "WETH"),
        ("FLOKI", "WETH"),
        // Exchange / utility
        ("FTT", "WETH"),
        ("GRT", "WETH"),
        ("BAT", "WETH"),
        // Algo stables
        ("FRAX", "WETH"),
        ("MIM", "WETH"),
        // Gaming / metaverse
        ("AXS", "WETH"),
        ("SAND", "WETH"),
        ("MANA", "WETH"),
        ("ENJ", "WETH"),
        ("GALA", "WETH"),
        // AI / infra
        ("FET", "WETH"),
        ("RNDR", "WETH"),
        ("GNO", "WETH"),
        ("LRC", "WETH"),
        // Newer DeFi
        ("ENS", "WETH"),
        // Additional meme/tax tokens
        ("BONE", "WETH"),
        ("ELON", "WETH"),
        ("AKITA", "WETH"),
        // Privacy / tax tokens
        ("TORN", "WETH"),
        ("BABYDOGE", "WETH"),
        ("KISHU", "WETH"),
    ];

    entries
        .iter()
        .map(|(symbol, denom)| {
            let token = resolve_token(symbol);
            let denom_addr = if *denom == "WETH" {
                weth
            } else {
                resolve_token(denom)
            };
            UniswapV2TokenInfo {
                symbol,
                token_address: token,
                denom_address: denom_addr,
                decimals: resolve_decimals(symbol),
            }
        })
        .collect()
});

static UNISWAP_V3_TOKEN_SET: Lazy<Vec<UniswapV3TokenInfo>> = Lazy::new(|| {
    let weth = resolve_token("WETH");
    let entries: &[(&str, &str, u32)] = &[
        // 0.05% fee tier
        ("USDC", "WETH", 500),
        ("WBTC", "WETH", 500),
        ("DAI", "WETH", 500),
        ("USDT", "WETH", 500),
        // 0.30% fee tier
        ("UNI", "WETH", 3000),
        ("LINK", "WETH", 3000),
        ("AAVE", "WETH", 3000),
        ("MKR", "WETH", 3000),
        ("MATIC", "WETH", 3000),
        ("LDO", "WETH", 3000),
        ("CRV", "WETH", 3000),
        ("SNX", "WETH", 3000),
        ("ENS", "WETH", 3000),
        ("FXS", "WETH", 3000),
        ("APE", "WETH", 3000),
        ("RPL", "WETH", 3000),
        ("ARB", "WETH", 3000),
        ("BLUR", "WETH", 3000),
        // 1.00% fee tier
        ("PEPE", "WETH", 10_000),
        ("SHIB", "WETH", 10_000),
        ("FLOKI", "WETH", 10_000),
    ];

    entries
        .iter()
        .map(|(symbol, denom, fee_tier)| {
            let token = resolve_token(symbol);
            let denom_addr = if *denom == "WETH" {
                weth
            } else {
                resolve_token(denom)
            };
            UniswapV3TokenInfo {
                symbol,
                token_address: token,
                denom_address: denom_addr,
                fee_tier: *fee_tier,
                decimals: resolve_decimals(symbol),
            }
        })
        .collect()
});

/// Returns the canonical Uniswap V2 token set metadata.
pub fn uniswap_v2_tokens() -> &'static [UniswapV2TokenInfo] {
    UNISWAP_V2_TOKEN_SET.as_slice()
}

/// Returns the canonical Uniswap V3 token set metadata.
pub fn uniswap_v3_tokens() -> &'static [UniswapV3TokenInfo] {
    UNISWAP_V3_TOKEN_SET.as_slice()
}
