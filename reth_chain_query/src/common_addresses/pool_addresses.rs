//! Known DEX factory/router addresses and helpers for protocol classification.

use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Named map of pool factory / registry contracts we track.
pub static POOL_FACTORIES: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let entries: &[(&str, Address)] = &[
        (
            "univ2_factory",
            address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
        ),
        (
            "univ3_factory",
            address!("1F98431c8aD98523631AE4a59f267346ea31F984"),
        ),
        (
            "univ4_pool_manager",
            address!("000000000004444C5DC75cB358380d2E3de08a90"),
        ),
        (
            "sushi_factory",
            address!("C0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"),
        ),
        (
            "pancake_factory",
            address!("cA143Ce32Fe78f1f7019d7d551a6402fC5350C73"),
        ),
        (
            "pancake_v3_factory",
            address!("0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
        ),
        (
            "curve_registry",
            address!("90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5"),
        ),
        (
            "curve_factory",
            address!("B9fC157394Af804a3578134A6585C0dc9cc990d4"),
        ),
        (
            "balancer_vault",
            address!("BA12222222228d8Ba445958a75a0704d566BF2C8"),
        ),
        (
            "kyber_dmm_factory",
            address!("833e4083B7ae46CeA85695c4f7ed25CDAd8886dE"),
        ),
        (
            "dodo_v2_factory",
            address!("3A97247DF274a17C59A3bd12735ea3FcDFb49950"),
        ),
        (
            "dodo_dpp_factory",
            address!("6B4Fa0bc61Eddc928e0Df9c7f01e407BfcD3e5EF"),
        ),
        (
            "oneinch_v2_factory",
            address!("bAF9A5d4b0052359326A6CDAb54BABAa3a3A9643"),
        ),
        (
            "shibaswap_factory",
            address!("115934131916C8b277DD010Ee02de363c09d037c"),
        ),
        (
            "fraxswap_factory",
            address!("43eC799eAdd63848443E2347C49f5f52e8Fe0F6f"),
        ),
    ];

    entries.iter().copied().collect()
});

/// Named map of router contracts.
pub static ROUTERS: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let entries: &[(&str, Address)] = &[
        (
            "univ2_router",
            address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
        ),
        (
            "univ3_router",
            address!("E592427A0AECe92De3Edee1F18E0157C05861564"),
        ),
        (
            "univ3_router2",
            address!("68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"),
        ),
        (
            "sushi_router",
            address!("d9e1cE17f2641f24aE83637ab66a2cca9C378B9F"),
        ),
        (
            "pancake_router",
            address!("EfF92A263d31888d860bD50809A8D171709b7b1c"),
        ),
        (
            "pancake_v3_router",
            address!("13f4EA83D0bd40E75C8222255bc855a974568Dd4"),
        ),
        (
            "kyber_router",
            address!("1c87257F5e8609940Bc751a07BB085Bb7f8cDBE6"),
        ),
        (
            "dodo_v2_router",
            address!("a2398842F37465f89540430bDC00219fA9E4D28a"),
        ),
        (
            "oneinch_router",
            address!("1111111254EEB25477B68fb85Ed929f73A960582"),
        ),
        (
            "shibaswap_router",
            address!("03f7724180AA6b939894B5Ca4314783B0b36b329"),
        ),
        (
            "fraxswap_router",
            address!("C14d550632db8592D1243Edc8B95b0Ad06703867"),
        ),
    ];

    entries.iter().copied().collect()
});

/// Classify a factory/manager address into a protocol label.
pub fn get_pool_protocol(factory: Address) -> Option<&'static str> {
    if factory == POOL_FACTORIES["univ2_factory"] {
        Some("univ2")
    } else if factory == POOL_FACTORIES["univ3_factory"] {
        Some("univ3")
    } else if factory == POOL_FACTORIES["univ4_pool_manager"] {
        Some("univ4")
    } else if factory == POOL_FACTORIES["sushi_factory"] {
        Some("sushi")
    } else if factory == POOL_FACTORIES["pancake_factory"] {
        Some("pancake")
    } else if factory == POOL_FACTORIES["pancake_v3_factory"] {
        Some("pancake_v3")
    } else if factory == POOL_FACTORIES["curve_registry"]
        || factory == POOL_FACTORIES["curve_factory"]
    {
        Some("curve")
    } else if factory == POOL_FACTORIES["balancer_vault"] {
        Some("balancer")
    } else if factory == POOL_FACTORIES["kyber_dmm_factory"] {
        Some("kyber")
    } else if factory == POOL_FACTORIES["dodo_v2_factory"]
        || factory == POOL_FACTORIES["dodo_dpp_factory"]
    {
        Some("dodo")
    } else if factory == POOL_FACTORIES["oneinch_v2_factory"] {
        Some("oneinch")
    } else if factory == POOL_FACTORIES["shibaswap_factory"] {
        Some("shibaswap")
    } else if factory == POOL_FACTORIES["fraxswap_factory"] {
        Some("fraxswap")
    } else {
        None
    }
}

/// Whether the address is the Uniswap V4 Pool Manager singleton.
pub fn is_v4_pool_manager(address: Address) -> bool {
    address == POOL_FACTORIES["univ4_pool_manager"]
}

/// Whether the address is one of the factories we track.
pub fn is_known_factory(address: Address) -> bool {
    POOL_FACTORIES.values().any(|&addr| addr == address)
}
