use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::address::normalize_address_string;
use crate::pools::base::BasePool;
use crate::pools::uniswap::v2::UniswapV2Pool;
use crate::pools::uniswap::{UniswapV3Pool, UniswapV4Pool};
use crate::token_activity::TokenBlockActivity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenLifecycleState {
    ContractCreation,
    PairCreation,
    TradingEnabled,
    #[serde(alias = "INACTIVE_SCAM", alias = "InactiveScam")]
    InactiveHiddenMint,
    InactiveOther,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ERC20TokenMetadata {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: String,
}

impl ERC20TokenMetadata {
    pub fn new(
        address: impl Into<String>,
        name: impl Into<String>,
        symbol: impl Into<String>,
        decimals: u8,
        total_supply: impl Into<String>,
    ) -> Self {
        Self {
            address: normalize_address_string(address),
            name: name.into(),
            symbol: symbol.into(),
            decimals,
            total_supply: total_supply.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolStateSnapshot {
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
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_mechanism: Option<String>,
    pub scam_mechanism_label: Option<String>,
    pub liquidity_removal: bool,
    pub liquidity_removal_label: Option<String>,
}

impl PoolStateSnapshot {
    pub fn from_base(base: &BasePool) -> Self {
        let scam_mechanism = base.inferred_scam_mechanism();
        let scam_mechanism_key = scam_mechanism
            .as_ref()
            .map(|mechanism| mechanism.mechanism.clone());
        let scam_mechanism_label = scam_mechanism
            .as_ref()
            .map(|mechanism| mechanism.label.clone());
        let scam_label = base
            .scam_label
            .clone()
            .or_else(|| scam_mechanism_label.clone());
        Self {
            pool_address: base.identity.pool_address.clone(),
            protocol: base.identity.protocol.clone(),
            denom_address: base.identity.denom_address.clone(),
            token_reserve: base.token_reserve(),
            denom_reserve: base.denom_reserve(),
            price: base.price(),
            total_liquidity: base.state.total_liquidity,
            can_buy: base.state.can_buy,
            can_sell: base.state.can_sell,
            trading_enabled: base.trading_enabled(),
            is_scam: base.has_liquidity_removal() || scam_mechanism.is_some(),
            scam_label: scam_label.clone(),
            scam_mechanism: scam_mechanism_key,
            scam_mechanism_label: scam_mechanism_label.clone(),
            liquidity_removal: base.has_liquidity_removal() || scam_mechanism.is_some(),
            liquidity_removal_label: scam_label,
        }
    }
}

impl From<&UniswapV2Pool> for PoolStateSnapshot {
    fn from(pool: &UniswapV2Pool) -> Self {
        Self::from_base(&pool.base)
    }
}

impl From<&UniswapV3Pool> for PoolStateSnapshot {
    fn from(pool: &UniswapV3Pool) -> Self {
        Self::from_base(&pool.base)
    }
}

impl From<&UniswapV4Pool> for PoolStateSnapshot {
    fn from(pool: &UniswapV4Pool) -> Self {
        Self::from_base(&pool.base)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenSummary {
    pub contract_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: String,
    pub token_life_cycle_status: Option<TokenLifecycleState>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_mechanism: Option<String>,
    pub scam_mechanism_label: Option<String>,
    pub hidden_mint_detected: bool,
    pub hidden_mint_block: Option<u64>,
    pub hidden_mint_tx: Option<String>,
    pub liquidity_removal_pool_count: usize,
    pub creation_block: Option<u64>,
    pub creator_address: Option<String>,
    pub has_pools: bool,
    pub pool_count: usize,
    pub protocols: Vec<String>,
    pub total_transactions: usize,
    pub activity_block_count: usize,
    pub total_buy_volume_by_denom: BTreeMap<String, f64>,
    pub total_sell_volume_by_denom: BTreeMap<String, f64>,
    pub total_bribe_eth: f64,
    pub recent_block_activity: Vec<TokenBlockActivity>,
    pub latest_block: Option<u64>,
    pub latest_timestamp: Option<u64>,
    pub total_liquidity_by_denom: HashMap<String, f64>,
    pub current_prices: HashMap<String, PoolStateSnapshot>,
}
