//! Canonical token sets used across DEX simulations.
//!
//! This module centralises token metadata (address, decimals, default
//! denomination and, where relevant, additional pool configuration) so that
//! higher level crates can build scenario-specific expectations without
//! duplicating addresses or fee tier metadata.

use alloy_primitives::{address, Address, B256, U256};
use once_cell::sync::Lazy;

use crate::common_addresses::denom_tokens::{get_token_decimals, token_address};
use crate::dex::BALANCER_VAULT;

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

/// Minimal metadata required to describe a SushiSwap token/denom pair.
#[derive(Debug, Clone, Copy)]
pub struct SushiSwapTokenInfo {
    pub symbol: &'static str,
    pub token_address: Address,
    pub denom_address: Address,
    pub decimals: u8,
}

/// Minimal metadata required to describe a Uniswap V4 pool.
#[derive(Debug, Clone, Copy)]
pub struct UniswapV4PoolInfo {
    pub symbol: &'static str,
    pub token_address: Address,
    pub denom_address: Address,
    pub denom_symbol: &'static str,
    pub pool_manager: Address,
    pub pool_id: B256,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
    pub token_decimals: u8,
    pub denom_decimals: u8,
    pub block_hint: Option<u64>,
}

/// Token metadata for a Curve pool constituent.
#[derive(Debug, Clone)]
pub struct CurvePoolTokenInfo {
    pub symbol: &'static str,
    pub token_address: Address,
    pub decimals: u8,
    pub index: usize,
}

/// Minimal metadata required to describe a Curve pool that we commonly use.
#[derive(Debug, Clone)]
pub struct CurvePoolInfo {
    pub name: &'static str,
    pub pool_address: Address,
    pub lp_token_address: Option<Address>,
    pub base_token_index: usize,
    pub quote_token_index: usize,
    pub tokens: Vec<CurvePoolTokenInfo>,
}

/// Token metadata for a Balancer pool constituent.
#[derive(Debug, Clone)]
pub struct BalancerTokenInfo {
    pub symbol: &'static str,
    pub token_address: Address,
    pub decimals: u8,
    pub index: usize,
    pub weight: Option<U256>,
}

/// Minimal metadata required to describe a Balancer pool.
#[derive(Debug, Clone)]
pub struct BalancerPoolInfo {
    pub name: &'static str,
    pub pool_id: B256,
    pub pool_address: Address,
    pub vault_address: Address,
    pub swap_fee_bps: u32,
    pub tokens: Vec<BalancerTokenInfo>,
}

/// Stablecoin pair specification mapping to protocol-specific pair labels
#[derive(Debug, Clone, Copy)]
pub struct StablecoinPairSpec {
    pub key: &'static str,
    pub protocol: &'static str,
    pub pair_label: &'static str,
}

const ETH_ADDRESS: Address = address!("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE");

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
        // Stablecoins
        ("USDC", "WETH"),
        ("USDT", "WETH"),
        ("DAI", "WETH"),
        ("LUSD", "WETH"),
        ("FRAX", "WETH"),
        ("FEI", "WETH"),
        ("MIM", "WETH"),
        // Wrapped assets / LSDs
        ("WBTC", "WETH"),
        ("stETH", "WETH"),
        ("cbETH", "WETH"),
        ("rETH", "WETH"),
        // DeFi blue chips
        ("UNI", "WETH"),
        ("LINK", "WETH"),
        ("AAVE", "WETH"),
        ("MKR", "WETH"),
        ("CRV", "WETH"),
        ("SNX", "WETH"),
        ("COMP", "WETH"),
        ("YFI", "WETH"),
        ("SUSHI", "WETH"),
        ("BAL", "WETH"),
        ("1INCH", "WETH"),
        ("ENS", "WETH"),
        ("FXS", "WETH"),
        ("LDO", "WETH"),
        // Layer 2 / staking tokens
        ("MATIC", "WETH"),
        // Growth / infra tokens
        ("GRT", "WETH"),
        ("GNO", "WETH"),
        ("FET", "WETH"),
        ("RNDR", "WETH"),
        ("LRC", "WETH"),
        // Gaming / metaverse
        ("MANA", "WETH"),
        ("SAND", "WETH"),
        ("ENJ", "WETH"),
        ("AXS", "WETH"),
        ("GALA", "WETH"),
        ("APE", "WETH"),
        // Meme / higher risk tokens
        ("PEPE", "WETH"),
        ("SHIB", "WETH"),
        ("DOGE", "WETH"),
        ("FLOKI", "WETH"),
        ("BONE", "WETH"),
        ("ELON", "WETH"),
        ("AKITA", "WETH"),
        ("BABYDOGE", "WETH"),
        ("KISHU", "WETH"),
        ("TORN", "WETH"),
        ("FTT", "WETH"),
        ("BLUR", "WETH"),
        ("BAT", "WETH"),
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
        ("LUSD", "WETH", 500),
        ("stETH", "WETH", 500),
        ("cbETH", "WETH", 500),
        ("rETH", "WETH", 500),
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
        ("BAL", "WETH", 3000),
        ("SUSHI", "WETH", 3000),
        ("COMP", "WETH", 3000),
        ("YFI", "WETH", 3000),
        ("1INCH", "WETH", 3000),
        ("APE", "WETH", 3000),
        ("RPL", "WETH", 3000),
        ("ARB", "WETH", 3000),
        ("BLUR", "WETH", 3000),
        ("BAT", "WETH", 3000),
        ("GRT", "WETH", 3000),
        ("GNO", "WETH", 3000),
        ("RNDR", "WETH", 3000),
        ("FET", "WETH", 3000),
        ("MANA", "WETH", 3000),
        ("SAND", "WETH", 3000),
        ("ENJ", "WETH", 3000),
        ("AXS", "WETH", 3000),
        ("GALA", "WETH", 3000),
        // 1.00% fee tier
        ("PEPE", "WETH", 10_000),
        ("SHIB", "WETH", 10_000),
        ("DOGE", "WETH", 10_000),
        ("FLOKI", "WETH", 10_000),
        ("BONE", "WETH", 10_000),
        ("ELON", "WETH", 10_000),
        ("AKITA", "WETH", 10_000),
        ("BABYDOGE", "WETH", 10_000),
        ("KISHU", "WETH", 10_000),
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

static SUSHISWAP_TOKEN_SET: Lazy<Vec<SushiSwapTokenInfo>> = Lazy::new(|| {
    let weth = resolve_token("WETH");
    let entries: &[(&str, &str)] = &[
        ("USDC", "WETH"),
        ("USDT", "WETH"),
        ("DAI", "WETH"),
        ("WBTC", "WETH"),
        ("MATIC", "WETH"),
        ("SUSHI", "WETH"),
        ("BAL", "WETH"),
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
            SushiSwapTokenInfo {
                symbol,
                token_address: token,
                denom_address: denom_addr,
                decimals: resolve_decimals(symbol),
            }
        })
        .collect()
});

static UNISWAP_V4_POOL_SET: Lazy<Vec<UniswapV4PoolInfo>> = Lazy::new(|| {
    let weth = resolve_token("WETH");
    let usdc = resolve_token("USDC");
    let token_decimals = resolve_decimals("USDC");
    let denom_decimals = resolve_decimals("WETH");
    vec![
        UniswapV4PoolInfo {
            symbol: "USDC",
            token_address: usdc,
            denom_address: weth,
            denom_symbol: "WETH",
            pool_manager: address!("000000000004444C5DC75cB358380d2E3de08a90"),
            pool_id: B256::from_slice(&hex_literal::hex!(
                "11142dd4ac627021305b9349c2167d89744c4e45c92ce383c04120337f86495c"
            )),
            fee: 490,
            tick_spacing: 10,
            hooks: Address::ZERO,
            token_decimals,
            denom_decimals,
            block_hint: Some(23560197),
        },
        UniswapV4PoolInfo {
            symbol: "MOONSTR",
            token_address: address!("7bf4C3Ea48522217446416D39dB92EBeF7848778"),
            denom_address: address!("0000000000000000000000000000000000000000"),
            denom_symbol: "ETH",
            pool_manager: address!("000000000004444C5DC75cB358380d2E3de08a90"),
            pool_id: B256::from_slice(&hex_literal::hex!(
                "5f8705d214f90a577483f45910165dce245b435a26e661cf757a70fd665249ce"
            )),
            fee: 0,
            tick_spacing: 60,
            hooks: address!("213F0db3d48580954471B3E69E5F486292a36844"),
            token_decimals: 18,
            denom_decimals: 18,
            block_hint: Some(23583177),
        },
    ]
});

static CURVE_POOL_SET: Lazy<Vec<CurvePoolInfo>> = Lazy::new(|| {
    vec![
        CurvePoolInfo {
            name: "3pool",
            pool_address: address!("DC24316b9AE028F1497c275EB9192a3Ea0f67022"),
            lp_token_address: Some(address!("6c3f90f043a72fa612cbac8115ee7e52bde6e490")),
            base_token_index: 0,
            quote_token_index: 1,
            tokens: vec![
                CurvePoolTokenInfo {
                    symbol: "DAI",
                    token_address: resolve_token("DAI"),
                    decimals: resolve_decimals("DAI"),
                    index: 0,
                },
                CurvePoolTokenInfo {
                    symbol: "USDC",
                    token_address: resolve_token("USDC"),
                    decimals: resolve_decimals("USDC"),
                    index: 1,
                },
                CurvePoolTokenInfo {
                    symbol: "USDT",
                    token_address: resolve_token("USDT"),
                    decimals: resolve_decimals("USDT"),
                    index: 2,
                },
            ],
        },
        CurvePoolInfo {
            name: "stETH-ETH",
            pool_address: address!("DC24316b9AE028F1497c275EB9192a3Ea0f67022"),
            lp_token_address: Some(address!("06325440D014e39736583c165C2963BA99fAf14E")),
            base_token_index: 0,
            quote_token_index: 1,
            tokens: vec![
                CurvePoolTokenInfo {
                    symbol: "stETH",
                    token_address: resolve_token("stETH"),
                    decimals: resolve_decimals("stETH"),
                    index: 0,
                },
                CurvePoolTokenInfo {
                    symbol: "ETH",
                    token_address: ETH_ADDRESS,
                    decimals: 18,
                    index: 1,
                },
            ],
        },
    ]
});

static BALANCER_POOL_SET: Lazy<Vec<BalancerPoolInfo>> = Lazy::new(|| {
    vec![BalancerPoolInfo {
        name: "BAL-WETH 80/20",
        pool_id: B256::from_slice(&hex_literal::hex!(
            "c7c7d2a7711576a8a93a3521d02e249e48f5fdfcbe6aa1a59e41d2d4c4b1f6a1"
        )),
        pool_address: address!("ba100000625a3754423978a60c9317c58a424e3d"),
        vault_address: BALANCER_VAULT,
        swap_fee_bps: 30,
        tokens: vec![
            BalancerTokenInfo {
                symbol: "BAL",
                token_address: resolve_token("BAL"),
                decimals: resolve_decimals("BAL"),
                index: 0,
                weight: Some(U256::from(80u8)),
            },
            BalancerTokenInfo {
                symbol: "WETH",
                token_address: resolve_token("WETH"),
                decimals: resolve_decimals("WETH"),
                index: 1,
                weight: Some(U256::from(20u8)),
            },
        ],
    }]
});

/// Return the canonical Uniswap V2 token set metadata.
pub fn uniswap_v2_tokens() -> &'static [UniswapV2TokenInfo] {
    UNISWAP_V2_TOKEN_SET.as_slice()
}

/// Return the canonical Uniswap V3 token set metadata.
pub fn uniswap_v3_tokens() -> &'static [UniswapV3TokenInfo] {
    UNISWAP_V3_TOKEN_SET.as_slice()
}

/// Return the canonical SushiSwap token set metadata.
pub fn sushiswap_tokens() -> &'static [SushiSwapTokenInfo] {
    SUSHISWAP_TOKEN_SET.as_slice()
}

/// Return the canonical Uniswap V4 pool metadata.
pub fn uniswap_v4_pools() -> &'static [UniswapV4PoolInfo] {
    UNISWAP_V4_POOL_SET.as_slice()
}

/// Return the canonical Curve pool metadata.
pub fn curve_pools() -> &'static [CurvePoolInfo] {
    CURVE_POOL_SET.as_slice()
}

/// Return the canonical Balancer pool metadata.
pub fn balancer_pools() -> &'static [BalancerPoolInfo] {
    BALANCER_POOL_SET.as_slice()
}

static ETH_USDC_PAIR_SPECS: &[StablecoinPairSpec] = &[
    StablecoinPairSpec {
        key: "UniswapV2",
        protocol: "UniswapV2",
        pair_label: "ETH/USD",
    },
    StablecoinPairSpec {
        key: "SushiSwap",
        protocol: "SushiSwap",
        pair_label: "ETH/USD",
    },
    StablecoinPairSpec {
        key: "UniswapV3_500",
        protocol: "UniswapV3",
        pair_label: "ETH/USD_V3_500",
    },
    StablecoinPairSpec {
        key: "UniswapV3_3000",
        protocol: "UniswapV3",
        pair_label: "ETH/USD_V3_3000",
    },
];

static ETH_USDT_PAIR_SPECS: &[StablecoinPairSpec] = &[
    StablecoinPairSpec {
        key: "UniswapV2",
        protocol: "UniswapV2",
        pair_label: "ETH/USDT",
    },
    StablecoinPairSpec {
        key: "SushiSwap",
        protocol: "SushiSwap",
        pair_label: "ETH/USDT",
    },
    StablecoinPairSpec {
        key: "UniswapV3_3000",
        protocol: "UniswapV3",
        pair_label: "ETH/USDT_V3",
    },
    StablecoinPairSpec {
        key: "Curve_TriCrypto2",
        protocol: "Curve",
        pair_label: "ETH/USDT_CURVE",
    },
];

static ETH_DAI_PAIR_SPECS: &[StablecoinPairSpec] = &[
    StablecoinPairSpec {
        key: "UniswapV2",
        protocol: "UniswapV2",
        pair_label: "ETH/DAI",
    },
    StablecoinPairSpec {
        key: "SushiSwap",
        protocol: "SushiSwap",
        pair_label: "ETH/DAI",
    },
];

/// Stablecoin pair specs for ETH/USDC across supported venues.
pub fn eth_usdc_pairs() -> &'static [StablecoinPairSpec] {
    ETH_USDC_PAIR_SPECS
}

/// Stablecoin pair specs for ETH/USDT across supported venues.
pub fn eth_usdt_pairs() -> &'static [StablecoinPairSpec] {
    ETH_USDT_PAIR_SPECS
}

/// Stablecoin pair specs for ETH/DAI across supported venues.
pub fn eth_dai_pairs() -> &'static [StablecoinPairSpec] {
    ETH_DAI_PAIR_SPECS
}

static USDC_USDT_PAIR_SPECS: &[StablecoinPairSpec] = &[
    StablecoinPairSpec {
        key: "UniswapV3_100",
        protocol: "UniswapV3",
        pair_label: "USDC/USDT_V3_100",
    },
    StablecoinPairSpec {
        key: "UniswapV3_500",
        protocol: "UniswapV3",
        pair_label: "USDC/USDT_V3_500",
    },
];

/// Stablecoin pair specs for USDC/USDT across supported venues.
pub fn usdc_usdt_pairs() -> &'static [StablecoinPairSpec] {
    USDC_USDT_PAIR_SPECS
}

static DAI_USDC_PAIR_SPECS: &[StablecoinPairSpec] = &[StablecoinPairSpec {
    key: "UniswapV3_100",
    protocol: "UniswapV3",
    pair_label: "DAI/USDC_V3",
}];

/// Stablecoin pair specs for DAI/USDC across supported venues.
pub fn dai_usdc_pairs() -> &'static [StablecoinPairSpec] {
    DAI_USDC_PAIR_SPECS
}
