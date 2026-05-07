use alloy_primitives::Address;
use eth_token::erc20::ERC20Token;
use eth_token::pools::{LPHolderSnapshot, PoolRuntimeState, TradingStatus, UniswapV2Pool};
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
    pub price_ratio_to_initial: Option<f64>,
    pub price_ratio_history: Vec<PriceRatioPoint>,
    pub liquidity_history: Vec<LiquidityPoint>,
    pub total_liquidity: f64,
    pub token_total_supply_scaled: Option<f64>,
    pub fully_diluted_value_denom: Option<f64>,
    pub pooled_token_supply_ratio: Option<f64>,
    pub pooled_token_supply_percent: Option<f64>,
    pub liquidity_to_fdv_ratio: Option<f64>,
    pub liquidity_to_fdv_percent: Option<f64>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub trading_enabled: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub creation_block: Option<u64>,
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
        let pooled_token_supply_ratio =
            total_supply.and_then(|supply| pool.base.pooled_token_supply_ratio(supply));
        let liquidity_to_fdv_ratio =
            total_supply.and_then(|supply| pool.base.liquidity_to_fdv_ratio(supply));
        let price_ratio_history = price_ratio_history(&pool.base.price_history);
        let liquidity_history = liquidity_history(pool);
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
            price_ratio_to_initial: pool.base.price_ratio_to_initial(),
            price_ratio_history,
            liquidity_history,
            total_liquidity: pool.base.state.total_liquidity,
            token_total_supply_scaled: total_supply,
            fully_diluted_value_denom,
            pooled_token_supply_ratio,
            pooled_token_supply_percent: ratio_percent(pooled_token_supply_ratio),
            liquidity_to_fdv_ratio,
            liquidity_to_fdv_percent: ratio_percent(liquidity_to_fdv_ratio),
            can_buy: pool.base.state.can_buy,
            can_sell: pool.base.state.can_sell,
            trading_enabled: pool.base.trading_enabled(),
            buy_tax: pool.base.buy_tax,
            sell_tax: pool.base.sell_tax,
            is_scam: pool.base.is_scam(),
            scam_label: pool.base.scam_label.clone(),
            creation_block: pool.base.creation_block,
            can_buy_block: pool.base.can_buy_block,
            latest_block_number: pool.base.latest_block_number,
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
