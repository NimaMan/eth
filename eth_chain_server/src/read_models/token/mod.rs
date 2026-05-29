use std::collections::{BTreeMap, HashSet};

use eth_token::contract_analysis::{
    analyze_erc20_token, BehaviorFlagKind, ContractAnalysisReport, ContractEvidence,
    ContractEvidenceSource, ContractSeverity,
};
use eth_token::erc20::{ERC20Token, TokenLifecycleState, TokenSummary};
use eth_token::pnl::{AddressPoolPnlSummary, PoolPnlConservationSummary};
use rust_decimal::Decimal;
use eth_token::token_analytics::{
    build_historical_observations_for_token, TokenPoolCurrentObservation,
};
use eth_token::tracking::TrackedTokenStatus;
use serde::Serialize;

use crate::ranges::{RangeIndexJob, RangeIndexState};

fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_string().parse().unwrap_or(0.0)
}
use crate::read_models::{pool::PoolView, token_analytics::TokenNetworkView};

pub mod activity;

const TOKEN_PNL_TOP_POSITION_LIMIT: usize = 25;
const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

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
    pub observations: Vec<TokenPoolCurrentObservation>,
    pub network: TokenNetworkView,
    pub pnl: TokenPnlView,
    pub contract_analysis: ContractAnalysisReport,
    pub denom_symbols: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPnlView {
    pub pool_count: usize,
    pub total_position_count: usize,
    pub total_display_position_count: usize,
    pub total_tx_count: u64,
    pub all_token_conserved: bool,
    pub all_denom_conserved: bool,
    pub pools: Vec<TokenPoolPnlView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolPnlView {
    pub pool_address: String,
    pub protocol: Option<String>,
    pub denom_address: String,
    pub denom_symbol: Option<String>,
    pub currency: String,
    pub token_decimals: u8,
    pub denom_decimals: u8,
    pub price: Option<f64>,
    pub tx_count: u64,
    pub position_count: usize,
    pub display_position_count: usize,
    pub omitted_position_count: usize,
    pub latest_block_number: Option<u64>,
    pub latest_block_timestamp: Option<u64>,
    pub conservation: PoolPnlConservationSummary,
    pub top_positions: Vec<TokenPoolPnlAddressView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolPnlAddressView {
    pub address: String,
    pub token_balance_raw: String,
    pub denom_cashflow_raw: String,
    pub native_fee_raw: String,
    pub native_bribe_raw: String,
    pub token_balance: f64,
    pub denom_cashflow: f64,
    pub native_fee: f64,
    pub native_bribe: f64,
    pub token_in: f64,
    pub token_out: f64,
    pub denom_in: f64,
    pub denom_out: f64,
    pub realized_pnl_denom: Option<f64>,
    pub marked_token_value_denom: Option<f64>,
    pub pnl_proxy_denom: Option<f64>,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub first_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub movement_count: u64,
}

impl TokenView {
    pub fn from_token(token: &ERC20Token, index_status: Option<TrackedTokenStatus>) -> Self {
        let pool_views = PoolView::from_token_pools(token);
        let summary = token_summary_with_pool_views(token, &pool_views);
        let liquidity_removal_pool_count = summary.liquidity_removal_pool_count;
        let scam_label = summary.scam_label.clone();
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
            is_scam: summary.is_scam,
            scam_label,
            hidden_mint_detected: token.hidden_mint_detected(),
            hidden_mint_block: token.hidden_mint_block(),
            hidden_mint_tx: token.hidden_mint_tx(),
            liquidity_removal_pool_count,
            trading_enabled: token.trading_enabled(),
            total_transactions: token.tx_hashes_to_makers.len(),
            activity_block_count: token.activity.blocks.len(),
            total_buy_volume_by_denom: token.activity.total_buy_volume_by_denom(),
            total_sell_volume_by_denom: token.activity.total_sell_volume_by_denom(),
            total_bribe_eth: token.activity.total_bribe_eth(),
            unique_address_count: token.unique_addresses().len(),
            current_owner: token.current_owner(),
            ownership_renounced: token.ownership_renounced(),
            contract_analysis: contract_analysis_with_pool_views(token, &pool_views),
        }
    }
}

pub(crate) fn token_summary_with_pool_views(
    token: &ERC20Token,
    pool_views: &[PoolView],
) -> TokenSummary {
    let mut summary = token.get_token_summary();
    let liquidity_removal_pool_count = pool_views
        .iter()
        .filter(|pool| pool.liquidity_removal)
        .count();

    if liquidity_removal_pool_count > summary.liquidity_removal_pool_count {
        summary.liquidity_removal_pool_count = liquidity_removal_pool_count;
        summary.is_scam = true;
        if summary.scam_label.is_none() {
            summary.scam_label = Some("liquidity_removal".to_string());
        }
        if summary.scam_mechanism.is_none() {
            if let Some(pool) = pool_views.iter().find(|pool| pool.scam_mechanism.is_some()) {
                summary.scam_mechanism = pool.scam_mechanism.clone();
                summary.scam_mechanism_label = pool.scam_mechanism_label.clone();
                summary.scam_label = pool.scam_label.clone().or(summary.scam_label);
            }
        }
    }

    for pool in pool_views {
        if let Some(snapshot) = summary.current_prices.get_mut(&pool.pool_address) {
            snapshot.is_scam = pool.is_scam;
            snapshot.scam_label = pool.scam_label.clone();
            snapshot.scam_mechanism = pool.scam_mechanism.clone();
            snapshot.scam_mechanism_label = pool.scam_mechanism_label.clone();
            snapshot.liquidity_removal = pool.liquidity_removal;
            snapshot.liquidity_removal_label = pool.liquidity_removal_label.clone();
        }
    }

    summary
}

pub(crate) fn contract_analysis_with_pool_views(
    token: &ERC20Token,
    pool_views: &[PoolView],
) -> ContractAnalysisReport {
    let mut report = analyze_erc20_token(token);
    let liquidity_removal_pool_count = pool_views
        .iter()
        .filter(|pool| pool.liquidity_removal)
        .count();

    if liquidity_removal_pool_count > report.pools.liquidity_removal_pool_count {
        report.pools.liquidity_removal_pool_count = liquidity_removal_pool_count;
        if !report
            .behavior_flags
            .contains(&BehaviorFlagKind::PoolLiquidityRemovalEvidence)
        {
            report
                .behavior_flags
                .push(BehaviorFlagKind::PoolLiquidityRemovalEvidence);
        }
        if !report
            .evidence
            .iter()
            .any(|evidence| evidence.kind == BehaviorFlagKind::PoolLiquidityRemovalEvidence)
        {
            report.evidence.push(ContractEvidence::new(
                BehaviorFlagKind::PoolLiquidityRemovalEvidence,
                ContractSeverity::High,
                ContractEvidenceSource::Pools,
                format!("{liquidity_removal_pool_count} pool(s) show liquidity-removal evidence"),
            ));
        }
    }

    report
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

pub fn build_denom_symbols(token: &ERC20Token) -> BTreeMap<String, String> {
    let mut symbols = BTreeMap::new();
    for pool in token.all_pool_bases() {
        if let Some(symbol) = crate::read_models::pool::denom_symbol(&pool.identity.denom_address) {
            symbols.insert(pool.identity.denom_address.clone(), symbol);
        }
    }
    symbols
}

impl TokenPnlView {
    pub fn from_token(token: &ERC20Token) -> Self {
        let mut pools = token
            .pnl
            .pools()
            .map(|(pool_address, pool)| {
                let base = token.pool_base(pool_address);
                let mark_price = base
                    .map(|base| base.price())
                    .filter(|price| price.is_finite() && *price > 0.0);
                let denom_symbol = base
                    .and_then(|base| {
                        crate::read_models::pool::denom_symbol(&base.identity.denom_address)
                    })
                    .or_else(|| crate::read_models::pool::denom_symbol(&pool.denom_address));
                let currency = denom_symbol
                    .clone()
                    .unwrap_or_else(|| pool.denom_address.clone());
                let display_position_count = pool
                    .positions
                    .values()
                    .filter(|position| {
                        position.address != pool.pool_address && position.address != ZERO_ADDRESS
                    })
                    .count();
                let top_positions = pool
                    .top_positions_by_denom_volume(TOKEN_PNL_TOP_POSITION_LIMIT, false, mark_price)
                    .into_iter()
                    .map(|summary| {
                        TokenPoolPnlAddressView::from_summary(
                            summary,
                            pool.token_decimals,
                            pool.denom_decimals,
                        )
                    })
                    .collect::<Vec<_>>();

                TokenPoolPnlView {
                    pool_address: pool.pool_address.clone(),
                    protocol: base.map(|base| base.identity.protocol.clone()),
                    denom_address: pool.denom_address.clone(),
                    denom_symbol,
                    currency,
                    token_decimals: pool.token_decimals,
                    denom_decimals: pool.denom_decimals,
                    price: mark_price,
                    tx_count: pool.tx_count,
                    position_count: pool.positions.len(),
                    display_position_count,
                    omitted_position_count: display_position_count
                        .saturating_sub(top_positions.len()),
                    latest_block_number: pool.latest_block_number,
                    latest_block_timestamp: pool.latest_block_timestamp,
                    conservation: pool.conservation_summary(),
                    top_positions,
                }
            })
            .collect::<Vec<_>>();

        pools.sort_by(|left, right| {
            right
                .tx_count
                .cmp(&left.tx_count)
                .then_with(|| {
                    right
                        .display_position_count
                        .cmp(&left.display_position_count)
                })
                .then_with(|| left.pool_address.cmp(&right.pool_address))
        });

        let pool_count = pools.len();
        let total_position_count = pools.iter().map(|pool| pool.position_count).sum();
        let total_display_position_count =
            pools.iter().map(|pool| pool.display_position_count).sum();
        let total_tx_count = pools.iter().map(|pool| pool.tx_count).sum();
        let all_token_conserved = pools
            .iter()
            .all(|pool| pool.conservation.token_is_conserved);
        let all_denom_conserved = pools
            .iter()
            .all(|pool| pool.conservation.denom_is_conserved);

        Self {
            pool_count,
            total_position_count,
            total_display_position_count,
            total_tx_count,
            all_token_conserved,
            all_denom_conserved,
            pools,
        }
    }
}

impl TokenPoolPnlAddressView {
    fn from_summary(
        summary: AddressPoolPnlSummary,
        token_decimals: u8,
        denom_decimals: u8,
    ) -> Self {
        let realized_pnl_denom = summary
            .pnl_proxy_denom
            .map(|pnl| pnl - summary.marked_token_value_denom.unwrap_or_default());
        Self {
            address: summary.address,
            token_in: scale_raw_decimal(&summary.token_in_raw, token_decimals),
            token_out: scale_raw_decimal(&summary.token_out_raw, token_decimals),
            denom_in: scale_raw_decimal(&summary.denom_in_raw, denom_decimals),
            denom_out: scale_raw_decimal(&summary.denom_out_raw, denom_decimals),
            token_balance_raw: summary.token_balance_raw,
            denom_cashflow_raw: summary.denom_cashflow_raw,
            native_fee_raw: summary.native_fee_raw,
            native_bribe_raw: summary.native_bribe_raw,
            token_balance: decimal_to_f64(summary.token_balance),
            denom_cashflow: decimal_to_f64(summary.denom_cashflow),
            native_fee: decimal_to_f64(summary.native_fee),
            native_bribe: decimal_to_f64(summary.native_bribe),
            realized_pnl_denom: realized_pnl_denom.map(decimal_to_f64),
            marked_token_value_denom: summary.marked_token_value_denom.map(decimal_to_f64),
            pnl_proxy_denom: summary.pnl_proxy_denom.map(decimal_to_f64),
            token_in_raw: summary.token_in_raw,
            token_out_raw: summary.token_out_raw,
            denom_in_raw: summary.denom_in_raw,
            denom_out_raw: summary.denom_out_raw,
            first_block: summary.first_block,
            latest_block: summary.latest_block,
            movement_count: summary.movement_count,
        }
    }
}

fn scale_raw_decimal(raw: &str, decimals: u8) -> f64 {
    raw.parse::<f64>().unwrap_or(0.0) / 10_f64.powi(i32::from(decimals))
}

pub async fn token_detail(run: &RangeIndexJob, token_address: &str) -> Option<TokenDetailResponse> {
    let state = run.state.read().await;
    let address = normalize_address(token_address);
    let token = state.processor.registry.tokens.get(&address)?;
    let index_status = index_status(&state, &address);
    let network = TokenNetworkView::from_graph(token, state.processor.network_graphs.get(&address));
    let recent_activity = token.activity.recent_blocks(50);
    let pools = PoolView::from_token_pools_with_activity(token, &recent_activity);
    let summary = token_summary_with_pool_views(token, &pools);
    let contract_analysis = contract_analysis_with_pool_views(token, &pools);
    let denom_symbols = build_denom_symbols(token);
    let pnl = TokenPnlView::from_token(token);
    let observations = token_observations_with_backfill(&state.observations, &address, token);

    Some(TokenDetailResponse {
        run_id: run.id.clone(),
        token: token.clone(),
        summary,
        index_status,
        pools,
        observations,
        network,
        pnl,
        contract_analysis,
        denom_symbols,
    })
}

pub fn token_observations(
    observations: &[TokenPoolCurrentObservation],
    token_address: &str,
) -> Vec<TokenPoolCurrentObservation> {
    observations
        .iter()
        .filter(|observation| {
            observation
                .key
                .token_address
                .eq_ignore_ascii_case(token_address)
        })
        .cloned()
        .collect()
}

pub fn token_observations_with_backfill(
    observations: &[TokenPoolCurrentObservation],
    token_address: &str,
    token: &ERC20Token,
) -> Vec<TokenPoolCurrentObservation> {
    let mut rows = token_observations(observations, token_address);
    let covered_pools: HashSet<String> = rows
        .iter()
        .map(|observation| observation.key.pool_address.to_ascii_lowercase())
        .collect();
    rows.extend(
        build_historical_observations_for_token(token)
            .into_iter()
            .filter(|observation| {
                !covered_pools.contains(&observation.key.pool_address.to_ascii_lowercase())
            }),
    );
    rows.sort_by(|left, right| {
        left.key
            .pool_address
            .cmp(&right.key.pool_address)
            .then(left.context.block_number.cmp(&right.context.block_number))
            .then(
                left.context
                    .active_observation_index
                    .cmp(&right.context.active_observation_index),
            )
    });
    rows
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
