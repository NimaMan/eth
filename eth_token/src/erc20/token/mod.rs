mod activity;
mod events;
mod pools;

use std::collections::{HashMap, HashSet};

use eyre::{eyre, Result};
use reth_chain_query::common_addresses::{KnownV2Protocol, KnownV3Protocol, DENOM_ADDRESSES};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_processor::ProcessedTransaction;

use crate::custody::CustodyFinding;
use crate::pnl::TokenPnlTracker;
use crate::pools::balancer::{BalancerPool, BalancerPoolToken};
use crate::pools::base::{BasePool, BasePoolConfig};
use crate::pools::curve::{CurvePool, CurvePoolToken};
use crate::pools::pancakeswap::new_pancakeswap_v3_pool;
use crate::pools::sushiswap::{new_sushiswap_v2_pool, new_sushiswap_v3_pool};
use crate::pools::uniswap::v2::{UniswapV2Pool, UniswapV2TransactionEvents, UniswapV2TxContext};
use crate::pools::uniswap::{v4_event_display_key, UniswapV3Pool, UniswapV4Pool};
use crate::pools::PoolStateFlags;
use crate::state::{TokenAuthorityTracker, TokenStatusManager, TokenTransferTracker};
use crate::token_activity::TokenActivityTracker;
use crate::utils::scale_raw_units;

pub const DEFAULT_TOKEN_HISTORY_LIMIT: usize = 1000;

use self::events::{
    lp_approvals_from_processed_transaction, lp_transfers_from_processed_transaction,
    signed_denom_swap_amounts, v2_events_from_processed_transaction,
};
use super::address::{
    address_string, hash_string, normalize_address, normalize_address_string, parse_address_lossy,
    same_address,
};
use super::types::{ERC20TokenMetadata, PoolStateSnapshot, TokenLifecycleState, TokenSummary};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ERC20Token {
    pub contract_address: String,
    #[serde(default)]
    pub is_live_mode: bool,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: String,
    pub history_limit: usize,
    pub creation_block: Option<u64>,
    pub creation_timestamp: Option<u64>,
    pub creation_tx: Option<String>,
    pub creator_address: Option<String>,
    pub creator_nonce: Option<u64>,
    pub token_life_cycle_status: Option<TokenLifecycleState>,
    #[serde(default)]
    pub activity: TokenActivityTracker,
    #[serde(default)]
    pub pnl: TokenPnlTracker,
    pub tx_hashes_to_makers: HashMap<String, String>,
    pub transaction_fees: Vec<Value>,
    pub latest_block_number: Option<u64>,
    pub latest_block_timestamp: Option<u64>,
    pub token_control_addresses: HashSet<String>,
    pub transfer_tracker: TokenTransferTracker,
    pub authority_tracker: TokenAuthorityTracker,
    pub status_manager: TokenStatusManager,
    /// Custody-risk findings: privileged powers observed firing against a
    /// holder's balance (e.g. the holder-balance backdoor drain that took our
    /// vault's tokens). Orthogonal to `can_sell`/pool scam labels — this axis
    /// records *who lost tokens*, not whether the swap path works. Derived only
    /// from the ProcessedTransaction (the event-less internal `transferFrom`).
    #[serde(default)]
    pub custody_findings: Vec<CustodyFinding>,
    #[serde(default)]
    pub v2_pools: HashMap<String, UniswapV2Pool>,
    #[serde(default)]
    pub v3_pools: HashMap<String, UniswapV3Pool>,
    #[serde(default)]
    pub v4_pools: HashMap<String, UniswapV4Pool>,
    #[serde(default)]
    pub curve_pools: HashMap<String, CurvePool>,
    #[serde(default)]
    pub balancer_pools: HashMap<String, BalancerPool>,
}

impl ERC20Token {
    pub fn new(metadata: ERC20TokenMetadata) -> Self {
        Self::with_live_mode(metadata, false)
    }

    pub fn with_live_mode(metadata: ERC20TokenMetadata, is_live_mode: bool) -> Self {
        let contract_address = metadata.address;
        let decimals = metadata.decimals;
        let total_supply = metadata.total_supply;
        Self {
            contract_address: contract_address.clone(),
            is_live_mode,
            name: metadata.name,
            symbol: metadata.symbol,
            decimals,
            total_supply: total_supply.clone(),
            history_limit: DEFAULT_TOKEN_HISTORY_LIMIT,
            creation_block: None,
            creation_timestamp: None,
            creation_tx: None,
            creator_address: None,
            creator_nonce: None,
            token_life_cycle_status: None,
            activity: TokenActivityTracker::default(),
            pnl: TokenPnlTracker::default(),
            tx_hashes_to_makers: HashMap::new(),
            transaction_fees: Vec::new(),
            latest_block_number: None,
            latest_block_timestamp: None,
            token_control_addresses: HashSet::new(),
            transfer_tracker: TokenTransferTracker::new(
                contract_address,
                decimals,
                DEFAULT_TOKEN_HISTORY_LIMIT,
            ),
            authority_tracker: TokenAuthorityTracker::new(DEFAULT_TOKEN_HISTORY_LIMIT),
            status_manager: TokenStatusManager::new(total_supply),
            custody_findings: Vec::new(),
            v2_pools: HashMap::new(),
            v3_pools: HashMap::new(),
            v4_pools: HashMap::new(),
            curve_pools: HashMap::new(),
            balancer_pools: HashMap::new(),
        }
    }

    pub fn set_live_mode(&mut self, is_live_mode: bool) {
        self.is_live_mode = is_live_mode;
    }

    pub fn handle_contract_creation(
        &mut self,
        block_number: u64,
        block_timestamp: u64,
        tx_hash: impl Into<String>,
        creator_address: impl Into<String>,
        creator_nonce: u64,
    ) {
        let creator_address = normalize_address_string(creator_address);
        self.creation_block = Some(block_number);
        self.creation_timestamp = Some(block_timestamp);
        self.creation_tx = Some(tx_hash.into());
        self.creator_address = Some(creator_address.clone());
        self.creator_nonce = Some(creator_nonce);
        self.token_life_cycle_status = Some(TokenLifecycleState::ContractCreation);
        self.token_control_addresses.insert(creator_address);
        let creator = self.creator_address.clone().unwrap_or_default();
        self.authority_tracker
            .register([parse_address_lossy(&creator)]);
        self.register_control_addresses_with_pools([creator]);
    }

    pub fn trading_enabled(&self) -> bool {
        self.all_pool_bases()
            .iter()
            .any(|pool| pool.trading_enabled())
    }

    pub fn is_scam(&self) -> bool {
        self.hidden_mint_detected()
            || self.all_pool_bases().iter().any(|pool| {
                pool.has_liquidity_removal() || pool.inferred_scam_mechanism().is_some()
            })
    }

    /// Custody-risk findings (powers observed firing against holder balances).
    /// Orthogonal to `is_scam()`/sellability — these identify *who lost tokens*.
    pub fn custody_findings(&self) -> &[CustodyFinding] {
        &self.custody_findings
    }

    /// Total scaled token amount confiscated from `holder` across all recorded
    /// custody findings (the victim's `from` side of an event-less drain). Use
    /// this to zero a position held at that address once it has been drained.
    pub fn custody_drained_amount(&self, holder: &str) -> f64 {
        let holder = normalize_address(holder);
        self.custody_findings
            .iter()
            .filter(|finding| {
                finding
                    .evidence
                    .get("victim")
                    .and_then(|v| v.as_str())
                    .map(|v| normalize_address(v) == holder)
                    .unwrap_or(false)
            })
            .filter_map(|finding| {
                finding
                    .evidence
                    .get("amount_scaled")
                    .and_then(|v| v.as_f64())
            })
            .sum()
    }

    pub fn scam_label(&self) -> Option<String> {
        self.status_manager.scam_label.clone().or_else(|| {
            self.all_pool_bases().into_iter().find_map(|pool| {
                pool.scam_label
                    .clone()
                    .or_else(|| {
                        pool.inferred_scam_mechanism()
                            .map(|mechanism| mechanism.label)
                    })
                    .or_else(|| {
                        pool.has_liquidity_removal()
                            .then(|| "liquidity_removal".to_string())
                    })
            })
        })
    }

    pub fn scam_mechanism(&self) -> Option<String> {
        if self.hidden_mint_detected() {
            return Some("hidden_mint_supply_expansion".to_string());
        }
        self.all_pool_bases().into_iter().find_map(|pool| {
            pool.inferred_scam_mechanism()
                .map(|mechanism| mechanism.mechanism)
        })
    }

    pub fn scam_mechanism_label(&self) -> Option<String> {
        if self.hidden_mint_detected() {
            return Some("Hidden Mint Supply Expansion".to_string());
        }
        self.all_pool_bases().into_iter().find_map(|pool| {
            pool.inferred_scam_mechanism()
                .map(|mechanism| mechanism.label)
        })
    }

    pub fn hidden_mint_detected(&self) -> bool {
        self.status_manager.is_scam
            && self.status_manager.scam_label.as_deref() == Some("hidden_mint")
    }

    pub fn hidden_mint_block(&self) -> Option<u64> {
        self.hidden_mint_detected()
            .then_some(self.status_manager.scam_block)
            .flatten()
    }

    pub fn hidden_mint_tx(&self) -> Option<String> {
        self.hidden_mint_detected()
            .then(|| self.status_manager.scam_tx.clone())
            .flatten()
    }

    pub fn liquidity_removal_pool_count(&self) -> usize {
        self.all_pool_bases()
            .iter()
            .filter(|pool| pool.has_liquidity_removal())
            .count()
    }

    pub fn total_bribe_amount(&self) -> f64 {
        self.transfer_tracker.total_bribe_amount
    }

    pub fn total_supply_from_transfers(&self) -> f64 {
        self.transfer_tracker.total_supply_from_transfers
    }

    pub fn total_supply_scaled(&self) -> Option<f64> {
        scale_raw_units(&self.total_supply, self.decimals)
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
    }

    pub fn unique_addresses(&self) -> Vec<String> {
        let mut addresses: Vec<_> = self
            .transfer_tracker
            .address_tx_counter
            .keys()
            .cloned()
            .collect();
        addresses.sort();
        addresses
    }

    pub fn current_owner(&self) -> Option<String> {
        self.authority_tracker.current_owner.clone()
    }

    pub fn ownership_renounced(&self) -> bool {
        self.authority_tracker.ownership_renounced
    }

    pub fn trading_enabled_block(&self) -> Option<u64> {
        self.all_pool_bases()
            .into_iter()
            .filter_map(|pool| pool.can_buy_block)
            .min()
    }

    pub fn trading_enabled_tx(&self) -> Option<String> {
        self.all_pool_bases()
            .into_iter()
            .filter_map(|pool| {
                pool.can_buy_block
                    .zip(pool.can_buy_tx.as_ref())
                    .map(|(block, tx)| (block, tx))
            })
            .min_by_key(|(block, _)| *block)
            .map(|(_, tx)| tx.clone())
    }

    pub fn all_pool_reserves(&self) -> HashMap<String, Value> {
        self.all_pool_bases()
            .into_iter()
            .map(|pool| {
                let address = pool.identity.pool_address.clone();
                (
                    address,
                    json!({
                        "token_reserve": pool.token_reserve(),
                        "denom_reserve": pool.denom_reserve(),
                        "denom_address": pool.identity.denom_address.clone(),
                        "protocol": pool.identity.protocol.clone(),
                    }),
                )
            })
            .collect()
    }

    pub fn get_token_summary(&self) -> TokenSummary {
        let mut protocols: Vec<_> = self
            .all_pool_bases()
            .into_iter()
            .map(|pool| pool.identity.protocol.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        protocols.sort();

        TokenSummary {
            contract_address: self.contract_address.clone(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            decimals: self.decimals,
            total_supply: self.total_supply.clone(),
            token_life_cycle_status: self.token_life_cycle_status.clone(),
            is_scam: self.is_scam(),
            scam_label: self.scam_label(),
            scam_mechanism: self.scam_mechanism(),
            scam_mechanism_label: self.scam_mechanism_label(),
            hidden_mint_detected: self.hidden_mint_detected(),
            hidden_mint_block: self.hidden_mint_block(),
            hidden_mint_tx: self.hidden_mint_tx(),
            liquidity_removal_pool_count: self.liquidity_removal_pool_count(),
            creation_block: self.creation_block,
            creator_address: self.creator_address.clone(),
            has_pools: self.has_pool(),
            pool_count: self.pool_count(),
            protocols,
            total_transactions: self.tx_hashes_to_makers.len(),
            activity_block_count: self.activity.blocks.len(),
            total_buy_volume_by_denom: self.activity.total_buy_volume_by_denom(),
            total_sell_volume_by_denom: self.activity.total_sell_volume_by_denom(),
            total_bribe_eth: self.activity.total_bribe_eth(),
            recent_block_activity: self.activity.recent_blocks(50),
            latest_block: self.latest_block_number,
            latest_timestamp: self.latest_block_timestamp,
            total_liquidity_by_denom: self.total_liquidity_by_denom(),
            current_prices: self.current_prices(),
        }
    }

    pub fn to_json_value(&self) -> Value {
        json!({
            "token_data": self,
            "summary": self.get_token_summary(),
            "is_scam": self.is_scam(),
            "scam_reason": self.scam_label().unwrap_or_default(),
            "hidden_mint_detected": self.hidden_mint_detected(),
            "liquidity_removal_pool_count": self.liquidity_removal_pool_count(),
        })
    }

    pub(crate) fn refresh_lifecycle_status(&mut self) {
        if self.hidden_mint_detected() {
            self.token_life_cycle_status = Some(TokenLifecycleState::InactiveHiddenMint);
        } else if self.is_scam() {
            self.token_life_cycle_status = Some(TokenLifecycleState::InactiveOther);
        } else if self.trading_enabled() {
            self.token_life_cycle_status = Some(TokenLifecycleState::TradingEnabled);
        } else if self.has_pool() {
            self.token_life_cycle_status = Some(TokenLifecycleState::PairCreation);
        } else if self.creation_block.is_some() {
            self.token_life_cycle_status = Some(TokenLifecycleState::ContractCreation);
        } else {
            self.token_life_cycle_status = None;
        }
    }
}

#[cfg(test)]
mod tests;
