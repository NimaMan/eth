use alloy_primitives::Address;
use eth_token::erc20::ERC20Token;
use eth_token::pools::{
    LPHolderSnapshot, PoolLifecycle, PoolRuntimeState, TaxBucket, TradingStatus, UniswapV2Pool,
};
use reth_chain_query::common_addresses::get_token_symbol;
use serde::Serialize;
use serde_json::Value;

use crate::range_indexer::RangeIndexJob;

#[derive(Clone, Debug, Serialize)]
pub struct PoolListResponse {
    pub run_id: String,
    pub count: usize,
    pub pools: Vec<PoolView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PriceRatioPoint {
    pub block_number: u64,
    pub price: f64,
    pub ratio: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiquidityPoint {
    pub block_number: u64,
    pub liquidity: f64,
    pub denom_reserve: f64,
    pub token_reserve: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolRiskLevel {
    Clear,
    Scam,
    Honeypot,
    HighTax,
    ExtremeTax,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolLiquidityLevel {
    Liquid,
    Dust,
    Drained,
    Unknown,
}

#[derive(Clone, Debug)]
struct PoolRiskView {
    level: PoolRiskLevel,
    label: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolView {
    pub token_address: String,
    pub token_symbol: String,
    pub pool_address: String,
    pub protocol: String,
    pub denom_address: String,
    pub denom_symbol: Option<String>,
    pub currency: String,
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub price: f64,
    pub initial_price: Option<f64>,
    pub raw_price_ratio_to_initial: Option<f64>,
    pub price_ratio_to_initial: Option<f64>,
    pub price_ratio_history: Vec<PriceRatioPoint>,
    pub liquidity_history: Vec<LiquidityPoint>,
    pub total_liquidity: f64,
    pub liquidity_level: PoolLiquidityLevel,
    pub liquidity_label: String,
    pub token_total_supply_scaled: Option<f64>,
    pub fully_diluted_value_denom: Option<f64>,
    pub pooled_token_supply_ratio: Option<f64>,
    pub pooled_token_supply_percent: Option<f64>,
    pub liquidity_to_fdv_ratio: Option<f64>,
    pub liquidity_to_fdv_percent: Option<f64>,
    pub supply_ratio_status: String,
    pub supply_ratio_label: Option<String>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub trading_enabled: bool,
    pub stage: PoolLifecycle,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub buy_tax_bucket: TaxBucket,
    pub sell_tax_bucket: TaxBucket,
    pub tax_bucket: TaxBucket,
    pub last_trading_failure_reason: Option<String>,
    pub last_trading_failure_class: Option<String>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub risk_level: PoolRiskLevel,
    pub risk_label: Option<String>,
    pub creation_block: Option<u64>,
    pub creation_timestamp: Option<u64>,
    pub can_buy_block: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub trading_status: TradingStatus,
    pub runtime_state: PoolRuntimeState,
    pub lp_total_supply: f64,
    pub lp_holder_count: usize,
    pub lp_holders: Vec<LPHolderSnapshot>,
    pub lp_total_approved_to_routers: f64,
    pub lp_approved_percentage: f64,
    pub lp_last_approval_block: Option<u64>,
    pub lp_last_approval: Option<Value>,
    pub lp_holders_with_approvals: Vec<String>,
    pub lp_transfer_count: usize,
    pub lp_approval_count: usize,
}

impl PoolView {
    pub fn from_pool(token: &ERC20Token, pool: &UniswapV2Pool) -> Self {
        let trading_status = pool.base.trading_status();
        let lp_holders = pool.lp_holders();
        let lp_holder_count = lp_holders.len();
        let total_supply = token.total_supply_scaled();
        let denom_symbol = denom_symbol(&pool.base.identity.denom_address);
        let currency = denom_symbol
            .clone()
            .unwrap_or_else(|| pool.base.identity.denom_address.clone());
        let fully_diluted_value_denom =
            total_supply.and_then(|supply| pool.base.fully_diluted_value_denom(supply));
        let liquidity_history = liquidity_history(pool);
        let liquidity_level = pool_liquidity_level(pool.base.state.total_liquidity, &currency);
        let raw_price_ratio_to_initial = pool.base.price_ratio_to_initial();
        let price_ratio_to_initial =
            display_price_ratio(raw_price_ratio_to_initial, liquidity_level);
        let price_ratio_history =
            display_price_ratio_history(&pool.base.price_history, liquidity_level);
        let raw_pooled_token_supply_ratio =
            total_supply.and_then(|supply| pool.base.pooled_token_supply_ratio(supply));
        let supply_ratio = display_supply_ratio(raw_pooled_token_supply_ratio);
        let liquidity_to_fdv_ratio = supply_ratio
            .pooled_token_supply_ratio
            .and_then(|_| total_supply.and_then(|supply| pool.base.liquidity_to_fdv_ratio(supply)));
        let buy_tax = display_tax(pool.base.buy_tax);
        let sell_tax = display_tax(pool.base.sell_tax);
        let buy_tax_bucket = TaxBucket::from_percent(buy_tax);
        let sell_tax_bucket = TaxBucket::from_percent(sell_tax);
        let tax_bucket = TaxBucket::combined(buy_tax, sell_tax);
        let risk = pool_risk(pool);
        Self {
            token_address: token.contract_address.clone(),
            token_symbol: token.symbol.clone(),
            pool_address: pool.base.identity.pool_address.clone(),
            protocol: pool.base.identity.protocol.clone(),
            denom_address: pool.base.identity.denom_address.clone(),
            denom_symbol,
            currency,
            token_reserve: pool.base.token_reserve(),
            denom_reserve: pool.base.denom_reserve(),
            price: pool.base.price(),
            initial_price: pool.base.initial_price(),
            raw_price_ratio_to_initial,
            price_ratio_to_initial,
            price_ratio_history,
            liquidity_history,
            total_liquidity: pool.base.state.total_liquidity,
            liquidity_level,
            liquidity_label: liquidity_level_label(liquidity_level).to_string(),
            token_total_supply_scaled: total_supply,
            fully_diluted_value_denom,
            pooled_token_supply_ratio: supply_ratio.pooled_token_supply_ratio,
            pooled_token_supply_percent: ratio_percent(supply_ratio.pooled_token_supply_ratio),
            liquidity_to_fdv_ratio,
            liquidity_to_fdv_percent: ratio_percent(liquidity_to_fdv_ratio),
            supply_ratio_status: supply_ratio.status.to_string(),
            supply_ratio_label: supply_ratio.label,
            can_buy: pool.base.state.can_buy,
            can_sell: pool.base.state.can_sell,
            trading_enabled: pool.base.trading_enabled(),
            stage: pool.base.state.lifecycle,
            buy_tax,
            sell_tax,
            buy_tax_bucket,
            sell_tax_bucket,
            tax_bucket,
            last_trading_failure_reason: pool.base.last_trading_failure_reason.clone(),
            last_trading_failure_class: pool.base.last_trading_failure_class.clone(),
            is_scam: risk.level != PoolRiskLevel::Clear,
            scam_label: risk.label.clone(),
            risk_level: risk.level,
            risk_label: risk.label,
            creation_block: pool.base.creation_block,
            creation_timestamp: pool.base.creation_timestamp,
            can_buy_block: pool.base.can_buy_block,
            latest_block_number: latest_pool_block_number(pool),
            trading_status,
            runtime_state: pool.base.state.clone(),
            lp_total_supply: pool.lp_tracker.total_supply,
            lp_holder_count,
            lp_holders,
            lp_total_approved_to_routers: pool.total_approved_to_routers(),
            lp_approved_percentage: pool.lp_approved_percentage(),
            lp_last_approval_block: pool.last_lp_approval_block(),
            lp_last_approval: pool.last_lp_approval_event(),
            lp_holders_with_approvals: pool.holders_with_approvals(),
            lp_transfer_count: pool.lp_tracker.transfers.len(),
            lp_approval_count: pool.lp_tracker.approval_events.len(),
        }
    }
}

const DUST_WETH_LIQUIDITY: f64 = 0.01;
const DRAINED_WETH_LIQUIDITY: f64 = 0.000001;
const DUST_STABLE_LIQUIDITY: f64 = 10.0;
const DRAINED_STABLE_LIQUIDITY: f64 = 0.01;
const MAX_VALID_SUPPLY_RATIO: f64 = 1.000001;

#[derive(Clone, Debug)]
struct DisplaySupplyRatio {
    pooled_token_supply_ratio: Option<f64>,
    status: &'static str,
    label: Option<String>,
}

fn pool_risk(pool: &UniswapV2Pool) -> PoolRiskView {
    if pool.base.is_scam() {
        return PoolRiskView {
            level: PoolRiskLevel::Scam,
            label: pool
                .base
                .scam_label
                .clone()
                .or_else(|| Some("scam".to_string())),
        };
    }

    if pool.base.state.can_buy && !pool.base.state.can_sell {
        return PoolRiskView {
            level: PoolRiskLevel::Honeypot,
            label: Some("cannot_sell".to_string()),
        };
    }

    match TaxBucket::combined(
        display_tax(pool.base.buy_tax),
        display_tax(pool.base.sell_tax),
    ) {
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

fn display_tax(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

fn display_price_ratio(value: Option<f64>, liquidity_level: PoolLiquidityLevel) -> Option<f64> {
    if liquidity_level != PoolLiquidityLevel::Liquid {
        return None;
    }
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn display_price_ratio_history(
    history: &[(u64, f64)],
    liquidity_level: PoolLiquidityLevel,
) -> Vec<PriceRatioPoint> {
    if liquidity_level != PoolLiquidityLevel::Liquid {
        return Vec::new();
    }
    price_ratio_history(history)
}

fn display_supply_ratio(value: Option<f64>) -> DisplaySupplyRatio {
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

fn pool_liquidity_level(liquidity: f64, currency: &str) -> PoolLiquidityLevel {
    if !liquidity.is_finite() || liquidity <= drained_liquidity_threshold(currency) {
        return PoolLiquidityLevel::Drained;
    }
    if !is_liquidity_currency(currency) {
        return PoolLiquidityLevel::Unknown;
    }
    if liquidity <= dust_liquidity_threshold(currency) {
        return PoolLiquidityLevel::Dust;
    }
    PoolLiquidityLevel::Liquid
}

fn liquidity_level_label(level: PoolLiquidityLevel) -> &'static str {
    match level {
        PoolLiquidityLevel::Liquid => "liquid",
        PoolLiquidityLevel::Dust => "dust",
        PoolLiquidityLevel::Drained => "drained",
        PoolLiquidityLevel::Unknown => "unknown_quote",
    }
}

fn is_liquidity_currency(currency: &str) -> bool {
    let currency = currency.to_ascii_uppercase();
    currency == "WETH" || is_stable_currency(&currency)
}

fn dust_liquidity_threshold(currency: &str) -> f64 {
    if is_stable_currency(currency) {
        DUST_STABLE_LIQUIDITY
    } else {
        DUST_WETH_LIQUIDITY
    }
}

fn drained_liquidity_threshold(currency: &str) -> f64 {
    if is_stable_currency(currency) {
        DRAINED_STABLE_LIQUIDITY
    } else {
        DRAINED_WETH_LIQUIDITY
    }
}

fn is_stable_currency(currency: &str) -> bool {
    matches!(
        currency.to_ascii_uppercase().as_str(),
        "USDC" | "USDT" | "DAI"
    )
}

fn liquidity_history(pool: &UniswapV2Pool) -> Vec<LiquidityPoint> {
    pool.base
        .reserve_tracker
        .reserve_history
        .iter()
        .filter_map(|snapshot| {
            if !snapshot.denom_reserve.is_finite() || !snapshot.token_reserve.is_finite() {
                return None;
            }
            let liquidity = if snapshot.denom_reserve >= pool.base.config.denom_threshold {
                snapshot.denom_reserve
            } else {
                0.0
            };
            Some(LiquidityPoint {
                block_number: snapshot.block_number,
                liquidity,
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

fn ratio_percent(value: Option<f64>) -> Option<f64> {
    value
        .map(|value| value * 100.0)
        .filter(|value| value.is_finite())
}

fn denom_symbol(address: &str) -> Option<String> {
    address
        .parse::<Address>()
        .ok()
        .and_then(get_token_symbol)
        .map(str::to_string)
}

fn latest_pool_block_number(pool: &UniswapV2Pool) -> Option<u64> {
    pool.base
        .latest_block_number
        .or_else(|| nonzero_block(pool.base.state.last_update_block))
        .or_else(|| nonzero_block(pool.base.state.last_sync_block))
        .or(pool.base.can_buy_block)
        .or(pool.base.creation_block)
}

fn nonzero_block(block: u64) -> Option<u64> {
    (block > 0).then_some(block)
}

pub async fn pool_list(run: &RangeIndexJob) -> PoolListResponse {
    let state = run.state.read().await;
    let mut pools = Vec::new();

    for token in state.processor.registry.tokens.values() {
        for pool in token.v2_pools.values() {
            pools.push(PoolView::from_pool(token, pool));
        }
    }

    pools.sort_by(|left, right| {
        left.token_address
            .cmp(&right.token_address)
            .then(left.pool_address.cmp(&right.pool_address))
    });

    PoolListResponse {
        run_id: run.id.clone(),
        count: pools.len(),
        pools,
    }
}

#[cfg(test)]
mod tests {
    use eth_token::pools::BasePoolConfig;

    use super::*;

    #[test]
    fn display_tax_hides_failed_simulation_sentinel() {
        assert_eq!(display_tax(Some(-1.0)), None);
        assert_eq!(display_tax(Some(2.99)), Some(2.99));
    }

    #[test]
    fn display_price_ratio_requires_liquid_pool() {
        assert_eq!(
            display_price_ratio(Some(1000.0), PoolLiquidityLevel::Dust),
            None
        );
        assert_eq!(
            display_price_ratio(Some(1000.0), PoolLiquidityLevel::Unknown),
            None
        );
        assert_eq!(
            display_price_ratio(Some(10.0), PoolLiquidityLevel::Liquid),
            Some(10.0)
        );
    }

    #[test]
    fn liquidity_level_marks_unknown_quote_assets() {
        assert_eq!(
            pool_liquidity_level(1_000.0, "WETH"),
            PoolLiquidityLevel::Liquid
        );
        assert_eq!(
            pool_liquidity_level(1_000_000.0, "0xunknown"),
            PoolLiquidityLevel::Unknown
        );
    }

    #[test]
    fn display_supply_ratio_rejects_reserve_above_total_supply() {
        let ratio = display_supply_ratio(Some(10_000.0));
        assert_eq!(ratio.pooled_token_supply_ratio, None);
        assert_eq!(ratio.status, "inconsistent");
        assert_eq!(
            ratio.label,
            Some("pool_reserve_exceeds_total_supply".to_string())
        );
    }

    #[test]
    fn latest_pool_block_uses_runtime_update_when_control_block_is_empty() {
        let mut pool = UniswapV2Pool::new(
            "0xpool",
            "0xtoken",
            "0xdenom",
            BasePoolConfig::new(18),
            std::iter::empty::<&str>(),
        );
        pool.base.state.last_update_block = 123;

        assert_eq!(latest_pool_block_number(&pool), Some(123));

        pool.base.latest_block_number = Some(456);
        assert_eq!(latest_pool_block_number(&pool), Some(456));
    }
}
