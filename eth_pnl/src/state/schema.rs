use eth_token::{
    erc20::ERC20Token,
    pools::{BasePool, PoolLifecycle},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenLatestState {
    pub scope_id: String,
    pub chain_id: i64,
    pub token_address: String,
    pub source_run_id: Option<String>,
    pub as_of_block: Option<u64>,
    pub source_reason: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply_raw: String,
    pub creation_block: Option<u64>,
    pub creation_timestamp: Option<u64>,
    pub creation_tx: Option<String>,
    pub creator_address: Option<String>,
    pub latest_activity_block: Option<u64>,
    pub latest_activity_timestamp: Option<u64>,
    pub latest_pool_activity_block: Option<u64>,
    pub lifecycle_status: Option<String>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_mechanism: Option<String>,
    pub scam_mechanism_label: Option<String>,
    pub pool_count: u64,
    pub active_pool_count: u64,
    pub retained_pool_count: u64,
    pub liquidity_removal_pool_count: u64,
}

impl TokenLatestState {
    pub fn from_token(
        scope_id: impl Into<String>,
        source_run_id: Option<String>,
        as_of_block: Option<u64>,
        source_reason: impl Into<String>,
        token: &ERC20Token,
    ) -> Self {
        let latest_pool_activity_block = token
            .all_pool_bases()
            .into_iter()
            .filter_map(pool_reference_block)
            .max();
        let latest_activity_block = [
            token.latest_block_number,
            token.activity.latest_block_number,
            token.creation_block,
            latest_pool_activity_block,
        ]
        .into_iter()
        .flatten()
        .max();
        let active_pool_count = token
            .all_pool_bases()
            .into_iter()
            .filter(|pool| pool.effective_can_buy() || pool.effective_can_sell())
            .count() as u64;

        Self {
            scope_id: scope_id.into(),
            chain_id: 1,
            token_address: token.contract_address.clone(),
            source_run_id,
            as_of_block,
            source_reason: source_reason.into(),
            name: token.name.clone(),
            symbol: token.symbol.clone(),
            decimals: token.decimals,
            total_supply_raw: token.total_supply.clone(),
            creation_block: token.creation_block,
            creation_timestamp: token.creation_timestamp,
            creation_tx: token.creation_tx.clone(),
            creator_address: token.creator_address.clone(),
            latest_activity_block,
            latest_activity_timestamp: token
                .latest_block_timestamp
                .or(token.activity.latest_block_timestamp)
                .or(token.creation_timestamp),
            latest_pool_activity_block,
            lifecycle_status: token
                .token_life_cycle_status
                .as_ref()
                .and_then(serialized_enum_label),
            is_scam: token.is_scam(),
            scam_label: token.scam_label(),
            scam_mechanism: token.scam_mechanism(),
            scam_mechanism_label: token.scam_mechanism_label(),
            pool_count: token.pool_count() as u64,
            active_pool_count,
            retained_pool_count: token.pool_count() as u64,
            liquidity_removal_pool_count: token.liquidity_removal_pool_count() as u64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolLatestState {
    pub scope_id: String,
    pub chain_id: i64,
    pub token_address: String,
    pub pool_id: String,
    pub source_run_id: Option<String>,
    pub as_of_block: Option<u64>,
    pub source_reason: String,
    pub protocol: String,
    pub denom_address: String,
    pub token_decimals: u8,
    pub denom_decimals: u8,
    pub creation_block: Option<u64>,
    pub creation_timestamp: Option<u64>,
    pub creation_tx: Option<String>,
    pub latest_activity_block: Option<u64>,
    pub latest_sync_block: Option<u64>,
    pub latest_reserve_block: Option<u64>,
    pub latest_swap_block: Option<u64>,
    pub latest_mint_block: Option<u64>,
    pub latest_burn_block: Option<u64>,
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub total_liquidity: f64,
    pub price_denom_per_token: f64,
    pub price_token_per_denom: f64,
    pub valuation_status: String,
    pub lifecycle_status: Option<String>,
    pub is_dust_pool: bool,
    pub can_buy: bool,
    pub can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub economic_sellable: Option<bool>,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tax_check_block: Option<u64>,
    pub has_liquidity_removal: bool,
    pub liquidity_removal_block: Option<u64>,
    pub liquidity_removal_tx: Option<String>,
    pub liquidity_removal_label: Option<String>,
    pub total_swaps: u64,
    pub total_mints: u64,
    pub total_burns: u64,
}

impl PoolLatestState {
    pub fn from_pool(
        scope_id: impl Into<String>,
        source_run_id: Option<String>,
        as_of_block: Option<u64>,
        source_reason: impl Into<String>,
        pool: &BasePool,
    ) -> Self {
        let trading = pool.trading_status();
        let latest_reserve_block = pool
            .reserve_tracker
            .latest_snapshot
            .as_ref()
            .map(|snapshot| snapshot.block_number);
        let latest_activity_block = pool_reference_block(pool);
        let lifecycle_status = serialized_enum_label(&pool.state.lifecycle);
        let valuation_status = valuation_status(pool);
        let is_dust_pool = matches!(pool.state.lifecycle, PoolLifecycle::Dust);
        let scam_mechanism = pool.inferred_scam_mechanism();
        let liquidity_removal_label = pool
            .scam_label
            .clone()
            .or_else(|| pool.reserve_tracker.scam_label.clone())
            .or_else(|| scam_mechanism.map(|mechanism| mechanism.label));

        Self {
            scope_id: scope_id.into(),
            chain_id: 1,
            token_address: pool.identity.token_address.clone(),
            pool_id: pool.identity.pool_address.clone(),
            source_run_id,
            as_of_block,
            source_reason: source_reason.into(),
            protocol: pool.identity.protocol.clone(),
            denom_address: pool.identity.denom_address.clone(),
            token_decimals: pool.config.token_decimals,
            denom_decimals: pool.config.denom_decimals.unwrap_or(18),
            creation_block: pool.creation_block,
            creation_timestamp: pool.creation_timestamp,
            creation_tx: pool.creation_tx.clone(),
            latest_activity_block,
            latest_sync_block: non_zero_block(pool.state.last_sync_block),
            latest_reserve_block,
            latest_swap_block: latest_event_block(&pool.swap_events),
            latest_mint_block: latest_event_block(&pool.mint_events),
            latest_burn_block: latest_event_block(&pool.burn_events),
            token_reserve: finite_or_zero(pool.state.token_reserve),
            denom_reserve: finite_or_zero(pool.state.denom_reserve),
            total_liquidity: finite_or_zero(pool.state.total_liquidity),
            price_denom_per_token: finite_or_zero(pool.state.price_denom_per_token),
            price_token_per_denom: finite_or_zero(pool.state.price_token_per_denom),
            valuation_status,
            lifecycle_status,
            is_dust_pool,
            can_buy: trading.can_buy,
            can_sell: trading.can_sell,
            effective_can_buy: trading.effective_can_buy,
            effective_can_sell: trading.effective_can_sell,
            economic_sellable: trading.economic_sellable,
            buy_tax: trading.buy_tax.filter(|value| value.is_finite()),
            sell_tax: trading.sell_tax.filter(|value| value.is_finite()),
            tax_check_block: trading.tax_check_block,
            has_liquidity_removal: pool.has_liquidity_removal(),
            liquidity_removal_block: pool.scam_block.or(pool.reserve_tracker.scam_block),
            liquidity_removal_tx: pool
                .scam_tx_hash
                .clone()
                .or_else(|| pool.reserve_tracker.scam_tx_hash.clone()),
            liquidity_removal_label,
            total_swaps: pool.state.total_swaps,
            total_mints: pool.state.total_mints,
            total_burns: pool.state.total_burns,
        }
    }
}

fn pool_reference_block(pool: &BasePool) -> Option<u64> {
    [
        pool.latest_block_number,
        pool.can_buy_block,
        pool.creation_block,
        pool.price_history.last().map(|(block, _)| *block),
        pool.reserve_tracker
            .latest_snapshot
            .as_ref()
            .map(|snapshot| snapshot.block_number),
        non_zero_block(pool.state.last_update_block),
        non_zero_block(pool.state.last_sync_block),
        pool.scam_block,
        pool.reserve_tracker.scam_block,
    ]
    .into_iter()
    .flatten()
    .max()
}

fn latest_event_block(events: &[serde_json::Value]) -> Option<u64> {
    events
        .iter()
        .rev()
        .find_map(|event| event.get("block_number").and_then(|value| value.as_u64()))
}

fn non_zero_block(block: u64) -> Option<u64> {
    (block > 0).then_some(block)
}

fn serialized_enum_label<T>(value: &T) -> Option<String>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
}

fn valuation_status(pool: &BasePool) -> String {
    if pool.has_liquidity_removal() {
        return "liquidity_removed".to_string();
    }
    match pool.state.lifecycle {
        PoolLifecycle::Dust => "dust".to_string(),
        PoolLifecycle::Drained => "drained".to_string(),
        PoolLifecycle::Evicted => "evicted".to_string(),
        PoolLifecycle::LiquidityRemoved => "liquidity_removed".to_string(),
        _ if !pool.state.price_denom_per_token.is_finite()
            || !pool.state.price_token_per_denom.is_finite() =>
        {
            "invalid_price".to_string()
        }
        _ if pool.state.denom_reserve <= 0.0 || pool.state.token_reserve <= 0.0 => {
            "missing_reserves".to_string()
        }
        _ if pool.price() <= 0.0 => "no_economic_price".to_string(),
        _ => "priced".to_string(),
    }
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}
