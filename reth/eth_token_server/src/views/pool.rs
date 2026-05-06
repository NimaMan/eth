use eth_token::pools::{PoolRuntimeState, TradingStatus, UniswapV2Pool};
use serde::Serialize;

use crate::runs::TrackingRun;

#[derive(Clone, Debug, Serialize)]
pub struct PoolListResponse {
    pub run_id: String,
    pub count: usize,
    pub pools: Vec<PoolView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolView {
    pub token_address: String,
    pub token_symbol: String,
    pub pool_address: String,
    pub protocol: String,
    pub denom_address: String,
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub price: f64,
    pub total_liquidity: f64,
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
}

impl PoolView {
    pub fn from_pool(token_address: &str, token_symbol: &str, pool: &UniswapV2Pool) -> Self {
        let trading_status = pool.base.trading_status();
        Self {
            token_address: token_address.to_string(),
            token_symbol: token_symbol.to_string(),
            pool_address: pool.base.identity.pool_address.clone(),
            protocol: pool.base.identity.protocol.clone(),
            denom_address: pool.base.identity.denom_address.clone(),
            token_reserve: pool.base.token_reserve(),
            denom_reserve: pool.base.denom_reserve(),
            price: pool.base.price(),
            total_liquidity: pool.base.state.total_liquidity,
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
        }
    }
}

pub async fn pool_list(run: &TrackingRun) -> PoolListResponse {
    let state = run.state.read().await;
    let mut pools = Vec::new();

    for token in state.processor.registry.tokens.values() {
        for pool in token.v2_pools.values() {
            pools.push(PoolView::from_pool(
                &token.contract_address,
                &token.symbol,
                pool,
            ));
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
