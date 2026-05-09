use std::collections::BTreeMap;

use eth_token::pools::TaxBucket;
use serde::Serialize;

use crate::range_indexer::progress::now_unix_secs;
use crate::range_indexer::RangeIndexJob;
use crate::views::pool::{PoolRiskLevel, PoolView};

const MIN_ELIGIBLE_WETH_LIQUIDITY: f64 = 0.5;
const MIN_ELIGIBLE_STABLE_LIQUIDITY: f64 = 500.0;
const SUPPORTED_CURRENCIES: [&str; 4] = ["ETH", "WETH", "USDC", "USDT"];
const ETH_BLOCK_SECONDS: u64 = 12;
const WINNER_THRESHOLDS: [f64; 6] = [2.0, 5.0, 10.0, 20.0, 50.0, 100.0];
const WINDOWS: [WindowSpec; 8] = [
    WindowSpec {
        label: "15m",
        seconds: 15 * 60,
    },
    WindowSpec {
        label: "1h",
        seconds: 60 * 60,
    },
    WindowSpec {
        label: "6h",
        seconds: 6 * 60 * 60,
    },
    WindowSpec {
        label: "24h",
        seconds: 24 * 60 * 60,
    },
    WindowSpec {
        label: "36h",
        seconds: 36 * 60 * 60,
    },
    WindowSpec {
        label: "48h",
        seconds: 48 * 60 * 60,
    },
    WindowSpec {
        label: "5d",
        seconds: 5 * 24 * 60 * 60,
    },
    WindowSpec {
        label: "7d",
        seconds: 7 * 24 * 60 * 60,
    },
];

#[derive(Clone, Debug, Serialize)]
pub struct LaunchStatsResponse {
    pub run_id: String,
    pub scope: &'static str,
    pub generated_at_unix_secs: u64,
    pub block_time_seconds: u64,
    pub filter: LaunchStatsFilter,
    pub totals: LaunchStatsTotals,
    pub non_eligible_reasons: Vec<ReasonCount>,
    pub winner_matrix: Vec<WinnerThresholdRow>,
    pub risk_outcomes: Vec<ReasonCount>,
    pub breakdowns: LaunchStatsBreakdowns,
}

#[derive(Clone, Debug, Serialize)]
pub struct LaunchStatsFilter {
    pub unit: &'static str,
    pub launch_definition: &'static str,
    pub min_liquidity_eth: f64,
    pub min_liquidity_usd: f64,
    pub supported_currencies: Vec<&'static str>,
    pub eligibility_basis: &'static str,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LaunchStatsTotals {
    pub launched_pools: usize,
    pub eligible_pools: usize,
    pub non_eligible_pools: usize,
    pub eligible_share_percent: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReasonCount {
    pub reason: String,
    pub label: String,
    pub count: usize,
    pub share_percent: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WinnerThresholdRow {
    pub threshold: f64,
    pub label: String,
    pub windows: Vec<WinnerWindowCell>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WinnerWindowCell {
    pub window: &'static str,
    pub seconds: u64,
    pub count: usize,
    pub eligible_percent: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LaunchStatsBreakdowns {
    pub protocol: Vec<CohortCount>,
    pub currency: Vec<CohortCount>,
    pub launch_hour_utc: Vec<CohortCount>,
    pub initial_liquidity: Vec<CohortCount>,
    pub tax_bucket: Vec<CohortCount>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CohortCount {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub eligible_percent: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum NonEligibleReason {
    UnsupportedCurrency,
    MissingCreationData,
    LowLiquidity,
    CannotBuy,
    CannotSell,
    MissingPriceData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum RiskOutcome {
    LiquidityRemoval,
    HoneypotFlag,
    HighTax,
    ExtremeTax,
    LpApprovalExposure,
}

#[derive(Clone, Copy, Debug)]
struct WindowSpec {
    label: &'static str,
    seconds: u64,
}

#[derive(Clone, Debug)]
struct PoolStatsRecord {
    protocol: String,
    currency: String,
    liquidity: f64,
    can_buy: bool,
    can_sell: bool,
    creation_block: Option<u64>,
    creation_timestamp: Option<u64>,
    price_ratio_history: Vec<(u64, f64)>,
    risk_level: PoolRiskLevel,
    tax_bucket: TaxBucket,
    lp_approved_percentage: f64,
}

impl From<&PoolView> for PoolStatsRecord {
    fn from(pool: &PoolView) -> Self {
        Self {
            protocol: pool.protocol.clone(),
            currency: pool.currency.clone(),
            liquidity: normalized_liquidity(pool),
            can_buy: pool.can_buy,
            can_sell: pool.can_sell,
            creation_block: pool.creation_block,
            creation_timestamp: pool.creation_timestamp,
            price_ratio_history: pool
                .price_ratio_history
                .iter()
                .map(|point| (point.block_number, point.ratio))
                .collect(),
            risk_level: pool.risk_level,
            tax_bucket: pool.tax_bucket,
            lp_approved_percentage: pool.lp_approved_percentage,
        }
    }
}

pub async fn launch_stats(run: &RangeIndexJob) -> LaunchStatsResponse {
    let state = run.state.read().await;
    let mut records = Vec::new();

    for token in state.processor.registry.tokens.values() {
        for pool in PoolView::from_token_pools(token) {
            records.push(PoolStatsRecord::from(&pool));
        }
    }

    build_launch_stats(&run.id, &records)
}

fn build_launch_stats(run_id: &str, records: &[PoolStatsRecord]) -> LaunchStatsResponse {
    let mut non_eligible_counts = BTreeMap::new();
    let mut eligible_records = Vec::new();

    for record in records {
        match eligibility_reason(record) {
            Some(reason) => {
                *non_eligible_counts.entry(reason).or_insert(0usize) += 1;
            }
            None => eligible_records.push(record),
        }
    }

    let launched_pools = records.len();
    let eligible_pools = eligible_records.len();
    let non_eligible_pools = launched_pools.saturating_sub(eligible_pools);

    LaunchStatsResponse {
        run_id: run_id.to_string(),
        scope: "historical_range",
        generated_at_unix_secs: now_unix_secs(),
        block_time_seconds: ETH_BLOCK_SECONDS,
        filter: LaunchStatsFilter {
            unit: "pool",
            launch_definition: "pool_creation",
            min_liquidity_eth: MIN_ELIGIBLE_WETH_LIQUIDITY,
            min_liquidity_usd: MIN_ELIGIBLE_STABLE_LIQUIDITY,
            supported_currencies: SUPPORTED_CURRENCIES.to_vec(),
            eligibility_basis: "current_snapshot",
        },
        totals: LaunchStatsTotals {
            launched_pools,
            eligible_pools,
            non_eligible_pools,
            eligible_share_percent: percent(eligible_pools, launched_pools),
        },
        non_eligible_reasons: reason_counts(non_eligible_counts, launched_pools),
        winner_matrix: winner_matrix(&eligible_records),
        risk_outcomes: risk_outcomes(&eligible_records),
        breakdowns: breakdowns(&eligible_records),
    }
}

fn eligibility_reason(record: &PoolStatsRecord) -> Option<NonEligibleReason> {
    if !is_supported_currency(&record.currency) {
        return Some(NonEligibleReason::UnsupportedCurrency);
    }
    if record.creation_block.is_none() || record.creation_timestamp.is_none() {
        return Some(NonEligibleReason::MissingCreationData);
    }
    if !record.liquidity.is_finite() || record.liquidity < min_eligible_liquidity(&record.currency)
    {
        return Some(NonEligibleReason::LowLiquidity);
    }
    if !record.can_buy {
        return Some(NonEligibleReason::CannotBuy);
    }
    if !record.can_sell {
        return Some(NonEligibleReason::CannotSell);
    }
    if record.price_ratio_history.is_empty() {
        return Some(NonEligibleReason::MissingPriceData);
    }
    None
}

fn is_supported_currency(currency: &str) -> bool {
    SUPPORTED_CURRENCIES
        .iter()
        .any(|supported| supported.eq_ignore_ascii_case(currency))
}

fn min_eligible_liquidity(currency: &str) -> f64 {
    match currency.to_ascii_uppercase().as_str() {
        "USDC" | "USDT" => MIN_ELIGIBLE_STABLE_LIQUIDITY,
        _ => MIN_ELIGIBLE_WETH_LIQUIDITY,
    }
}

fn normalized_liquidity(pool: &PoolView) -> f64 {
    if pool.total_liquidity.is_finite() && pool.total_liquidity > 0.0 {
        pool.total_liquidity
    } else {
        pool.denom_reserve.max(0.0)
    }
}

fn reason_counts(
    counts: BTreeMap<NonEligibleReason, usize>,
    launched_pools: usize,
) -> Vec<ReasonCount> {
    counts
        .into_iter()
        .map(|(reason, count)| ReasonCount {
            reason: reason.key().to_string(),
            label: reason.label().to_string(),
            count,
            share_percent: percent(count, launched_pools),
        })
        .collect()
}

fn winner_matrix(eligible_records: &[&PoolStatsRecord]) -> Vec<WinnerThresholdRow> {
    WINNER_THRESHOLDS
        .iter()
        .map(|threshold| WinnerThresholdRow {
            threshold: *threshold,
            label: format!("{}x", format_threshold(*threshold)),
            windows: WINDOWS
                .iter()
                .map(|window| {
                    let count = eligible_records
                        .iter()
                        .filter(|record| crossed_threshold(record, *threshold, window.seconds))
                        .count();
                    WinnerWindowCell {
                        window: window.label,
                        seconds: window.seconds,
                        count,
                        eligible_percent: percent(count, eligible_records.len()),
                    }
                })
                .collect(),
        })
        .collect()
}

fn crossed_threshold(record: &PoolStatsRecord, threshold: f64, window_seconds: u64) -> bool {
    let Some(creation_block) = record.creation_block else {
        return false;
    };
    let window_blocks = window_seconds / ETH_BLOCK_SECONDS;
    let end_block = creation_block.saturating_add(window_blocks);

    record
        .price_ratio_history
        .iter()
        .any(|(block_number, ratio)| {
            *block_number <= end_block && ratio.is_finite() && *ratio >= threshold
        })
}

fn risk_outcomes(eligible_records: &[&PoolStatsRecord]) -> Vec<ReasonCount> {
    let mut counts = BTreeMap::new();
    for record in eligible_records {
        if matches!(record.risk_level, PoolRiskLevel::LiquidityRemoval) {
            *counts
                .entry(RiskOutcome::LiquidityRemoval)
                .or_insert(0usize) += 1;
        }
        if matches!(record.risk_level, PoolRiskLevel::Honeypot) {
            *counts.entry(RiskOutcome::HoneypotFlag).or_insert(0usize) += 1;
        }
        if matches!(record.tax_bucket, TaxBucket::HighTax) {
            *counts.entry(RiskOutcome::HighTax).or_insert(0usize) += 1;
        }
        if matches!(record.tax_bucket, TaxBucket::ExtremeTax) {
            *counts.entry(RiskOutcome::ExtremeTax).or_insert(0usize) += 1;
        }
        if record.lp_approved_percentage.is_finite() && record.lp_approved_percentage > 0.0 {
            *counts
                .entry(RiskOutcome::LpApprovalExposure)
                .or_insert(0usize) += 1;
        }
    }

    counts
        .into_iter()
        .map(|(reason, count)| ReasonCount {
            reason: reason.key().to_string(),
            label: reason.label().to_string(),
            count,
            share_percent: percent(count, eligible_records.len()),
        })
        .collect()
}

fn breakdowns(eligible_records: &[&PoolStatsRecord]) -> LaunchStatsBreakdowns {
    let denominator = eligible_records.len();
    let mut protocol = BTreeMap::new();
    let mut currency = BTreeMap::new();
    let mut launch_hour = BTreeMap::new();
    let mut initial_liquidity = BTreeMap::new();
    let mut tax_bucket = BTreeMap::new();

    for record in eligible_records {
        add_count(&mut protocol, &record.protocol, &record.protocol);
        add_count(&mut currency, &record.currency, &record.currency);

        if let Some(timestamp) = record.creation_timestamp {
            let hour = (timestamp / 3600) % 24;
            let key = format!("{hour:02}");
            let label = format!("{hour:02}:00 UTC");
            add_count(&mut launch_hour, &key, &label);
        }

        let liquidity_bucket = liquidity_bucket(record.liquidity);
        add_count(
            &mut initial_liquidity,
            liquidity_bucket.key,
            liquidity_bucket.label,
        );

        let tax_bucket_label = tax_bucket_label(record.tax_bucket);
        add_count(
            &mut tax_bucket,
            tax_bucket_key(record.tax_bucket),
            tax_bucket_label,
        );
    }

    LaunchStatsBreakdowns {
        protocol: cohort_counts(protocol, denominator),
        currency: cohort_counts(currency, denominator),
        launch_hour_utc: cohort_counts(launch_hour, denominator),
        initial_liquidity: cohort_counts(initial_liquidity, denominator),
        tax_bucket: cohort_counts(tax_bucket, denominator),
    }
}

fn add_count(counts: &mut BTreeMap<String, (String, usize)>, key: &str, label: &str) {
    let entry = counts
        .entry(key.to_string())
        .or_insert_with(|| (label.to_string(), 0));
    entry.1 += 1;
}

fn cohort_counts(
    counts: BTreeMap<String, (String, usize)>,
    denominator: usize,
) -> Vec<CohortCount> {
    counts
        .into_iter()
        .map(|(key, (label, count))| CohortCount {
            key,
            label,
            count,
            eligible_percent: percent(count, denominator),
        })
        .collect()
}

struct LiquidityBucket {
    key: &'static str,
    label: &'static str,
}

fn liquidity_bucket(liquidity: f64) -> LiquidityBucket {
    if liquidity < 1.0 {
        LiquidityBucket {
            key: "0_5_to_1",
            label: "0.5-1 ETH",
        }
    } else if liquidity < 2.0 {
        LiquidityBucket {
            key: "1_to_2",
            label: "1-2 ETH",
        }
    } else if liquidity < 5.0 {
        LiquidityBucket {
            key: "2_to_5",
            label: "2-5 ETH",
        }
    } else if liquidity < 10.0 {
        LiquidityBucket {
            key: "5_to_10",
            label: "5-10 ETH",
        }
    } else {
        LiquidityBucket {
            key: "10_plus",
            label: "10+ ETH",
        }
    }
}

fn percent(part: usize, whole: usize) -> Option<f64> {
    if whole == 0 {
        return None;
    }
    Some((part as f64 / whole as f64) * 100.0)
}

fn format_threshold(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as u64)
    } else {
        format!("{value:.2}")
    }
}

fn tax_bucket_key(bucket: TaxBucket) -> &'static str {
    match bucket {
        TaxBucket::Unknown => "unknown",
        TaxBucket::NoTax => "no_tax",
        TaxBucket::LowTax => "low_tax",
        TaxBucket::ModerateTax => "moderate_tax",
        TaxBucket::HighTax => "high_tax",
        TaxBucket::ExtremeTax => "extreme_tax",
    }
}

fn tax_bucket_label(bucket: TaxBucket) -> &'static str {
    match bucket {
        TaxBucket::Unknown => "Unknown",
        TaxBucket::NoTax => "No tax",
        TaxBucket::LowTax => "Low tax",
        TaxBucket::ModerateTax => "Moderate tax",
        TaxBucket::HighTax => "High tax",
        TaxBucket::ExtremeTax => "Extreme tax",
    }
}

impl NonEligibleReason {
    fn key(self) -> &'static str {
        match self {
            Self::UnsupportedCurrency => "unsupported_currency",
            Self::MissingCreationData => "missing_creation_data",
            Self::LowLiquidity => "low_liquidity",
            Self::CannotBuy => "cannot_buy",
            Self::CannotSell => "cannot_sell",
            Self::MissingPriceData => "missing_price_data",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::UnsupportedCurrency => "Unsupported currency",
            Self::MissingCreationData => "Missing creation data",
            Self::LowLiquidity => "Low liquidity",
            Self::CannotBuy => "Cannot buy",
            Self::CannotSell => "Cannot sell",
            Self::MissingPriceData => "Missing price data",
        }
    }
}

impl RiskOutcome {
    fn key(self) -> &'static str {
        match self {
            Self::LiquidityRemoval => "liquidity_removal",
            Self::HoneypotFlag => "honeypot_flag",
            Self::HighTax => "high_tax",
            Self::ExtremeTax => "extreme_tax",
            Self::LpApprovalExposure => "lp_approval_exposure",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::LiquidityRemoval => "Liquidity removal",
            Self::HoneypotFlag => "Honeypot flag",
            Self::HighTax => "High tax",
            Self::ExtremeTax => "Extreme tax",
            Self::LpApprovalExposure => "LP approval exposure",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> PoolStatsRecord {
        PoolStatsRecord {
            protocol: "UNISWAP-V2".to_string(),
            currency: "WETH".to_string(),
            liquidity: 1.0,
            can_buy: true,
            can_sell: true,
            creation_block: Some(100),
            creation_timestamp: Some(1_700_000_000),
            price_ratio_history: vec![(100, 1.0), (150, 2.1), (500, 10.0)],
            risk_level: PoolRiskLevel::Clear,
            tax_bucket: TaxBucket::NoTax,
            lp_approved_percentage: 0.0,
        }
    }

    #[test]
    fn eligibility_accepts_usd_stables_with_stable_liquidity_floor() {
        let mut record = record();
        record.currency = "USDC".to_string();
        record.liquidity = MIN_ELIGIBLE_STABLE_LIQUIDITY;

        assert_eq!(eligibility_reason(&record), None);
    }

    #[test]
    fn eligibility_rejects_usd_stables_below_stable_liquidity_floor() {
        let mut record = record();
        record.currency = "USDT".to_string();
        record.liquidity = MIN_ELIGIBLE_STABLE_LIQUIDITY - 0.01;

        assert_eq!(
            eligibility_reason(&record),
            Some(NonEligibleReason::LowLiquidity)
        );
    }

    #[test]
    fn eligibility_requires_supported_currency() {
        let mut record = record();
        record.currency = "DAI".to_string();

        assert_eq!(
            eligibility_reason(&record),
            Some(NonEligibleReason::UnsupportedCurrency)
        );
    }

    #[test]
    fn eligibility_rejects_low_liquidity() {
        let mut record = record();
        record.liquidity = 0.49;

        assert_eq!(
            eligibility_reason(&record),
            Some(NonEligibleReason::LowLiquidity)
        );
    }

    #[test]
    fn eligibility_rejects_unsellable_pools() {
        let mut record = record();
        record.can_sell = false;

        assert_eq!(
            eligibility_reason(&record),
            Some(NonEligibleReason::CannotSell)
        );
    }

    #[test]
    fn winner_windows_use_block_time_estimate_from_pool_creation() {
        let record = record();

        assert!(crossed_threshold(&record, 2.0, 15 * 60));
        assert!(!crossed_threshold(&record, 10.0, 15 * 60));
        assert!(crossed_threshold(&record, 10.0, 6 * 60 * 60));
    }
}
