//! Shared pool eligibility rules for ETH token trading research and strategies.
//!
//! This crate is intentionally small and data-oriented. It owns the first-pass
//! split between pools that a strategy may consider and pools that should stay
//! in non-eligible research buckets.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ETH_ELIGIBLE_LIQUIDITY: f64 = 0.5;
pub const STABLE_ELIGIBLE_LIQUIDITY: f64 = 500.0;
pub const ETH_DUST_LIQUIDITY: f64 = 0.01;
pub const STABLE_DUST_LIQUIDITY: f64 = 10.0;
pub const ETH_LOW_LIQUIDITY: f64 = 1.0;
pub const STABLE_LOW_LIQUIDITY: f64 = 1_000.0;

const DEFAULT_SUPPORTED_QUOTES: [&str; 4] = ["ETH", "WETH", "USDC", "USDT"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EligibilityConfig {
    pub supported_quote_symbols: Vec<String>,
    pub min_eth_liquidity: f64,
    pub min_stable_liquidity: f64,
    pub dust_eth_liquidity: f64,
    pub dust_stable_liquidity: f64,
    pub low_eth_liquidity: f64,
    pub low_stable_liquidity: f64,
    pub require_buy: bool,
    pub require_sell: bool,
    pub reject_risk: bool,
    pub require_creation_data: bool,
    pub require_price_history: bool,
}

impl Default for EligibilityConfig {
    fn default() -> Self {
        Self {
            supported_quote_symbols: DEFAULT_SUPPORTED_QUOTES
                .into_iter()
                .map(str::to_string)
                .collect(),
            min_eth_liquidity: ETH_ELIGIBLE_LIQUIDITY,
            min_stable_liquidity: STABLE_ELIGIBLE_LIQUIDITY,
            dust_eth_liquidity: ETH_DUST_LIQUIDITY,
            dust_stable_liquidity: STABLE_DUST_LIQUIDITY,
            low_eth_liquidity: ETH_LOW_LIQUIDITY,
            low_stable_liquidity: STABLE_LOW_LIQUIDITY,
            require_buy: true,
            require_sell: true,
            reject_risk: true,
            require_creation_data: false,
            require_price_history: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolEligibilityInput {
    #[serde(
        default,
        alias = "currency",
        alias = "denom_symbol",
        alias = "denomSymbol"
    )]
    pub quote_symbol: Option<String>,
    #[serde(default)]
    pub denom_reserve: Option<f64>,
    #[serde(default)]
    pub token_reserve: Option<f64>,
    #[serde(default)]
    pub can_buy: bool,
    #[serde(default)]
    pub can_sell: bool,
    #[serde(default, alias = "isScam", alias = "scam")]
    pub is_scam: bool,
    #[serde(default)]
    pub creation_block: Option<u64>,
    #[serde(default)]
    pub creation_timestamp: Option<u64>,
    #[serde(default)]
    pub has_price_history: bool,
}

impl PoolEligibilityInput {
    pub fn new(
        quote_symbol: Option<impl Into<String>>,
        denom_reserve: Option<f64>,
        can_buy: bool,
        can_sell: bool,
        is_scam: bool,
    ) -> Self {
        Self {
            quote_symbol: quote_symbol.map(Into::into),
            denom_reserve,
            token_reserve: None,
            can_buy,
            can_sell,
            is_scam,
            creation_block: None,
            creation_timestamp: None,
            has_price_history: false,
        }
    }

    pub fn with_creation_data(
        mut self,
        creation_block: Option<u64>,
        creation_timestamp: Option<u64>,
    ) -> Self {
        self.creation_block = creation_block;
        self.creation_timestamp = creation_timestamp;
        self
    }

    pub fn with_price_history(mut self, has_price_history: bool) -> Self {
        self.has_price_history = has_price_history;
        self
    }

    pub fn from_json_value(value: &Value) -> serde_json::Result<Self> {
        let mut input: Self = serde_json::from_value(value.clone())?;
        if input.quote_symbol.is_none() {
            input.quote_symbol = string_field(value, &["currency", "denom_symbol", "denomSymbol"]);
        }
        if input.denom_reserve.is_none() {
            input.denom_reserve = number_field(value, &["denom_reserve", "denomReserve"]);
        }
        if input.token_reserve.is_none() {
            input.token_reserve = number_field(value, &["token_reserve", "tokenReserve"]);
        }
        if !input.is_scam {
            input.is_scam = risk_field(value);
        }
        if !input.has_price_history {
            input.has_price_history = has_price_history(value);
        }
        Ok(input)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EligibilityDecision {
    pub eligible: bool,
    pub reason: Option<NonEligibleReason>,
    pub reason_key: Option<&'static str>,
    pub reason_label: Option<&'static str>,
    pub quote_symbol: Option<String>,
    pub denom_reserve: Option<f64>,
    pub min_liquidity: Option<f64>,
    pub meaningful_liquidity: Option<f64>,
    pub low_liquidity: Option<f64>,
    pub liquidity_class: LiquidityClass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NonEligibleReason {
    UnsupportedCurrency,
    MissingCreationData,
    LowLiquidity,
    CannotBuy,
    CannotSell,
    RiskBlocked,
    MissingPriceData,
}

impl NonEligibleReason {
    pub fn key(self) -> &'static str {
        match self {
            Self::UnsupportedCurrency => "unsupported_currency",
            Self::MissingCreationData => "missing_creation_data",
            Self::LowLiquidity => "low_liquidity",
            Self::CannotBuy => "cannot_buy",
            Self::CannotSell => "cannot_sell",
            Self::RiskBlocked => "risk_blocked",
            Self::MissingPriceData => "missing_price_data",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::UnsupportedCurrency => "Unsupported currency",
            Self::MissingCreationData => "Missing creation data",
            Self::LowLiquidity => "Low liquidity",
            Self::CannotBuy => "Cannot buy",
            Self::CannotSell => "Cannot sell",
            Self::RiskBlocked => "Risk blocked",
            Self::MissingPriceData => "Missing price data",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityClass {
    UnsupportedCurrency,
    Unknown,
    Dust,
    Low,
    Eligible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuoteFamily {
    Eth,
    Stable,
}

pub fn evaluate_pool(input: &PoolEligibilityInput) -> EligibilityDecision {
    evaluate_pool_with_config(input, &EligibilityConfig::default())
}

pub fn evaluate_pool_with_config(
    input: &PoolEligibilityInput,
    config: &EligibilityConfig,
) -> EligibilityDecision {
    let quote_symbol = normalized_quote_symbol(input.quote_symbol.as_deref());
    let thresholds = quote_symbol
        .as_deref()
        .and_then(|symbol| thresholds_for_quote(symbol, config));
    let liquidity_class = liquidity_class_for_input(input, config);
    let reason = match (quote_symbol.as_deref(), thresholds) {
        (Some(symbol), Some(_)) if config.supports_quote_symbol(symbol) => None,
        _ => Some(NonEligibleReason::UnsupportedCurrency),
    }
    .or_else(|| {
        if config.require_creation_data
            && (input.creation_block.is_none() || input.creation_timestamp.is_none())
        {
            Some(NonEligibleReason::MissingCreationData)
        } else {
            None
        }
    })
    .or_else(|| {
        let min_liquidity = thresholds.map(|thresholds| thresholds.eligible);
        match (finite_positive(input.denom_reserve), min_liquidity) {
            (Some(liquidity), Some(min_liquidity)) if liquidity >= min_liquidity => None,
            _ => Some(NonEligibleReason::LowLiquidity),
        }
    })
    .or_else(|| {
        if config.require_buy && !input.can_buy {
            Some(NonEligibleReason::CannotBuy)
        } else {
            None
        }
    })
    .or_else(|| {
        if config.require_sell && !input.can_sell {
            Some(NonEligibleReason::CannotSell)
        } else {
            None
        }
    })
    .or_else(|| {
        if config.reject_risk && input.is_scam {
            Some(NonEligibleReason::RiskBlocked)
        } else {
            None
        }
    })
    .or_else(|| {
        if config.require_price_history && !input.has_price_history {
            Some(NonEligibleReason::MissingPriceData)
        } else {
            None
        }
    });

    EligibilityDecision {
        eligible: reason.is_none(),
        reason,
        reason_key: reason.map(NonEligibleReason::key),
        reason_label: reason.map(NonEligibleReason::label),
        quote_symbol,
        denom_reserve: finite_positive(input.denom_reserve),
        min_liquidity: thresholds.map(|thresholds| thresholds.eligible),
        meaningful_liquidity: thresholds.map(|thresholds| thresholds.dust),
        low_liquidity: thresholds.map(|thresholds| thresholds.low),
        liquidity_class,
    }
}

pub fn eligibility_label(input: &PoolEligibilityInput) -> &'static str {
    evaluate_pool(input).reason_key.unwrap_or("eligible")
}

pub fn is_supported_quote_symbol(symbol: &str) -> bool {
    EligibilityConfig::default().supports_quote_symbol(symbol)
}

pub fn eligible_liquidity_threshold(symbol: &str) -> Option<f64> {
    thresholds_for_quote(symbol, &EligibilityConfig::default())
        .map(|thresholds| thresholds.eligible)
}

pub fn meaningful_liquidity_threshold(symbol: &str) -> Option<f64> {
    thresholds_for_quote(symbol, &EligibilityConfig::default()).map(|thresholds| thresholds.dust)
}

pub fn low_liquidity_threshold(symbol: &str) -> Option<f64> {
    thresholds_for_quote(symbol, &EligibilityConfig::default()).map(|thresholds| thresholds.low)
}

pub fn liquidity_class_for_input(
    input: &PoolEligibilityInput,
    config: &EligibilityConfig,
) -> LiquidityClass {
    let Some(symbol) = normalized_quote_symbol(input.quote_symbol.as_deref()) else {
        return LiquidityClass::UnsupportedCurrency;
    };
    let Some(thresholds) = thresholds_for_quote(&symbol, config) else {
        return LiquidityClass::UnsupportedCurrency;
    };
    if !config.supports_quote_symbol(&symbol) {
        return LiquidityClass::UnsupportedCurrency;
    }
    let Some(liquidity) = finite_positive(input.denom_reserve) else {
        return LiquidityClass::Unknown;
    };
    if liquidity < thresholds.dust {
        LiquidityClass::Dust
    } else if liquidity < thresholds.eligible {
        LiquidityClass::Low
    } else {
        LiquidityClass::Eligible
    }
}

impl EligibilityConfig {
    pub fn strategy_stats() -> Self {
        Self {
            require_creation_data: true,
            require_price_history: true,
            ..Self::default()
        }
    }

    pub fn supports_quote_symbol(&self, symbol: &str) -> bool {
        let normalized = symbol.trim().to_ascii_uppercase();
        self.supported_quote_symbols
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(&normalized))
    }
}

#[derive(Clone, Copy, Debug)]
struct LiquidityThresholds {
    eligible: f64,
    dust: f64,
    low: f64,
}

fn thresholds_for_quote(symbol: &str, config: &EligibilityConfig) -> Option<LiquidityThresholds> {
    match quote_family(symbol)? {
        QuoteFamily::Eth => Some(LiquidityThresholds {
            eligible: config.min_eth_liquidity,
            dust: config.dust_eth_liquidity,
            low: config.low_eth_liquidity,
        }),
        QuoteFamily::Stable => Some(LiquidityThresholds {
            eligible: config.min_stable_liquidity,
            dust: config.dust_stable_liquidity,
            low: config.low_stable_liquidity,
        }),
    }
}

fn quote_family(symbol: &str) -> Option<QuoteFamily> {
    match symbol.trim().to_ascii_uppercase().as_str() {
        "ETH" | "WETH" => Some(QuoteFamily::Eth),
        "USDC" | "USDT" => Some(QuoteFamily::Stable),
        _ => None,
    }
}

fn normalized_quote_symbol(symbol: Option<&str>) -> Option<String> {
    symbol
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(str::to_ascii_uppercase)
}

fn finite_positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string)
    })
}

fn number_field(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
                .filter(|number| number.is_finite())
        })
    })
}

fn risk_field(value: &Value) -> bool {
    [
        "risk_level",
        "riskLevel",
        "risk_label",
        "riskLabel",
        "scam_label",
        "scamLabel",
    ]
    .iter()
    .filter_map(|key| value.get(*key))
    .any(risk_value_truthy)
}

fn risk_value_truthy(value: &Value) -> bool {
    if value.as_bool().unwrap_or(false) {
        return true;
    }
    let Some(text) = value.as_str() else {
        return false;
    };
    let text = text.trim().to_ascii_lowercase();
    if text.is_empty() || matches!(text.as_str(), "clear" | "ok" | "none" | "unknown" | "-") {
        return false;
    }
    [
        "liquidity_removal",
        "honeypot",
        "scam",
        "rug",
        "blocked",
        "avoid",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn has_price_history(value: &Value) -> bool {
    value
        .get("price_ratio_history")
        .or_else(|| value.get("priceRatioHistory"))
        .and_then(Value::as_array)
        .map(|history| !history.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PoolEligibilityInput {
        PoolEligibilityInput::new(Some("WETH".to_string()), Some(1.0), true, true, false)
            .with_creation_data(Some(100), Some(1_700_000_000))
            .with_price_history(true)
    }

    #[test]
    fn accepts_weth_pool_at_default_floor() {
        let decision = evaluate_pool(&input());

        assert!(decision.eligible);
        assert_eq!(decision.reason, None);
        assert_eq!(decision.min_liquidity, Some(ETH_ELIGIBLE_LIQUIDITY));
        assert_eq!(decision.liquidity_class, LiquidityClass::Eligible);
    }

    #[test]
    fn rejects_weth_pool_below_default_floor() {
        let mut input = input();
        input.denom_reserve = Some(0.49);

        let decision = evaluate_pool(&input);

        assert!(!decision.eligible);
        assert_eq!(decision.reason, Some(NonEligibleReason::LowLiquidity));
        assert_eq!(decision.liquidity_class, LiquidityClass::Low);
    }

    #[test]
    fn rejects_unsupported_quote_before_liquidity_checks() {
        let mut input = input();
        input.quote_symbol = Some("DAI".to_string());
        input.denom_reserve = Some(10_000.0);

        let decision = evaluate_pool(&input);

        assert_eq!(
            decision.reason,
            Some(NonEligibleReason::UnsupportedCurrency)
        );
        assert_eq!(
            decision.liquidity_class,
            LiquidityClass::UnsupportedCurrency
        );
    }

    #[test]
    fn uses_stable_floor_for_usdc_and_usdt() {
        let mut input = input();
        input.quote_symbol = Some("USDC".to_string());
        input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY);
        assert!(evaluate_pool(&input).eligible);

        input.quote_symbol = Some("USDT".to_string());
        input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY - 0.01);
        assert_eq!(
            evaluate_pool(&input).reason,
            Some(NonEligibleReason::LowLiquidity)
        );
    }

    #[test]
    fn stats_config_requires_creation_and_price_history() {
        let mut input = input();
        input.creation_timestamp = None;
        input.has_price_history = false;

        let decision = evaluate_pool_with_config(&input, &EligibilityConfig::strategy_stats());
        assert_eq!(
            decision.reason,
            Some(NonEligibleReason::MissingCreationData)
        );

        input.creation_timestamp = Some(1_700_000_000);
        let decision = evaluate_pool_with_config(&input, &EligibilityConfig::strategy_stats());
        assert_eq!(decision.reason, Some(NonEligibleReason::MissingPriceData));
    }

    #[test]
    fn json_input_recognizes_server_pool_fields() {
        let value = serde_json::json!({
            "currency": "WETH",
            "denom_reserve": 1.0,
            "can_buy": true,
            "can_sell": true,
            "risk_level": "clear",
            "price_ratio_history": [{"block_number": 1, "ratio": 2.0}]
        });
        let input = PoolEligibilityInput::from_json_value(&value).unwrap();

        assert!(input.has_price_history);
        assert!(evaluate_pool(&input).eligible);
    }

    #[test]
    fn json_input_maps_liquidity_removal_risk_to_blocked() {
        let value = serde_json::json!({
            "currency": "WETH",
            "denom_reserve": 1.0,
            "can_buy": true,
            "can_sell": true,
            "risk_level": "liquidity_removal"
        });
        let input = PoolEligibilityInput::from_json_value(&value).unwrap();

        assert_eq!(
            evaluate_pool(&input).reason,
            Some(NonEligibleReason::RiskBlocked)
        );
    }
}
