use crate::{ExpectedBehavior, TokenConfig};
use alloy_primitives::Address;
use reth_chain_query::common_addresses::denom_tokens::token_address;
use tx_processor::simulator::PoolType;

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
        // ===================== STABLECOINS =====================
        // Tier 1: Blue Chip Stablecoins (Should work perfectly)
        TokenConfig {
            symbol: "USDC",
            token_address: must_address("USDC"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 6,
        },
        TokenConfig {
            symbol: "USDT",
            token_address: must_address("USDT"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 6,
        },
        TokenConfig {
            symbol: "DAI",
            token_address: must_address("DAI"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== WRAPPED ASSETS =====================
        TokenConfig {
            symbol: "WBTC",
            token_address: must_address("WBTC"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 8,
        },
        TokenConfig {
            symbol: "stETH",
            token_address: must_address("stETH"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== DEFI BLUE CHIPS =====================
        TokenConfig {
            symbol: "UNI",
            token_address: must_address("UNI"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LINK",
            token_address: must_address("LINK"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "AAVE",
            token_address: must_address("AAVE"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MKR",
            token_address: must_address("MKR"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "CRV",
            token_address: must_address("CRV"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== LAYER 2 TOKENS =====================
        TokenConfig {
            symbol: "MATIC",
            token_address: must_address("MATIC"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LDO",
            token_address: must_address("LDO"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== MEME TOKENS =====================
        TokenConfig {
            symbol: "PEPE",
            token_address: must_address("PEPE"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "SHIB",
            token_address: must_address("SHIB"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "DOGE",
            token_address: must_address("DOGE"), // Wrapped DOGE
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 8,
        },
        TokenConfig {
            symbol: "FLOKI",
            token_address: must_address("FLOKI"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 9,
        },
        // ===================== EXCHANGE TOKENS =====================
        TokenConfig {
            symbol: "FTT",
            token_address: must_address("FTT"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // FTT might have issues after FTX collapse
            decimals: 18,
        },
        // ===================== UTILITY TOKENS =====================
        TokenConfig {
            symbol: "GRT",
            token_address: must_address("GRT"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "BAT",
            token_address: must_address("BAT"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== ALGO STABLES =====================
        TokenConfig {
            symbol: "FRAX",
            token_address: must_address("FRAX"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MIM",
            token_address: must_address("MIM"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== GAMING/METAVERSE =====================
        TokenConfig {
            symbol: "AXS",
            token_address: must_address("AXS"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "SAND",
            token_address: must_address("SAND"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MANA",
            token_address: must_address("MANA"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "ENJ",
            token_address: must_address("ENJ"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "GALA",
            token_address: must_address("GALA"), // GALA/WETH V2
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 8,
        },
        // ===================== AI/TECH TOKENS =====================
        TokenConfig {
            symbol: "FET",
            token_address: must_address("FET"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "RNDR",
            token_address: must_address("RNDR"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== INFRASTRUCTURE =====================
        TokenConfig {
            symbol: "GNO",
            token_address: must_address("GNO"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LRC",
            token_address: must_address("LRC"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== DEFI 2.0/NEWER =====================
        TokenConfig {
            symbol: "ENS",
            token_address: must_address("ENS"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        // ===================== MORE MEME TOKENS =====================
        TokenConfig {
            symbol: "BONE",
            token_address: must_address("BONE"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "ELON",
            token_address: must_address("ELON"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "AKITA",
            token_address: must_address("AKITA"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        // ===================== PRIVACY/CONTROVERSIAL =====================
        TokenConfig {
            symbol: "TORN",
            token_address: must_address("TORN"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // Might fail due to sanctions
            decimals: 18,
        },
        // ===================== TAX/REFLECTION TOKENS =====================
        TokenConfig {
            symbol: "BABYDOGE",
            token_address: must_address("BABYDOGE"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // Known reflection token
            decimals: 9,
        },
        TokenConfig {
            symbol: "KISHU",
            token_address: must_address("KISHU"),
            denom_address: weth,
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // 2% redistribution fee
            decimals: 9,
        },
    ]
}
