//! Shared pool classification rules for ETH token trading research and strategies.
//!
//! This crate is intentionally small and data-oriented. It owns the first-pass
//! split between pools that entered the tradable research cohort and pools that
//! should stay in non-eligible research buckets. It also owns the current
//! category for an eligible pool, such as active, liquidity removed, hidden
//! mint, or other risk outcomes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ETH_ELIGIBLE_LIQUIDITY: f64 = 0.5;
pub const STABLE_ELIGIBLE_LIQUIDITY: f64 = 1_000.0;
pub const ETH_DUST_LIQUIDITY: f64 = 0.01;
pub const STABLE_DUST_LIQUIDITY: f64 = 10.0;
pub const ETH_LOW_LIQUIDITY: f64 = 1.0;
pub const STABLE_LOW_LIQUIDITY: f64 = 1_000.0;

pub const LP_APPROVAL_EXPOSURE_THRESHOLD: f64 = 20.0;
pub const LP_HOLDER_CONCENTRATION_THRESHOLD: f64 = 90.0;

const DEFAULT_SUPPORTED_QUOTES: [&str; 5] = ["ETH", "WETH", "USDC", "USDT", "DAI"];
const SIGNIFICANT_LIQUIDITY_DROP_RATIO: f64 = 0.80;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolClassificationConfig {
    pub supported_quote_symbols: Vec<String>,
    pub min_eth_liquidity: f64,
    pub min_stable_liquidity: f64,
    pub dust_eth_liquidity: f64,
    pub dust_stable_liquidity: f64,
    pub low_eth_liquidity: f64,
    pub low_stable_liquidity: f64,
    pub require_buy: bool,
    pub require_sell: bool,
    pub reject_risk_from_cohort: bool,
    pub require_creation_data: bool,
    pub require_price_history: bool,
}

impl Default for PoolClassificationConfig {
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
            reject_risk_from_cohort: false,
            require_creation_data: false,
            require_price_history: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolClassificationInput {
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
    pub max_denom_reserve: Option<f64>,
    #[serde(default)]
    pub token_reserve: Option<f64>,
    #[serde(default)]
    pub can_buy: bool,
    #[serde(default)]
    pub can_sell: bool,
    #[serde(
        default,
        alias = "cohortCanBuy",
        alias = "ever_can_buy",
        alias = "everCanBuy"
    )]
    pub cohort_can_buy: Option<bool>,
    #[serde(
        default,
        alias = "cohortCanSell",
        alias = "ever_can_sell",
        alias = "everCanSell"
    )]
    pub cohort_can_sell: Option<bool>,
    #[serde(default, alias = "isScam", alias = "scam")]
    pub is_scam: bool,
    #[serde(default)]
    pub hidden_mint: bool,
    #[serde(default)]
    pub liquidity_removed: bool,
    #[serde(default)]
    pub honeypot: bool,
    #[serde(default)]
    pub tax_bucket: Option<String>,
    #[serde(default)]
    pub creation_block: Option<u64>,
    #[serde(default)]
    pub creation_timestamp: Option<u64>,
    #[serde(default)]
    pub has_price_history: bool,
    #[serde(default)]
    pub lp_approved_percentage: Option<f64>,
    #[serde(default)]
    pub lp_max_holder_share: Option<f64>,
    #[serde(default)]
    pub supply_ratio_status: Option<String>,
    #[serde(default)]
    pub ownership_renounced: Option<bool>,
}

impl PoolClassificationInput {
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
            max_denom_reserve: denom_reserve,
            token_reserve: None,
            can_buy,
            can_sell,
            cohort_can_buy: None,
            cohort_can_sell: None,
            is_scam,
            hidden_mint: false,
            liquidity_removed: false,
            honeypot: false,
            tax_bucket: None,
            creation_block: None,
            creation_timestamp: None,
            has_price_history: false,
            lp_approved_percentage: None,
            lp_max_holder_share: None,
            supply_ratio_status: None,
            ownership_renounced: None,
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

    pub fn with_lp_safety(
        mut self,
        lp_approved_percentage: Option<f64>,
        lp_max_holder_share: Option<f64>,
    ) -> Self {
        self.lp_approved_percentage = lp_approved_percentage;
        self.lp_max_holder_share = lp_max_holder_share;
        self
    }

    pub fn with_supply_ratio_status(mut self, supply_ratio_status: Option<String>) -> Self {
        self.supply_ratio_status = supply_ratio_status;
        self
    }

    pub fn with_ownership_renounced(mut self, ownership_renounced: Option<bool>) -> Self {
        self.ownership_renounced = ownership_renounced;
        self
    }

    pub fn for_cohort_entry(&self) -> Self {
        let mut input = self.clone();
        input.max_denom_reserve = input.denom_reserve;
        input.cohort_can_buy = None;
        input.cohort_can_sell = None;
        input
    }

    pub fn from_json_value(value: &Value) -> serde_json::Result<Self> {
        let mut input: Self = serde_json::from_value(value.clone())?;
        if input.quote_symbol.is_none() {
            input.quote_symbol = string_field(value, &["currency", "denom_symbol", "denomSymbol"]);
        }
        if input.denom_reserve.is_none() {
            input.denom_reserve = number_field(value, &["denom_reserve", "denomReserve"]);
        }
        if input.max_denom_reserve.is_none() {
            input.max_denom_reserve = number_field(
                value,
                &[
                    "max_denom_reserve",
                    "maxDenomReserve",
                    "max_liquidity",
                    "maxLiquidity",
                ],
            );
        }
        if input.token_reserve.is_none() {
            input.token_reserve = number_field(value, &["token_reserve", "tokenReserve"]);
        }
        if !input.is_scam {
            input.is_scam = risk_field(value);
        }
        if !input.hidden_mint {
            input.hidden_mint = boolish_field(
                value,
                &[
                    "hidden_mint",
                    "hiddenMint",
                    "hidden_mint_detected",
                    "hiddenMintDetected",
                ],
            ) || risk_text_contains(value, &["hidden_mint", "hiddenmint"]);
        }
        if !input.liquidity_removed {
            input.liquidity_removed = boolish_field(
                value,
                &[
                    "liquidity_removed",
                    "liquidityRemoved",
                    "liquidity_removal",
                    "liquidityRemoval",
                ],
            ) || risk_text_contains(
                value,
                &["liquidity_removal", "denom_removal", "liquidity_removed"],
            );
        }
        if !input.honeypot {
            input.honeypot =
                boolish_field(value, &["honeypot", "cannot_sell_scam", "cannotSellScam"])
                    || risk_text_contains(value, &["honeypot"]);
        }
        if input.tax_bucket.is_none() {
            input.tax_bucket = string_field(value, &["tax_bucket", "taxBucket"]);
        }
        if !input.has_price_history {
            input.has_price_history = has_price_history(value);
        }
        if input.lp_approved_percentage.is_none() {
            input.lp_approved_percentage =
                number_field(value, &["lp_approved_percentage", "lpApprovedPercentage"]);
        }
        if input.lp_max_holder_share.is_none() {
            input.lp_max_holder_share =
                number_field(value, &["lp_max_holder_share", "lpMaxHolderShare"]);
        }
        if input.supply_ratio_status.is_none() {
            input.supply_ratio_status =
                string_field(value, &["supply_ratio_status", "supplyRatioStatus"]);
        }
        if input.ownership_renounced.is_none() {
            input.ownership_renounced =
                boolish_field(value, &["ownership_renounced", "ownershipRenounced"])
                    .then_some(true)
                    .or_else(|| {
                        boolish_field(value, &["ownership_not_renounced", "ownershipNotRenounced"])
                            .then_some(false)
                    });
        }
        Ok(input)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolEligibilityObservation {
    pub block_number: u64,
    pub input: PoolClassificationInput,
}

impl PoolEligibilityObservation {
    pub fn new(block_number: u64, input: PoolClassificationInput) -> Self {
        Self {
            block_number,
            input,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PoolEligibilityEntry {
    pub block_number: u64,
    pub input: PoolClassificationInput,
    pub classification: PoolClassification,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolClassification {
    pub cohort: PoolCohort,
    pub category: PoolCategory,
    pub tradable_now: bool,
    pub eligible_outcome: Option<EligiblePoolOutcome>,
    pub eligible: bool,
    pub reason: Option<NonEligibleReason>,
    pub reason_key: Option<&'static str>,
    pub reason_label: Option<&'static str>,
    pub quote_symbol: Option<String>,
    pub denom_reserve: Option<f64>,
    pub max_denom_reserve: Option<f64>,
    pub eligibility_liquidity: Option<f64>,
    pub min_liquidity: Option<f64>,
    pub meaningful_liquidity: Option<f64>,
    pub low_liquidity: Option<f64>,
    pub liquidity_class: LiquidityClass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolCohort {
    Eligible,
    Ineligible,
}

impl PoolCohort {
    pub fn key(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Ineligible => "ineligible",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Eligible => "Eligible",
            Self::Ineligible => "Ineligible",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolCategory {
    EligibleActive,
    EligibleRisk,
    Ineligible,
}

impl PoolCategory {
    pub fn key(self) -> &'static str {
        match self {
            Self::EligibleActive => "eligible_active",
            Self::EligibleRisk => "eligible_risk",
            Self::Ineligible => "ineligible",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::EligibleActive => "Eligible Active",
            Self::EligibleRisk => "Eligible Risk",
            Self::Ineligible => "Ineligible",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligiblePoolOutcome {
    Active,
    LiquidityRemoval,
    HiddenMint,
    Honeypot,
    CurrentLowLiquidity,
    CannotBuy,
    CannotSell,
    ExtremeTax,
    LpApprovalExposure,
    ConcentratedLpOwnership,
}

impl EligiblePoolOutcome {
    pub fn key(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::LiquidityRemoval => "liquidity_removal",
            Self::HiddenMint => "hidden_mint",
            Self::Honeypot => "honeypot",
            Self::CurrentLowLiquidity => "current_low_liquidity",
            Self::CannotBuy => "cannot_buy",
            Self::CannotSell => "cannot_sell",
            Self::ExtremeTax => "extreme_tax",
            Self::LpApprovalExposure => "lp_approval_exposure",
            Self::ConcentratedLpOwnership => "concentrated_lp_ownership",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::LiquidityRemoval => "Liquidity removal",
            Self::HiddenMint => "Hidden mint",
            Self::Honeypot => "Honeypot",
            Self::CurrentLowLiquidity => "Current low liquidity",
            Self::CannotBuy => "Cannot buy",
            Self::CannotSell => "Cannot sell",
            Self::ExtremeTax => "Extreme tax",
            Self::LpApprovalExposure => "LP approval exposure",
            Self::ConcentratedLpOwnership => "Concentrated LP ownership",
        }
    }
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

pub fn classify_pool(input: &PoolClassificationInput) -> PoolClassification {
    classify_pool_with_config(input, &PoolClassificationConfig::default())
}

pub fn first_eligible_observation(
    observations: impl IntoIterator<Item = PoolEligibilityObservation>,
) -> Option<PoolEligibilityEntry> {
    first_eligible_observation_with_config(observations, &PoolClassificationConfig::default())
}

pub fn first_eligible_observation_with_config(
    observations: impl IntoIterator<Item = PoolEligibilityObservation>,
    config: &PoolClassificationConfig,
) -> Option<PoolEligibilityEntry> {
    let mut observations: Vec<_> = observations.into_iter().collect();
    observations.sort_by_key(|observation| observation.block_number);

    observations.into_iter().find_map(|observation| {
        let entry_input = observation.input.for_cohort_entry();
        let classification = classify_pool_with_config(&entry_input, config);
        classification.eligible.then_some(PoolEligibilityEntry {
            block_number: observation.block_number,
            input: entry_input,
            classification,
        })
    })
}

pub fn classify_pool_with_config(
    input: &PoolClassificationInput,
    config: &PoolClassificationConfig,
) -> PoolClassification {
    let quote_symbol = normalized_quote_symbol(input.quote_symbol.as_deref());
    let thresholds = quote_symbol
        .as_deref()
        .and_then(|symbol| thresholds_for_quote(symbol, config));
    let liquidity_class = liquidity_class_for_input(input, config);
    let current_liquidity = finite_positive(input.denom_reserve);
    let max_liquidity = finite_positive(input.max_denom_reserve);
    let eligibility_liquidity = max_liquidity.or(current_liquidity);
    let liquidity_removed = input.liquidity_removed
        || derived_liquidity_removed(current_liquidity, max_liquidity, thresholds);
    let cohort_can_buy = input.cohort_can_buy.unwrap_or(input.can_buy);
    let cohort_can_sell = input.cohort_can_sell.unwrap_or(input.can_sell);
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
        match (eligibility_liquidity, min_liquidity) {
            (Some(liquidity), Some(min_liquidity)) if liquidity >= min_liquidity => None,
            _ => Some(NonEligibleReason::LowLiquidity),
        }
    })
    .or_else(|| {
        if config.require_buy && !cohort_can_buy {
            Some(NonEligibleReason::CannotBuy)
        } else {
            None
        }
    })
    .or_else(|| {
        if config.require_sell && !cohort_can_sell {
            Some(NonEligibleReason::CannotSell)
        } else {
            None
        }
    })
    .or_else(|| {
        if config.reject_risk_from_cohort
            && (input.is_scam || input.hidden_mint || liquidity_removed || input.honeypot)
        {
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

    let eligible = reason.is_none();
    let eligible_outcome =
        eligible.then(|| eligible_outcome(input, current_liquidity, thresholds, liquidity_removed));
    let cohort = if eligible {
        PoolCohort::Eligible
    } else {
        PoolCohort::Ineligible
    };
    let category = match eligible_outcome {
        None => PoolCategory::Ineligible,
        Some(EligiblePoolOutcome::Active) => PoolCategory::EligibleActive,
        Some(_) => PoolCategory::EligibleRisk,
    };

    PoolClassification {
        cohort,
        category,
        tradable_now: eligible_outcome == Some(EligiblePoolOutcome::Active),
        eligible_outcome,
        eligible,
        reason,
        reason_key: reason.map(NonEligibleReason::key),
        reason_label: reason.map(NonEligibleReason::label),
        quote_symbol,
        denom_reserve: current_liquidity,
        max_denom_reserve: max_liquidity,
        eligibility_liquidity,
        min_liquidity: thresholds.map(|thresholds| thresholds.eligible),
        meaningful_liquidity: thresholds.map(|thresholds| thresholds.dust),
        low_liquidity: thresholds.map(|thresholds| thresholds.low),
        liquidity_class,
    }
}

fn eligible_outcome(
    input: &PoolClassificationInput,
    current_liquidity: Option<f64>,
    thresholds: Option<LiquidityThresholds>,
    liquidity_removed: bool,
) -> EligiblePoolOutcome {
    if liquidity_removed {
        return EligiblePoolOutcome::LiquidityRemoval;
    }
    if input.hidden_mint {
        return EligiblePoolOutcome::HiddenMint;
    }
    if input.honeypot || (input.can_buy && !input.can_sell) {
        return EligiblePoolOutcome::Honeypot;
    }
    match normalized_tax_bucket(input.tax_bucket.as_deref()).as_deref() {
        Some("extreme_tax") => return EligiblePoolOutcome::ExtremeTax,
        _ => {}
    }
    if !current_liquidity_allows_entry(current_liquidity, thresholds) {
        return EligiblePoolOutcome::CurrentLowLiquidity;
    }
    if !input.can_buy {
        return EligiblePoolOutcome::CannotBuy;
    }
    if !input.can_sell {
        return EligiblePoolOutcome::CannotSell;
    }
    if let Some(approved) = input.lp_approved_percentage {
        if approved >= LP_APPROVAL_EXPOSURE_THRESHOLD {
            return EligiblePoolOutcome::LpApprovalExposure;
        }
    }
    if let Some(max_share) = input.lp_max_holder_share {
        if max_share >= LP_HOLDER_CONCENTRATION_THRESHOLD {
            return EligiblePoolOutcome::ConcentratedLpOwnership;
        }
    }
    EligiblePoolOutcome::Active
}

fn current_liquidity_allows_entry(
    current_liquidity: Option<f64>,
    thresholds: Option<LiquidityThresholds>,
) -> bool {
    match (current_liquidity, thresholds) {
        (Some(liquidity), Some(thresholds)) => liquidity >= thresholds.eligible,
        _ => false,
    }
}

fn derived_liquidity_removed(
    current_liquidity: Option<f64>,
    max_liquidity: Option<f64>,
    thresholds: Option<LiquidityThresholds>,
) -> bool {
    let (Some(current), Some(max_seen), Some(thresholds)) =
        (current_liquidity, max_liquidity, thresholds)
    else {
        return false;
    };
    if max_seen <= 0.0 || max_seen < thresholds.eligible || current >= thresholds.eligible {
        return false;
    }
    ((max_seen - current.max(0.0)) / max_seen) >= SIGNIFICANT_LIQUIDITY_DROP_RATIO
}

pub fn classification_label(input: &PoolClassificationInput) -> &'static str {
    let classification = classify_pool(input);
    classification.reason_key.unwrap_or_else(|| {
        classification
            .eligible_outcome
            .map(EligiblePoolOutcome::key)
            .unwrap_or_else(|| classification.category.key())
    })
}

pub fn is_supported_quote_symbol(symbol: &str) -> bool {
    PoolClassificationConfig::default().supports_quote_symbol(symbol)
}

pub fn eligible_liquidity_threshold(symbol: &str) -> Option<f64> {
    thresholds_for_quote(symbol, &PoolClassificationConfig::default())
        .map(|thresholds| thresholds.eligible)
}

pub fn meaningful_liquidity_threshold(symbol: &str) -> Option<f64> {
    thresholds_for_quote(symbol, &PoolClassificationConfig::default())
        .map(|thresholds| thresholds.dust)
}

pub fn low_liquidity_threshold(symbol: &str) -> Option<f64> {
    thresholds_for_quote(symbol, &PoolClassificationConfig::default())
        .map(|thresholds| thresholds.low)
}

pub fn liquidity_class_for_input(
    input: &PoolClassificationInput,
    config: &PoolClassificationConfig,
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

impl PoolClassificationConfig {
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

fn thresholds_for_quote(
    symbol: &str,
    config: &PoolClassificationConfig,
) -> Option<LiquidityThresholds> {
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
        "USDC" | "USDT" | "DAI" => Some(QuoteFamily::Stable),
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

fn boolish_field(value: &Value, keys: &[&str]) -> bool {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .any(boolish_value_truthy)
}

fn boolish_value_truthy(value: &Value) -> bool {
    if value.as_bool().unwrap_or(false) {
        return true;
    }
    if let Some(number) = value.as_i64() {
        return number != 0;
    }
    let Some(text) = value.as_str() else {
        return false;
    };
    let text = text.trim().to_ascii_lowercase();
    matches!(text.as_str(), "true" | "yes" | "1" | "scam" | "risk")
        || text.contains("hidden_mint")
        || text.contains("liquidity_removal")
        || text.contains("honeypot")
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

fn risk_text_contains(value: &Value, markers: &[&str]) -> bool {
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
    .filter_map(Value::as_str)
    .map(|text| {
        text.trim()
            .replace(' ', "_")
            .replace('-', "_")
            .to_ascii_lowercase()
    })
    .any(|text| markers.iter().any(|marker| text.contains(marker)))
}

fn normalized_tax_bucket(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.replace(' ', "_").to_ascii_lowercase())
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

    fn input() -> PoolClassificationInput {
        PoolClassificationInput::new(Some("WETH".to_string()), Some(1.0), true, true, false)
            .with_creation_data(Some(100), Some(1_700_000_000))
            .with_price_history(true)
    }

    #[test]
    fn accepts_weth_pool_at_default_floor() {
        let decision = classify_pool(&input());

        assert!(decision.eligible);
        assert_eq!(decision.reason, None);
        assert_eq!(decision.min_liquidity, Some(ETH_ELIGIBLE_LIQUIDITY));
        assert_eq!(decision.liquidity_class, LiquidityClass::Eligible);
    }

    #[test]
    fn rejects_weth_pool_below_default_floor() {
        let mut input = input();
        input.denom_reserve = Some(0.49);
        input.max_denom_reserve = input.denom_reserve;

        let decision = classify_pool(&input);

        assert!(!decision.eligible);
        assert_eq!(decision.reason, Some(NonEligibleReason::LowLiquidity));
        assert_eq!(decision.liquidity_class, LiquidityClass::Low);
    }

    #[test]
    fn rejects_unsupported_quote_before_liquidity_checks() {
        let mut input = input();
        input.quote_symbol = Some("WBTC".to_string());
        input.denom_reserve = Some(10_000.0);

        let decision = classify_pool(&input);

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
    fn uses_stable_floor_for_usdc_usdt_and_dai() {
        let mut input = input();
        input.quote_symbol = Some("USDC".to_string());
        input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY);
        input.max_denom_reserve = input.denom_reserve;
        assert!(classify_pool(&input).eligible);

        input.quote_symbol = Some("USDT".to_string());
        input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY - 0.01);
        input.max_denom_reserve = input.denom_reserve;
        assert_eq!(
            classify_pool(&input).reason,
            Some(NonEligibleReason::LowLiquidity)
        );

        input.quote_symbol = Some("DAI".to_string());
        input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY);
        input.max_denom_reserve = input.denom_reserve;
        assert!(classify_pool(&input).eligible);
    }

    #[test]
    fn stats_config_requires_creation_and_price_history() {
        let mut input = input();
        input.creation_timestamp = None;
        input.has_price_history = false;

        let decision =
            classify_pool_with_config(&input, &PoolClassificationConfig::strategy_stats());
        assert_eq!(
            decision.reason,
            Some(NonEligibleReason::MissingCreationData)
        );

        input.creation_timestamp = Some(1_700_000_000);
        let decision =
            classify_pool_with_config(&input, &PoolClassificationConfig::strategy_stats());
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
        let input = PoolClassificationInput::from_json_value(&value).unwrap();

        assert!(input.has_price_history);
        assert!(classify_pool(&input).eligible);
    }

    #[test]
    fn json_input_keeps_liquidity_removal_inside_eligible_cohort() {
        let value = serde_json::json!({
            "currency": "WETH",
            "denom_reserve": 1.0,
            "max_denom_reserve": 2.0,
            "can_buy": true,
            "can_sell": true,
            "risk_level": "liquidity_removal"
        });
        let input = PoolClassificationInput::from_json_value(&value).unwrap();
        let decision = classify_pool(&input);

        assert!(decision.eligible);
        assert_eq!(decision.cohort, PoolCohort::Eligible);
        assert_eq!(decision.category, PoolCategory::EligibleRisk);
        assert_eq!(
            decision.eligible_outcome,
            Some(EligiblePoolOutcome::LiquidityRemoval)
        );
        assert_eq!(decision.reason, None);
    }

    #[test]
    fn current_liquidity_drop_stays_eligible_as_liquidity_removal() {
        let mut input = input();
        input.denom_reserve = Some(0.03);
        input.max_denom_reserve = Some(1.2);

        let decision = classify_pool(&input);

        assert!(decision.eligible);
        assert_eq!(decision.reason, None);
        assert_eq!(decision.cohort, PoolCohort::Eligible);
        assert_eq!(decision.category, PoolCategory::EligibleRisk);
        assert_eq!(
            decision.eligible_outcome,
            Some(EligiblePoolOutcome::LiquidityRemoval)
        );
        assert_eq!(decision.eligibility_liquidity, Some(1.2));
        assert_eq!(decision.liquidity_class, LiquidityClass::Low);
    }

    #[test]
    fn current_liquidity_minor_dip_stays_eligible_as_current_low_liquidity() {
        let mut input = input();
        input.denom_reserve = Some(0.49);
        input.max_denom_reserve = Some(0.51);

        let decision = classify_pool(&input);

        assert!(decision.eligible);
        assert_eq!(decision.reason, None);
        assert_eq!(
            decision.eligible_outcome,
            Some(EligiblePoolOutcome::CurrentLowLiquidity)
        );
        assert!(!decision.tradable_now);
    }

    #[test]
    fn cohort_uses_ever_tradable_flags_but_current_sell_failure_is_outcome() {
        let mut input = input();
        input.can_buy = true;
        input.can_sell = false;
        input.cohort_can_buy = Some(true);
        input.cohort_can_sell = Some(true);

        let decision = classify_pool(&input);

        assert!(decision.eligible);
        assert_eq!(decision.reason, None);
        assert_eq!(
            decision.eligible_outcome,
            Some(EligiblePoolOutcome::Honeypot)
        );
        assert!(!decision.tradable_now);
    }

    #[test]
    fn first_eligible_observation_is_not_pool_creation() {
        let observations = vec![
            PoolEligibilityObservation::new(
                100,
                PoolClassificationInput::new(Some("WETH"), Some(0.1), false, false, false),
            ),
            PoolEligibilityObservation::new(
                101,
                PoolClassificationInput::new(Some("WETH"), Some(0.8), true, false, false),
            ),
            PoolEligibilityObservation::new(
                102,
                PoolClassificationInput::new(Some("WETH"), Some(0.8), true, true, false),
            ),
        ];

        let entry = first_eligible_observation(observations).unwrap();

        assert_eq!(entry.block_number, 102);
        assert!(entry.classification.eligible);
        assert_eq!(
            entry.classification.eligible_outcome,
            Some(EligiblePoolOutcome::Active)
        );
    }

    #[test]
    fn first_eligible_observation_uses_current_entry_state_not_lifetime_state() {
        let mut low_current_after_prior_liquidity =
            PoolClassificationInput::new(Some("WETH"), Some(0.03), true, true, false);
        low_current_after_prior_liquidity.max_denom_reserve = Some(1.2);
        low_current_after_prior_liquidity.cohort_can_buy = Some(true);
        low_current_after_prior_liquidity.cohort_can_sell = Some(true);

        let observations = vec![
            PoolEligibilityObservation::new(100, low_current_after_prior_liquidity),
            PoolEligibilityObservation::new(
                101,
                PoolClassificationInput::new(Some("WETH"), Some(0.6), true, true, false),
            ),
        ];

        let entry = first_eligible_observation(observations).unwrap();

        assert_eq!(entry.block_number, 101);
        assert_eq!(entry.input.max_denom_reserve, Some(0.6));
        assert_eq!(entry.input.cohort_can_buy, None);
        assert_eq!(entry.input.cohort_can_sell, None);
    }
}
