use std::collections::BTreeMap;

use eth_token::contract_analysis::{analyze_erc20_token, ContractAnalysisReport};
use eth_token::erc20::{ERC20Token, TokenLifecycleState, TokenSummary};
use eth_token::tracking::TrackedTokenStatus;
use serde::Serialize;

use crate::range_indexer::{RangeIndexJob, RangeIndexState};
use crate::views::{network::TokenNetworkView, pool::PoolView};

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
    pub protocols: Vec<String>,
    pub pool_count_by_protocol: BTreeMap<String, usize>,
    pub has_pools: bool,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub hidden_mint_detected: bool,
    pub hidden_mint_block: Option<u64>,
    pub hidden_mint_tx: Option<String>,
    pub liquidity_removal_pool_count: usize,
    pub trading_enabled: bool,
    pub total_transactions: usize,
    pub activity_block_count: usize,
    pub total_buy_volume_by_denom: BTreeMap<String, f64>,
    pub total_sell_volume_by_denom: BTreeMap<String, f64>,
    pub total_bribe_eth: f64,
    pub unique_address_count: usize,
    pub current_owner: Option<String>,
    pub ownership_renounced: bool,
    pub contract_analysis: ContractAnalysisReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenDetailResponse {
    pub run_id: String,
    pub token: ERC20Token,
    pub summary: TokenSummary,
    pub index_status: Option<TrackedTokenStatus>,
    pub pools: Vec<PoolView>,
    pub network: TokenNetworkView,
    pub contract_analysis: ContractAnalysisReport,
}

impl TokenView {
    pub fn from_token(token: &ERC20Token, index_status: Option<TrackedTokenStatus>) -> Self {
        let summary = token.get_token_summary();
        let mut pool_count_by_protocol = BTreeMap::new();
        for pool in token.all_pool_bases() {
            *pool_count_by_protocol
                .entry(pool.identity.protocol.clone())
                .or_insert(0) += 1;
        }

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
            pool_count: token.pool_count(),
            protocols: summary.protocols,
            pool_count_by_protocol,
            has_pools: token.has_pool(),
            is_scam: token.is_scam(),
            scam_label: token.scam_label(),
            hidden_mint_detected: token.hidden_mint_detected(),
            hidden_mint_block: token.hidden_mint_block(),
            hidden_mint_tx: token.hidden_mint_tx(),
            liquidity_removal_pool_count: token.liquidity_removal_pool_count(),
            trading_enabled: token.trading_enabled(),
            total_transactions: token.tx_hashes_to_makers.len(),
            activity_block_count: token.activity.blocks.len(),
            total_buy_volume_by_denom: token.activity.total_buy_volume_by_denom(),
            total_sell_volume_by_denom: token.activity.total_sell_volume_by_denom(),
            total_bribe_eth: token.activity.total_bribe_eth(),
            unique_address_count: token.unique_addresses().len(),
            current_owner: token.current_owner(),
            ownership_renounced: token.ownership_renounced(),
            contract_analysis: analyze_erc20_token(token),
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
    let network = TokenNetworkView::from_graph(token, state.processor.network_graphs.get(&address));
    let pools = PoolView::from_token_pools(token);
    let contract_analysis = analyze_erc20_token(token);

    Some(TokenDetailResponse {
        run_id: run.id.clone(),
        token: token.clone(),
        summary: token.get_token_summary(),
        index_status,
        pools,
        network,
        contract_analysis,
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
