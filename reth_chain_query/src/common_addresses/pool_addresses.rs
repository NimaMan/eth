//! Known DEX factory/router addresses and helpers for protocol classification.

use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Known Ethereum mainnet V2-style protocols that share the Uniswap V2 router ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownV2Protocol {
    UniswapV2,
    SushiSwapV2,
    PancakeSwapV2,
    ShibaSwapV2,
    FraxswapV2,
}

/// Known Ethereum mainnet V3-style protocols that share the Uniswap V3 periphery ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownV3Protocol {
    UniswapV3,
    SushiSwapV3,
    PancakeSwapV3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownV2ProtocolDescriptor {
    pub protocol: KnownV2Protocol,
    pub label: &'static str,
    pub factory: Address,
    pub router: Address,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownV3ProtocolDescriptor {
    pub protocol: KnownV3Protocol,
    pub label: &'static str,
    pub factory: Address,
    pub router: Address,
}

impl KnownV2Protocol {
    pub const ALL: [KnownV2Protocol; 5] = [
        KnownV2Protocol::UniswapV2,
        KnownV2Protocol::SushiSwapV2,
        KnownV2Protocol::PancakeSwapV2,
        KnownV2Protocol::ShibaSwapV2,
        KnownV2Protocol::FraxswapV2,
    ];

    pub fn descriptor(self) -> KnownV2ProtocolDescriptor {
        match self {
            Self::UniswapV2 => KnownV2ProtocolDescriptor {
                protocol: self,
                label: "UNISWAP-V2",
                factory: address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
                router: address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            },
            Self::SushiSwapV2 => KnownV2ProtocolDescriptor {
                protocol: self,
                label: "SUSHISWAP-V2",
                factory: address!("C0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"),
                router: address!("d9e1cE17f2641f24aE83637ab66a2cca9C378B9F"),
            },
            Self::PancakeSwapV2 => KnownV2ProtocolDescriptor {
                protocol: self,
                label: "PANCAKESWAP-V2",
                factory: address!("1097053Fd2ea711dad45caCcc45EfF7548fCB362"),
                router: address!("EfF92A263d31888d860bD50809A8D171709b7b1c"),
            },
            Self::ShibaSwapV2 => KnownV2ProtocolDescriptor {
                protocol: self,
                label: "SHIBASWAP-V2",
                factory: address!("115934131916C8b277DD010Ee02de363c09d037c"),
                router: address!("03f7724180AA6b939894B5Ca4314783B0b36b329"),
            },
            Self::FraxswapV2 => KnownV2ProtocolDescriptor {
                protocol: self,
                label: "FRAXSWAP-V2",
                factory: address!("43eC799eAdd63848443E2347C49f5f52e8Fe0F6f"),
                router: address!("C14d550632db8592D1243Edc8B95b0Ad06703867"),
            },
        }
    }

    pub fn label(self) -> &'static str {
        self.descriptor().label
    }

    pub fn factory(self) -> Address {
        self.descriptor().factory
    }

    pub fn router(self) -> Address {
        self.descriptor().router
    }

    pub fn from_factory(factory: Address) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|protocol| protocol.factory() == factory)
    }

    pub fn from_label(label: &str) -> Option<Self> {
        let normalized = label
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .collect::<String>();
        match normalized.as_str() {
            "UNISWAPV2" | "UNIV2" => return Some(Self::UniswapV2),
            "SUSHISWAP" | "SUSHISWAPV2" | "SUSHISWAP2" => return Some(Self::SushiSwapV2),
            "PANCAKESWAP" | "PANCAKESWAPV2" | "PANCAKEV2" => {
                return Some(Self::PancakeSwapV2);
            }
            "SHIBASWAP" | "SHIBASWAPV2" => return Some(Self::ShibaSwapV2),
            "FRAXSWAP" | "FRAXSWAPV2" => return Some(Self::FraxswapV2),
            _ => {}
        }
        Self::ALL.iter().copied().find(|protocol| {
            protocol
                .label()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_uppercase())
                .collect::<String>()
                == normalized
        })
    }
}

impl KnownV3Protocol {
    pub const ALL: [KnownV3Protocol; 3] = [
        KnownV3Protocol::UniswapV3,
        KnownV3Protocol::SushiSwapV3,
        KnownV3Protocol::PancakeSwapV3,
    ];

    pub fn descriptor(self) -> KnownV3ProtocolDescriptor {
        match self {
            Self::UniswapV3 => KnownV3ProtocolDescriptor {
                protocol: self,
                label: "UNISWAP-V3",
                factory: address!("1F98431c8aD98523631AE4a59f267346ea31F984"),
                router: address!("E592427A0AEce92De3Edee1F18E0157C05861564"),
            },
            Self::SushiSwapV3 => KnownV3ProtocolDescriptor {
                protocol: self,
                label: "SUSHISWAP-V3",
                factory: address!("bACEB8eC6b9355Dfc0269C18bac9d6E2Bdc29C4F"),
                router: address!("2E6cd2d30aa43f40aa81619ff4b6E0a41479B13F"),
            },
            Self::PancakeSwapV3 => KnownV3ProtocolDescriptor {
                protocol: self,
                label: "PANCAKESWAP-V3",
                factory: address!("0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
                router: address!("13f4EA83D0bd40E75C8222255bc855a974568Dd4"),
            },
        }
    }

    pub fn label(self) -> &'static str {
        self.descriptor().label
    }

    pub fn factory(self) -> Address {
        self.descriptor().factory
    }

    pub fn router(self) -> Address {
        self.descriptor().router
    }

    pub fn from_factory(factory: Address) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|protocol| protocol.factory() == factory)
    }

    pub fn from_label(label: &str) -> Option<Self> {
        let normalized = label
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .collect::<String>();
        match normalized.as_str() {
            "UNISWAPV3" | "UNIV3" => Some(Self::UniswapV3),
            "SUSHISWAPV3" | "SUSHIV3" => Some(Self::SushiSwapV3),
            "PANCAKESWAPV3" | "PANCAKEV3" => Some(Self::PancakeSwapV3),
            _ => Self::ALL.iter().copied().find(|protocol| {
                protocol
                    .label()
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| c.to_ascii_uppercase())
                    .collect::<String>()
                    == normalized
            }),
        }
    }
}

/// Named map of pool factory / registry contracts we track.
pub static POOL_FACTORIES: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let entries: &[(&str, Address)] = &[
        ("univ2_factory", KnownV2Protocol::UniswapV2.factory()),
        ("univ3_factory", KnownV3Protocol::UniswapV3.factory()),
        (
            "univ4_pool_manager",
            address!("000000000004444C5DC75cB358380d2E3de08a90"),
        ),
        ("sushi_factory", KnownV2Protocol::SushiSwapV2.factory()),
        ("sushi_v3_factory", KnownV3Protocol::SushiSwapV3.factory()),
        ("pancake_factory", KnownV2Protocol::PancakeSwapV2.factory()),
        (
            "pancake_v3_factory",
            KnownV3Protocol::PancakeSwapV3.factory(),
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
        ("shibaswap_factory", KnownV2Protocol::ShibaSwapV2.factory()),
        ("fraxswap_factory", KnownV2Protocol::FraxswapV2.factory()),
    ];

    entries.iter().copied().collect()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_v2_protocol_classifies_pancakeswap_ethereum_factory() {
        let factory = address!("1097053Fd2ea711dad45caCcc45EfF7548fCB362");

        assert_eq!(
            KnownV2Protocol::from_factory(factory),
            Some(KnownV2Protocol::PancakeSwapV2)
        );
        assert_eq!(KnownV2Protocol::PancakeSwapV2.label(), "PANCAKESWAP-V2");
        assert_eq!(
            KnownV2Protocol::PancakeSwapV2.router(),
            address!("EfF92A263d31888d860bD50809A8D171709b7b1c")
        );
        assert_eq!(POOL_FACTORIES["pancake_factory"], factory);
    }

    #[test]
    fn known_v2_protocol_classifies_labels() {
        assert_eq!(
            KnownV2Protocol::from_label("SUSHISWAP-V2"),
            Some(KnownV2Protocol::SushiSwapV2)
        );
        assert_eq!(
            KnownV2Protocol::from_label("pancakeswap_v2"),
            Some(KnownV2Protocol::PancakeSwapV2)
        );
    }

    #[test]
    fn known_v3_protocol_classifies_pancakeswap_ethereum_factory() {
        let factory = address!("0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865");

        assert_eq!(
            KnownV3Protocol::from_factory(factory),
            Some(KnownV3Protocol::PancakeSwapV3)
        );
        assert_eq!(KnownV3Protocol::PancakeSwapV3.label(), "PANCAKESWAP-V3");
        assert_eq!(
            KnownV3Protocol::PancakeSwapV3.router(),
            address!("13f4EA83D0bd40E75C8222255bc855a974568Dd4")
        );
        assert_eq!(
            KnownV3Protocol::from_label("pancake_v3"),
            Some(KnownV3Protocol::PancakeSwapV3)
        );
    }
}

/// Named map of router contracts.
pub static ROUTERS: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let entries: &[(&str, Address)] = &[
        ("univ2_router", KnownV2Protocol::UniswapV2.router()),
        ("univ3_router", KnownV3Protocol::UniswapV3.router()),
        (
            "univ3_router2",
            address!("68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"),
        ),
        ("sushi_router", KnownV2Protocol::SushiSwapV2.router()),
        ("sushi_v3_router", KnownV3Protocol::SushiSwapV3.router()),
        ("pancake_router", KnownV2Protocol::PancakeSwapV2.router()),
        ("pancake_v3_router", KnownV3Protocol::PancakeSwapV3.router()),
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
        ("shibaswap_router", KnownV2Protocol::ShibaSwapV2.router()),
        ("fraxswap_router", KnownV2Protocol::FraxswapV2.router()),
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
    } else if factory == POOL_FACTORIES["sushi_v3_factory"] {
        Some("sushi_v3")
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
