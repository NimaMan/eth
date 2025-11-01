use crate::{ExpectedBehavior, TokenConfig};
use alloy_primitives::Address;
use reth_chain_query::common_addresses::denom_tokens::token_address;

fn must_address(symbol: &str) -> Address {
    token_address(symbol).unwrap_or_else(|| {
        panic!(
            "Token address for {} not configured in common_addresses",
            symbol
        )
    })
}

pub fn token_configs() -> Vec<TokenConfig> {
    let weth = must_address("WETH");

    vec![
        // ===================== 0.05% FEE TIER (500) =====================
        // Stablecoins and highly liquid pairs use 0.05%
        TokenConfig {
            symbol: "USDC",
            token_address: must_address("USDC"),
            denom_address: weth,
            fee_tier: 500,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 6,
        },
        TokenConfig {
            symbol: "WBTC",
            token_address: must_address("WBTC"),
            denom_address: weth,
            fee_tier: 500,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 8,
        },
        TokenConfig {
            symbol: "DAI",
            token_address: must_address("DAI"),
            denom_address: weth,
            fee_tier: 500,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "USDT",
            token_address: must_address("USDT"),
            denom_address: weth,
            fee_tier: 500,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 6,
        },
        // ===================== 0.30% FEE TIER (3000) =====================
        // Most common fee tier for standard volatility tokens
        TokenConfig {
            symbol: "UNI",
            token_address: must_address("UNI"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LINK",
            token_address: must_address("LINK"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "AAVE",
            token_address: must_address("AAVE"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MKR",
            token_address: must_address("MKR"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MATIC",
            token_address: must_address("MATIC"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LDO",
            token_address: must_address("LDO"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "CRV",
            token_address: must_address("CRV"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "SNX",
            token_address: must_address("SNX"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "ENS",
            token_address: must_address("ENS"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "FXS",
            token_address: must_address("FXS"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== 1.00% FEE TIER (10000) =====================
        // High volatility tokens, memecoins
        TokenConfig {
            symbol: "PEPE",
            token_address: must_address("PEPE"),
            denom_address: weth,
            fee_tier: 10000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "SHIB",
            token_address: must_address("SHIB"),
            denom_address: weth,
            fee_tier: 10000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "FLOKI",
            token_address: must_address("FLOKI"),
            denom_address: weth,
            fee_tier: 10000,
            expected_behavior: ExpectedBehavior::MightFail("May have taxes"),
            decimals: 9,
        },
        TokenConfig {
            symbol: "APE",
            token_address: must_address("APE"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "DOGE",
            token_address: must_address("DOGE"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 8,
        },
        // ===================== ADDITIONAL TOKENS =====================
        TokenConfig {
            symbol: "RPL",
            token_address: must_address("RPL"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "ARB",
            token_address: must_address("ARB"),
            denom_address: weth,
            fee_tier: 500,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "OP",
            token_address: must_address("OP"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::MightFail("Bridge token"),
            decimals: 18,
        },
        TokenConfig {
            symbol: "BLUR",
            token_address: must_address("BLUR"),
            denom_address: weth,
            fee_tier: 3000,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
    ]
}
