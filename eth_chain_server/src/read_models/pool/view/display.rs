use eth_price::liquidity::PoolLiquidityLevel;
use eth_token::pools::{BasePool, PoolLifecycle, TaxBucket};

use super::{CurrentTradingView, LiquidityPoint, PoolRiskLevel, PoolRiskView, PriceRatioPoint};

const MAX_VALID_SUPPLY_RATIO: f64 = 1.000001;
const MIN_TRUSTWORTHY_POOLED_TOKEN_SUPPLY_RATIO: f64 = 1e-6;

pub(super) fn max_denom_reserve(history: &[LiquidityPoint], current: f64) -> f64 {
    history
        .iter()
        .map(|point| point.denom_reserve)
        .chain(std::iter::once(current))
        .filter(|value| value.is_finite() && *value >= 0.0)
        .fold(0.0, f64::max)
}

pub(super) fn tax_bucket_key(bucket: TaxBucket) -> &'static str {
    match bucket {
        TaxBucket::Unknown => "unknown",
        TaxBucket::NoTax => "no_tax",
        TaxBucket::LowTax => "low_tax",
        TaxBucket::ModerateTax => "moderate_tax",
        TaxBucket::HighTax => "high_tax",
        TaxBucket::ExtremeTax => "extreme_tax",
    }
}

#[derive(Clone, Debug)]
pub(super) struct DisplaySupplyRatio {
    pub(super) pooled_token_supply_ratio: Option<f64>,
    pub(super) status: &'static str,
    pub(super) label: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct DisplayReserveQuality {
    pub(super) status: &'static str,
    pub(super) label: Option<String>,
    pub(super) price_ratio_trustworthy: bool,
}

pub(super) fn current_trading_view(
    base: &BasePool,
    liquidity_level: PoolLiquidityLevel,
) -> CurrentTradingView {
    let liquidity_allows_trading = matches!(
        liquidity_level,
        PoolLiquidityLevel::Liquid | PoolLiquidityLevel::Unknown
    ) && !matches!(
        base.state.lifecycle,
        PoolLifecycle::Dust
            | PoolLifecycle::Drained
            | PoolLifecycle::LiquidityRemoved
            | PoolLifecycle::Evicted
    );
    CurrentTradingView {
        can_buy: liquidity_allows_trading && base.state.can_buy,
        can_sell: liquidity_allows_trading && base.state.can_sell,
    }
}

pub(super) fn current_lifecycle_view(
    base: &BasePool,
    current_trading: CurrentTradingView,
) -> PoolLifecycle {
    match base.state.lifecycle {
        PoolLifecycle::Dust
        | PoolLifecycle::Drained
        | PoolLifecycle::LiquidityRemoved
        | PoolLifecycle::Evicted => base.state.lifecycle,
        _ if current_trading.can_buy && current_trading.can_sell => PoolLifecycle::Trading,
        _ if current_trading.can_buy => PoolLifecycle::CannotSell,
        _ => base.state.lifecycle,
    }
}

pub(super) fn pool_risk(base: &BasePool, current_trading: CurrentTradingView) -> PoolRiskView {
    if base.has_liquidity_removal() {
        return PoolRiskView {
            level: PoolRiskLevel::LiquidityRemoval,
            label: base
                .scam_label
                .clone()
                .or_else(|| Some("liquidity_removal".to_string())),
        };
    }

    if current_trading.can_buy && !current_trading.can_sell {
        return PoolRiskView {
            level: PoolRiskLevel::Honeypot,
            label: Some("cannot_sell".to_string()),
        };
    }

    match TaxBucket::combined(display_tax(base.buy_tax), display_tax(base.sell_tax)) {
        TaxBucket::ExtremeTax => {
            return PoolRiskView {
                level: PoolRiskLevel::ExtremeTax,
                label: TaxBucket::ExtremeTax.risk_label().map(str::to_string),
            };
        }
        TaxBucket::HighTax => {
            return PoolRiskView {
                level: PoolRiskLevel::HighTax,
                label: TaxBucket::HighTax.risk_label().map(str::to_string),
            };
        }
        _ => {}
    }

    PoolRiskView {
        level: PoolRiskLevel::Clear,
        label: None,
    }
}

pub(super) fn display_tax(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

pub(super) fn economic_sellable_from_tax(can_sell: bool, sell_tax: Option<f64>) -> Option<bool> {
    if !can_sell {
        return Some(false);
    }
    display_tax(sell_tax).map(|tax| tax <= 40.0)
}

pub(super) fn display_price_ratio(
    value: Option<f64>,
    liquidity_level: PoolLiquidityLevel,
    reserve_quality: &DisplayReserveQuality,
) -> Option<f64> {
    if matches!(
        liquidity_level,
        PoolLiquidityLevel::Dust | PoolLiquidityLevel::Drained
    ) || !reserve_quality.price_ratio_trustworthy
    {
        return Some(0.0);
    }

    value.filter(|value| value.is_finite() && *value > 0.0)
}

pub(super) fn display_price_ratio_history(
    history: &[(u64, f64)],
    liquidity_level: PoolLiquidityLevel,
    reserve_quality: &DisplayReserveQuality,
) -> Vec<PriceRatioPoint> {
    if matches!(
        liquidity_level,
        PoolLiquidityLevel::Dust | PoolLiquidityLevel::Drained
    ) || !reserve_quality.price_ratio_trustworthy
    {
        return Vec::new();
    }
    price_ratio_history(history)
}

pub(super) fn display_supply_ratio(value: Option<f64>) -> DisplaySupplyRatio {
    let Some(value) = value.filter(|value| value.is_finite() && *value >= 0.0) else {
        return DisplaySupplyRatio {
            pooled_token_supply_ratio: None,
            status: "unknown",
            label: None,
        };
    };

    if value > MAX_VALID_SUPPLY_RATIO {
        return DisplaySupplyRatio {
            pooled_token_supply_ratio: None,
            status: "inconsistent",
            label: Some("pool_reserve_exceeds_total_supply".to_string()),
        };
    }

    DisplaySupplyRatio {
        pooled_token_supply_ratio: Some(value),
        status: "ok",
        label: None,
    }
}

pub(super) fn display_reserve_quality(
    supply_ratio: &DisplaySupplyRatio,
    liquidity_level: PoolLiquidityLevel,
) -> DisplayReserveQuality {
    if matches!(
        liquidity_level,
        PoolLiquidityLevel::Dust | PoolLiquidityLevel::Drained
    ) {
        return DisplayReserveQuality {
            status: "denom_liquidity_unsafe",
            label: Some("denom liquidity is dust or drained".to_string()),
            price_ratio_trustworthy: false,
        };
    }

    if supply_ratio.status == "inconsistent" {
        return DisplayReserveQuality {
            status: "inconsistent_supply",
            label: supply_ratio.label.clone(),
            price_ratio_trustworthy: false,
        };
    }

    if let Some(ratio) = supply_ratio.pooled_token_supply_ratio {
        if ratio < MIN_TRUSTWORTHY_POOLED_TOKEN_SUPPLY_RATIO {
            return DisplayReserveQuality {
                status: "token_reserve_dust",
                label: Some("pooled token reserve is dust relative to supply".to_string()),
                price_ratio_trustworthy: false,
            };
        }
    }

    DisplayReserveQuality {
        status: "ok",
        label: None,
        price_ratio_trustworthy: true,
    }
}

pub(super) fn liquidity_history(base: &BasePool) -> Vec<LiquidityPoint> {
    base.reserve_tracker
        .reserve_history
        .iter()
        .filter_map(|snapshot| {
            if !snapshot.denom_reserve.is_finite() || !snapshot.token_reserve.is_finite() {
                return None;
            }
            Some(LiquidityPoint {
                block_number: snapshot.block_number,
                liquidity: snapshot.denom_reserve.max(0.0),
                denom_reserve: snapshot.denom_reserve,
                token_reserve: snapshot.token_reserve,
            })
        })
        .collect()
}

fn price_ratio_history(history: &[(u64, f64)]) -> Vec<PriceRatioPoint> {
    let Some((_, initial_price)) = history
        .iter()
        .find(|(_, price)| price.is_finite() && *price > 0.0)
    else {
        return Vec::new();
    };

    history
        .iter()
        .filter_map(|(block_number, price)| {
            if !price.is_finite() || *price <= 0.0 {
                return None;
            }
            let ratio = price / initial_price;
            if !ratio.is_finite() {
                return None;
            }
            Some(PriceRatioPoint {
                block_number: *block_number,
                price: *price,
                ratio,
            })
        })
        .collect()
}

pub(super) fn truncate_history_at_liquidity_removal(
    liquidity_history: &[LiquidityPoint],
    price_ratio_history: &[PriceRatioPoint],
    liquidity_removal_block: Option<u64>,
) -> (Vec<LiquidityPoint>, Vec<PriceRatioPoint>) {
    let Some(removal_block) = liquidity_removal_block else {
        return (liquidity_history.to_vec(), price_ratio_history.to_vec());
    };

    let max_before = liquidity_history
        .iter()
        .filter(|p| p.block_number <= removal_block)
        .map(|p| p.liquidity)
        .filter(|l| l.is_finite() && *l > 0.0)
        .fold(0.0, f64::max);

    let after = liquidity_history
        .iter()
        .find(|p| p.block_number > removal_block)
        .map(|p| p.liquidity)
        .unwrap_or(0.0);

    let significant_removal = max_before > 0.0 && after / max_before < 0.1;
    if !significant_removal {
        return (liquidity_history.to_vec(), price_ratio_history.to_vec());
    }

    let truncated_liquidity: Vec<_> = liquidity_history
        .iter()
        .take_while(|p| p.block_number <= removal_block)
        .cloned()
        .collect();

    let truncated_price_ratio: Vec<_> = price_ratio_history
        .iter()
        .take_while(|p| p.block_number <= removal_block)
        .cloned()
        .collect();

    (truncated_liquidity, truncated_price_ratio)
}

pub(super) fn ratio_percent(value: Option<f64>) -> Option<f64> {
    value
        .map(|value| value * 100.0)
        .filter(|value| value.is_finite())
}
