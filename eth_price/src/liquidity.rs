use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
pub const USDC_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
pub const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
pub const DAI_ADDRESS: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";

const DEFAULT_ETH_USD_PRICE: f64 = 3_000.0;
const DEFAULT_DUST_USD: f64 = 25.0;
const DEFAULT_DRAINED_USD: f64 = 0.01;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolLiquidityLevel {
    Liquid,
    Dust,
    Drained,
    Unknown,
}

impl PoolLiquidityLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Liquid => "liquid",
            Self::Dust => "dust",
            Self::Drained => "drained",
            Self::Unknown => "unknown_quote",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::Liquid => 3,
            Self::Unknown => 2,
            Self::Dust => 1,
            Self::Drained => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DenomClass {
    Weth,
    Stablecoin,
    Other,
}

impl DenomClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Weth => "weth",
            Self::Stablecoin => "stablecoin",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LiquidityReference {
    pub eth_usd_price: f64,
    pub dust_usd: f64,
    pub drained_usd: f64,
}

impl Default for LiquidityReference {
    fn default() -> Self {
        Self {
            eth_usd_price: DEFAULT_ETH_USD_PRICE,
            dust_usd: DEFAULT_DUST_USD,
            drained_usd: DEFAULT_DRAINED_USD,
        }
    }
}

impl LiquidityReference {
    pub fn new(eth_usd_price: f64) -> Self {
        Self {
            eth_usd_price,
            ..Self::default()
        }
    }

    pub fn valid_eth_usd_price(self) -> Option<f64> {
        self.eth_usd_price
            .is_finite()
            .then_some(self.eth_usd_price)
            .filter(|price| *price > 0.0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PoolLiquidityAssessment {
    pub denom_class: DenomClass,
    pub level: PoolLiquidityLevel,
    pub denom_reserve: f64,
    pub value_eth: Option<f64>,
    pub value_usd: Option<f64>,
    pub rank_score: f64,
}

impl PoolLiquidityAssessment {
    pub fn compare_for_ranking(left: &Self, right: &Self) -> Ordering {
        right.level.rank().cmp(&left.level.rank()).then_with(|| {
            right
                .rank_score
                .partial_cmp(&left.rank_score)
                .unwrap_or(Ordering::Equal)
        })
    }
}

pub fn assess_denom_liquidity(
    denom_reserve: f64,
    denom_symbol: Option<&str>,
    denom_address: Option<&str>,
    reference: LiquidityReference,
) -> PoolLiquidityAssessment {
    let denom_class = classify_denom(denom_symbol, denom_address);
    if !denom_reserve.is_finite() || denom_reserve <= 0.0 {
        return PoolLiquidityAssessment {
            denom_class,
            level: PoolLiquidityLevel::Drained,
            denom_reserve,
            value_eth: None,
            value_usd: None,
            rank_score: 0.0,
        };
    }

    let (value_eth, value_usd) = match denom_class {
        DenomClass::Weth => {
            let value_usd = reference
                .valid_eth_usd_price()
                .map(|eth_usd_price| denom_reserve * eth_usd_price);
            (Some(denom_reserve), value_usd)
        }
        DenomClass::Stablecoin => {
            let value_eth = reference
                .valid_eth_usd_price()
                .map(|eth_usd_price| denom_reserve / eth_usd_price);
            (value_eth, Some(denom_reserve))
        }
        DenomClass::Other => (None, None),
    };

    let Some(value_usd) = value_usd.filter(|value| value.is_finite() && *value >= 0.0) else {
        return PoolLiquidityAssessment {
            denom_class,
            level: PoolLiquidityLevel::Unknown,
            denom_reserve,
            value_eth,
            value_usd: None,
            rank_score: 0.0,
        };
    };

    let level = if value_usd <= reference.drained_usd {
        PoolLiquidityLevel::Drained
    } else if value_usd <= reference.dust_usd {
        PoolLiquidityLevel::Dust
    } else {
        PoolLiquidityLevel::Liquid
    };

    PoolLiquidityAssessment {
        denom_class,
        level,
        denom_reserve,
        value_eth,
        value_usd: Some(value_usd),
        rank_score: value_usd,
    }
}

pub fn classify_denom(denom_symbol: Option<&str>, denom_address: Option<&str>) -> DenomClass {
    if denom_symbol
        .map(|symbol| normalize_symbol(symbol) == "WETH" || normalize_symbol(symbol) == "ETH")
        .unwrap_or(false)
        || denom_address
            .map(|address| normalize_address(address) == WETH_ADDRESS)
            .unwrap_or(false)
    {
        return DenomClass::Weth;
    }

    if denom_symbol
        .map(|symbol| matches!(normalize_symbol(symbol).as_str(), "USDC" | "USDT" | "DAI"))
        .unwrap_or(false)
        || denom_address
            .map(|address| {
                matches!(
                    normalize_address(address).as_str(),
                    USDC_ADDRESS | USDT_ADDRESS | DAI_ADDRESS
                )
            })
            .unwrap_or(false)
    {
        return DenomClass::Stablecoin;
    }

    DenomClass::Other
}

fn normalize_symbol(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_weth_and_stable_liquidity_in_usd_terms() {
        let reference = LiquidityReference::new(2_500.0);
        let weth = assess_denom_liquidity(1.0, Some("WETH"), None, reference);
        let stable = assess_denom_liquidity(2_000.0, Some("USDC"), None, reference);

        assert_eq!(weth.value_usd, Some(2_500.0));
        assert_eq!(stable.value_usd, Some(2_000.0));
        assert_eq!(
            PoolLiquidityAssessment::compare_for_ranking(&weth, &stable),
            Ordering::Less
        );
    }

    #[test]
    fn classifies_common_denom_addresses() {
        assert_eq!(classify_denom(None, Some(WETH_ADDRESS)), DenomClass::Weth);
        assert_eq!(
            classify_denom(None, Some(USDC_ADDRESS)),
            DenomClass::Stablecoin
        );
        assert_eq!(classify_denom(Some("TOKEN"), None), DenomClass::Other);
    }

    #[test]
    fn uses_usd_thresholds_for_liquidity_level() {
        let reference = LiquidityReference::new(3_000.0);
        assert_eq!(
            assess_denom_liquidity(0.001, Some("WETH"), None, reference).level,
            PoolLiquidityLevel::Dust
        );
        assert_eq!(
            assess_denom_liquidity(25.1, Some("USDC"), None, reference).level,
            PoolLiquidityLevel::Liquid
        );
        assert_eq!(
            assess_denom_liquidity(0.0, Some("USDC"), None, reference).level,
            PoolLiquidityLevel::Drained
        );
    }
}
