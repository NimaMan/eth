use eth_token::erc20::{ERC20Token, TokenLifecycleState, TokenSummary};
use eth_token::manager::TrackedTokenStatus;
use serde::Serialize;

use crate::range_indexer::{RangeIndexJob, RangeIndexState};
use crate::views::pool::PoolView;

#[derive(Clone, Debug, Serialize)]
pub struct TokenListResponse {
    pub run_id: String,
    pub count: usize,
    pub tokens: Vec<TokenView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenView {
    pub contract_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: String,
    pub total_supply_scaled: Option<f64>,
    pub index_status: Option<TrackedTokenStatus>,
    pub lifecycle_status: Option<TokenLifecycleState>,
    pub creation_block: Option<u64>,
    pub creation_timestamp: Option<u64>,
    pub creator_address: Option<String>,
    pub latest_block: Option<u64>,
    pub latest_timestamp: Option<u64>,
    pub pool_count: usize,
    pub has_pools: bool,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub trading_enabled: bool,
    pub total_transactions: usize,
    pub unique_address_count: usize,
    pub current_owner: Option<String>,
    pub ownership_renounced: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenDetailResponse {
    pub run_id: String,
    pub token: ERC20Token,
    pub summary: TokenSummary,
    pub index_status: Option<TrackedTokenStatus>,
    pub pools: Vec<PoolView>,
}

impl TokenView {
    pub fn from_token(token: &ERC20Token, index_status: Option<TrackedTokenStatus>) -> Self {
        Self {
            contract_address: token.contract_address.clone(),
            name: token.name.clone(),
            symbol: token.symbol.clone(),
            decimals: token.decimals,
            total_supply: token.total_supply.clone(),
            total_supply_scaled: token.total_supply_scaled(),
            index_status,
            lifecycle_status: token.token_life_cycle_status.clone(),
            creation_block: token.creation_block,
            creation_timestamp: token.creation_timestamp,
            creator_address: token.creator_address.clone(),
            latest_block: token.latest_block_number,
            latest_timestamp: token.latest_block_timestamp,
            pool_count: token.v2_pools.len(),
            has_pools: token.has_pool(),
            is_scam: token.is_scam(),
            scam_label: token.scam_label(),
            trading_enabled: token.trading_enabled(),
            total_transactions: token.tx_hashes_to_makers.len(),
            unique_address_count: token.unique_addresses().len(),
            current_owner: token.current_owner(),
            ownership_renounced: token.ownership_renounced(),
        }
    }
}

pub async fn token_list(run: &RangeIndexJob) -> TokenListResponse {
    let state = run.state.read().await;
    let mut tokens = Vec::with_capacity(state.processor.registry.tokens.len());

    for token in state.processor.registry.tokens.values() {
        let index_status = index_status(&state, &token.contract_address);
        tokens.push(TokenView::from_token(token, index_status));
    }

    tokens.sort_by(|left, right| {
        right
            .creation_timestamp
            .cmp(&left.creation_timestamp)
            .then(right.creation_block.cmp(&left.creation_block))
            .then(left.contract_address.cmp(&right.contract_address))
    });

    TokenListResponse {
        run_id: run.id.clone(),
        count: tokens.len(),
        tokens,
    }
}

pub async fn token_detail(run: &RangeIndexJob, token_address: &str) -> Option<TokenDetailResponse> {
    let state = run.state.read().await;
    let address = normalize_address(token_address);
    let token = state.processor.registry.tokens.get(&address)?;
    let index_status = index_status(&state, &address);
    let mut pools = token
        .v2_pools
        .values()
        .map(|pool| PoolView::from_pool(token, pool))
        .collect::<Vec<_>>();
    pools.sort_by(|left, right| left.pool_address.cmp(&right.pool_address));

    Some(TokenDetailResponse {
        run_id: run.id.clone(),
        token: token.clone(),
        summary: token.get_token_summary(),
        index_status,
        pools,
    })
}

fn index_status(state: &RangeIndexState, token_address: &str) -> Option<TrackedTokenStatus> {
    state
        .processor
        .token_index
        .entries
        .get(&normalize_address(token_address))
        .map(|entry| entry.token_status.clone())
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
