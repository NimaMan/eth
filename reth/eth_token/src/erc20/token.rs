use std::collections::{HashMap, HashSet};

use alloy_primitives::Address;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_processor::ProcessedTransaction;

use crate::pools::balancer::{BalancerPool, BalancerPoolToken};
use crate::pools::base::{BasePool, BasePoolConfig};
use crate::pools::curve::{CurvePool, CurvePoolToken};
use crate::pools::sushiswap::new_sushiswap_v2_pool;
use crate::pools::uniswap::v2::{
    LPApprovalEvent, LPTransferEvent, UniswapV2BurnEvent, UniswapV2MintEvent, UniswapV2Pool,
    UniswapV2SwapEvent, UniswapV2SyncEvent, UniswapV2TransactionEvents, UniswapV2TxContext,
};
use crate::pools::uniswap::{UniswapV3Pool, UniswapV4Pool};
use crate::state::{TokenAuthorityTracker, TokenStatusManager, TokenTransferTracker};
use crate::utils::scale_raw_units;

pub const DEFAULT_TOKEN_HISTORY_LIMIT: usize = 1000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenLifecycleState {
    ContractCreation,
    PairCreation,
    TradingEnabled,
    InactiveScam,
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
}

impl PoolStateSnapshot {
    pub fn from_base(base: &BasePool) -> Self {
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
            is_scam: base.is_scam(),
            scam_label: base.scam_label.clone(),
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
    pub creation_block: Option<u64>,
    pub creator_address: Option<String>,
    pub has_pools: bool,
    pub pool_count: usize,
    pub protocols: Vec<String>,
    pub total_transactions: usize,
    pub latest_block: Option<u64>,
    pub latest_timestamp: Option<u64>,
    pub total_liquidity_by_denom: HashMap<String, f64>,
    pub current_prices: HashMap<String, PoolStateSnapshot>,
}

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
    pub tx_hashes_to_makers: HashMap<String, String>,
    pub transaction_fees: Vec<Value>,
    pub latest_block_number: Option<u64>,
    pub latest_block_timestamp: Option<u64>,
    pub token_control_addresses: HashSet<String>,
    pub transfer_tracker: TokenTransferTracker,
    pub authority_tracker: TokenAuthorityTracker,
    pub status_manager: TokenStatusManager,
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

    pub fn create_uniswap_v2_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut UniswapV2Pool {
        config.token_decimals = self.decimals;
        let pool = UniswapV2Pool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            known_routers,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v2_pool(pool);
        self.v2_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_uniswap_v2_pool(&mut self, pool: UniswapV2Pool) -> Option<UniswapV2Pool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let previous = self.v2_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn uniswap_v2_pool(&self, pool_address: impl AsRef<str>) -> Option<&UniswapV2Pool> {
        self.v2_pools.get(&normalize_address(pool_address))
    }

    pub fn uniswap_v2_pool_mut(
        &mut self,
        pool_address: impl AsRef<str>,
    ) -> Option<&mut UniswapV2Pool> {
        self.v2_pools.get_mut(&normalize_address(pool_address))
    }

    pub fn update_uniswap_v2_pool_events(
        &mut self,
        pool_address: impl AsRef<str>,
        events: &UniswapV2TransactionEvents,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool_address = normalize_address(pool_address);
        self.record_transaction_metadata(
            &tx.tx_hash,
            tx.from_address.as_deref(),
            tx.block_number,
            tx.block_timestamp,
        );
        let pool = self
            .uniswap_v2_pool_mut(&pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V2 pool {pool_address}"))?;
        pool.update_from_events(events, tx)?;
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn update_uniswap_v2_pool_from_processed_transaction(
        &mut self,
        pool_address: impl AsRef<str>,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        let pool_address = normalize_address(pool_address);
        let tx_context = UniswapV2TxContext {
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_hash: hash_string(&transaction.hash),
            from_address: Some(address_string(&transaction.from_address)),
        };
        let events = v2_events_from_processed_transaction(transaction, &pool_address);

        self.record_transaction_metadata(
            &tx_context.tx_hash,
            tx_context.from_address.as_deref(),
            tx_context.block_number,
            tx_context.block_timestamp,
        );
        let pool = self
            .uniswap_v2_pool_mut(&pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V2 pool {pool_address}"))?;
        pool.update_from_events(&events, &tx_context)?;
        for transfer in lp_transfers_from_processed_transaction(transaction, &pool_address) {
            pool.process_lp_transfer(&transfer)?;
        }
        for approval in lp_approvals_from_processed_transaction(transaction, &pool_address) {
            pool.process_lp_approval(&approval)?;
        }
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn create_sushiswap_v2_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut UniswapV2Pool {
        config.token_decimals = self.decimals;
        let pool = new_sushiswap_v2_pool(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            known_routers,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v2_pool(pool);
        self.v2_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn create_uniswap_v3_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        token0: impl Into<String>,
        token1: impl Into<String>,
        fee_tier: u32,
        tick_spacing: i32,
        mut config: BasePoolConfig,
    ) -> &mut UniswapV3Pool {
        config.token_decimals = self.decimals;
        let pool = UniswapV3Pool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            token0,
            token1,
            fee_tier,
            tick_spacing,
            config,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v3_pool(pool);
        self.v3_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_uniswap_v3_pool(&mut self, pool: UniswapV3Pool) -> Option<UniswapV3Pool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let previous = self.v3_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn uniswap_v3_pool(&self, pool_address: impl AsRef<str>) -> Option<&UniswapV3Pool> {
        self.v3_pools.get(&normalize_address(pool_address))
    }

    pub fn uniswap_v3_pool_mut(
        &mut self,
        pool_address: impl AsRef<str>,
    ) -> Option<&mut UniswapV3Pool> {
        self.v3_pools.get_mut(&normalize_address(pool_address))
    }

    pub fn update_uniswap_v3_pool_from_processed_transaction(
        &mut self,
        pool_address: impl AsRef<str>,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        let pool_address = normalize_address(pool_address);
        let tx_context = UniswapV2TxContext {
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_hash: hash_string(&transaction.hash),
            from_address: Some(address_string(&transaction.from_address)),
        };
        self.record_transaction_metadata(
            &tx_context.tx_hash,
            tx_context.from_address.as_deref(),
            tx_context.block_number,
            tx_context.block_timestamp,
        );
        let pool = self
            .uniswap_v3_pool_mut(&pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V3 pool {pool_address}"))?;
        pool.update_from_processed_transaction(transaction, &tx_context)?;
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn add_uniswap_v4_pool(&mut self, pool: UniswapV4Pool) -> Option<UniswapV4Pool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_key = pool.base.identity.pool_address.clone();
        let previous = self.v4_pools.insert(pool_key, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn uniswap_v4_pool(&self, pool_key: impl AsRef<str>) -> Option<&UniswapV4Pool> {
        self.v4_pools.get(&normalize_address(pool_key))
    }

    pub fn uniswap_v4_pool_mut(&mut self, pool_key: impl AsRef<str>) -> Option<&mut UniswapV4Pool> {
        self.v4_pools.get_mut(&normalize_address(pool_key))
    }

    pub fn update_uniswap_v4_pool_from_processed_transaction(
        &mut self,
        pool_key: impl AsRef<str>,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        let pool_key = normalize_address(pool_key);
        let tx_context = UniswapV2TxContext {
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_hash: hash_string(&transaction.hash),
            from_address: Some(address_string(&transaction.from_address)),
        };
        self.record_transaction_metadata(
            &tx_context.tx_hash,
            tx_context.from_address.as_deref(),
            tx_context.block_number,
            tx_context.block_timestamp,
        );
        let pool = self
            .uniswap_v4_pool_mut(&pool_key)
            .ok_or_else(|| eyre!("unknown Uniswap V4 pool {pool_key}"))?;
        pool.update_from_processed_transaction(transaction, &tx_context)?;
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn create_curve_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        name: Option<String>,
        lp_token_address: Option<String>,
        base_token_index: usize,
        quote_token_index: usize,
        tokens: Vec<CurvePoolToken>,
    ) -> &mut CurvePool {
        config.token_decimals = self.decimals;
        let pool = CurvePool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            name,
            lp_token_address,
            base_token_index,
            quote_token_index,
            tokens,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_curve_pool(pool);
        self.curve_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_curve_pool(&mut self, pool: CurvePool) -> Option<CurvePool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let previous = self.curve_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn create_balancer_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        pool_id: impl Into<String>,
        vault_address: impl Into<String>,
        swap_fee_bps: Option<u32>,
        tokens: Vec<BalancerPoolToken>,
    ) -> &mut BalancerPool {
        config.token_decimals = self.decimals;
        let pool = BalancerPool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            pool_id,
            vault_address,
            swap_fee_bps,
            tokens,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_balancer_pool(pool);
        self.balancer_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_balancer_pool(&mut self, pool: BalancerPool) -> Option<BalancerPool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let previous = self.balancer_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn update_token_state_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        self.record_transaction_metadata(
            hash_string(&transaction.hash),
            Some(&address_string(&transaction.from_address)),
            transaction.block_number,
            transaction.block_timestamp,
        );
        self.transfer_tracker
            .update_from_processed_transaction(transaction)?;
        self.status_manager.update_from_processed_transaction(
            transaction,
            self.transfer_tracker.total_supply_from_transfers,
            self.decimals,
        )?;
        let newly_added = self
            .authority_tracker
            .update_from_processed_transaction(transaction);
        if !newly_added.is_empty() {
            self.token_control_addresses.extend(newly_added.clone());
            self.register_control_addresses_with_pools(newly_added);
        }
        self.refresh_lifecycle_status();
        Ok(())
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

    pub fn record_transaction_metadata(
        &mut self,
        tx_hash: impl AsRef<str>,
        from_address: Option<&str>,
        block_number: u64,
        block_timestamp: u64,
    ) {
        if let Some(from_address) = from_address {
            self.tx_hashes_to_makers.insert(
                tx_hash.as_ref().to_string(),
                normalize_address(from_address),
            );
        }
        self.latest_block_number = Some(block_number);
        self.latest_block_timestamp = Some(block_timestamp);
    }

    pub fn pool_addresses(&self) -> Vec<String> {
        let mut addresses = self.uniswap_v2_pool_addresses();
        addresses.extend(self.uniswap_v3_pool_addresses());
        addresses.extend(self.uniswap_v4_pool_keys());
        addresses.extend(self.curve_pools.keys().cloned());
        addresses.extend(self.balancer_pools.keys().cloned());
        addresses.sort();
        addresses.dedup();
        addresses
    }

    pub fn uniswap_v2_pool_addresses(&self) -> Vec<String> {
        let mut addresses: Vec<_> = self.v2_pools.keys().cloned().collect();
        addresses.sort();
        addresses
    }

    pub fn uniswap_v3_pool_addresses(&self) -> Vec<String> {
        let mut addresses: Vec<_> = self.v3_pools.keys().cloned().collect();
        addresses.sort();
        addresses
    }

    pub fn uniswap_v4_pool_keys(&self) -> Vec<String> {
        let mut addresses: Vec<_> = self.v4_pools.keys().cloned().collect();
        addresses.sort();
        addresses
    }

    pub fn pool_count(&self) -> usize {
        self.v2_pools.len()
            + self.v3_pools.len()
            + self.v4_pools.len()
            + self.curve_pools.len()
            + self.balancer_pools.len()
    }

    pub fn has_pool(&self) -> bool {
        self.pool_count() > 0
    }

    pub fn trading_enabled(&self) -> bool {
        self.all_pool_bases()
            .iter()
            .any(|pool| pool.trading_enabled())
    }

    pub fn is_scam(&self) -> bool {
        self.status_manager.is_scam || self.all_pool_bases().iter().any(|pool| pool.is_scam())
    }

    pub fn scam_label(&self) -> Option<String> {
        if let Some(label) = self.status_manager.scam_label.clone() {
            return Some(label);
        }
        self.all_pool_bases()
            .into_iter()
            .find_map(|pool| pool.scam_label.clone())
    }

    pub fn current_prices(&self) -> HashMap<String, PoolStateSnapshot> {
        let mut prices = HashMap::new();
        for (address, pool) in &self.v2_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from(pool));
        }
        for (address, pool) in &self.v3_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from(pool));
        }
        for (address, pool) in &self.v4_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from(pool));
        }
        for (address, pool) in &self.curve_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from_base(&pool.base));
        }
        for (address, pool) in &self.balancer_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from_base(&pool.base));
        }
        prices
    }

    pub fn get_pool_info(&self) -> HashMap<String, PoolStateSnapshot> {
        self.current_prices()
    }

    pub fn total_liquidity_by_denom(&self) -> HashMap<String, f64> {
        let mut liquidity = HashMap::new();
        for pool in self.all_pool_bases() {
            *liquidity
                .entry(pool.identity.denom_address.clone())
                .or_insert(0.0) += pool.state.total_liquidity;
        }
        liquidity
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
            creation_block: self.creation_block,
            creator_address: self.creator_address.clone(),
            has_pools: self.has_pool(),
            pool_count: self.pool_count(),
            protocols,
            total_transactions: self.tx_hashes_to_makers.len(),
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
        })
    }

    fn refresh_lifecycle_status(&mut self) {
        if self.is_scam() {
            self.token_life_cycle_status = Some(TokenLifecycleState::InactiveScam);
        } else if self.trading_enabled() {
            self.token_life_cycle_status = Some(TokenLifecycleState::TradingEnabled);
        } else if self.has_pool() {
            self.token_life_cycle_status = Some(TokenLifecycleState::PairCreation);
        }
    }

    fn register_control_addresses_with_pools(
        &mut self,
        addresses: impl IntoIterator<Item = String>,
    ) {
        let addresses: Vec<_> = addresses.into_iter().collect();
        if addresses.is_empty() {
            return;
        }
        for pool in self.v2_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.v3_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.v4_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.curve_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.balancer_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
    }

    pub fn pool_base(&self, pool_key: impl AsRef<str>) -> Option<&BasePool> {
        let pool_key = normalize_address(pool_key);
        self.v2_pools
            .get(&pool_key)
            .map(|pool| &pool.base)
            .or_else(|| self.v3_pools.get(&pool_key).map(|pool| &pool.base))
            .or_else(|| self.v4_pools.get(&pool_key).map(|pool| &pool.base))
            .or_else(|| self.curve_pools.get(&pool_key).map(|pool| &pool.base))
            .or_else(|| self.balancer_pools.get(&pool_key).map(|pool| &pool.base))
    }

    pub fn all_pool_bases(&self) -> Vec<&BasePool> {
        let mut pools = Vec::with_capacity(self.pool_count());
        pools.extend(self.v2_pools.values().map(|pool| &pool.base));
        pools.extend(self.v3_pools.values().map(|pool| &pool.base));
        pools.extend(self.v4_pools.values().map(|pool| &pool.base));
        pools.extend(self.curve_pools.values().map(|pool| &pool.base));
        pools.extend(self.balancer_pools.values().map(|pool| &pool.base));
        pools
    }
}

fn v2_events_from_processed_transaction(
    transaction: &ProcessedTransaction,
    pool_address: &str,
) -> UniswapV2TransactionEvents {
    UniswapV2TransactionEvents {
        syncs: transaction
            .uniswap_v2_syncs
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2SyncEvent {
                pair_address: address_string(&event.pair_address),
                reserve0: event.reserve0.to_string(),
                reserve1: event.reserve1.to_string(),
            })
            .collect(),
        swaps: transaction
            .uniswap_v2_swaps
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2SwapEvent {
                pair_address: address_string(&event.pair_address),
                sender: Some(address_string(&event.sender)),
                to: Some(address_string(&event.to)),
                amount0_in: event.amount0_in.to_string(),
                amount1_in: event.amount1_in.to_string(),
                amount0_out: event.amount0_out.to_string(),
                amount1_out: event.amount1_out.to_string(),
            })
            .collect(),
        mints: transaction
            .uniswap_v2_mints
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2MintEvent {
                pair_address: address_string(&event.pair_address),
                to: Some(address_string(&event.sender)),
                amount: Some(event.amount0.to_string()),
            })
            .collect(),
        burns: transaction
            .uniswap_v2_burns
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2BurnEvent {
                pair_address: address_string(&event.pair_address),
                sender: Some(address_string(&event.sender)),
                amount0: event.amount0.to_string(),
                amount1: event.amount1.to_string(),
            })
            .collect(),
    }
}

fn lp_transfers_from_processed_transaction(
    transaction: &ProcessedTransaction,
    pool_address: &str,
) -> Vec<LPTransferEvent> {
    transaction
        .erc20_transfers
        .iter()
        .filter(|event| same_address(&event.token_address, pool_address))
        .map(|event| LPTransferEvent {
            from_address: address_string(&event.from_address),
            to_address: address_string(&event.to_address),
            amount: event.amount.to_string(),
            block_number: Some(transaction.block_number),
            tx_hash: Some(hash_string(&transaction.hash)),
            log_index: Some(event.log_index),
        })
        .collect()
}

fn lp_approvals_from_processed_transaction(
    transaction: &ProcessedTransaction,
    pool_address: &str,
) -> Vec<LPApprovalEvent> {
    transaction
        .erc20_approval_events
        .iter()
        .filter(|event| same_address(&event.token_address, pool_address))
        .map(|event| LPApprovalEvent {
            owner: address_string(&event.owner),
            spender: address_string(&event.spender),
            amount: event.amount.to_string(),
            block_number: Some(transaction.block_number),
            tx_hash: Some(hash_string(&transaction.hash)),
            block_timestamp: Some(transaction.block_timestamp),
        })
        .collect()
}

fn same_address(address: &Address, value: &str) -> bool {
    address_string(address) == normalize_address(value)
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn hash_string(hash: &alloy_primitives::B256) -> String {
    format!("{hash:#x}")
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: impl Into<String>) -> String {
    value.into().trim().to_ascii_lowercase()
}

fn parse_address_lossy(value: &str) -> Address {
    value.parse().unwrap_or(Address::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pools::uniswap::v2::{
        UniswapV2SyncEvent, UniswapV2TransactionEvents, UNISWAP_V2_PROTOCOL,
    };
    use crate::pools::{
        BalancerPoolToken, CurvePoolToken, BALANCER_V2_PROTOCOL, CURVE_V1_PROTOCOL,
        SUSHISWAP_V2_PROTOCOL,
    };

    fn token() -> ERC20Token {
        ERC20Token::new(ERC20TokenMetadata::new(
            "0x0000000000000000000000000000000000000001",
            "Token",
            "TKN",
            18,
            "1000000000000000000000",
        ))
    }

    #[test]
    fn token_live_mode_defaults_false_and_can_be_enabled() {
        let metadata = ERC20TokenMetadata::new(
            "0x0000000000000000000000000000000000000001",
            "Token",
            "TKN",
            18,
            "1000000000000000000000",
        );

        assert!(!ERC20Token::new(metadata.clone()).is_live_mode);
        assert!(ERC20Token::with_live_mode(metadata, true).is_live_mode);
    }

    #[test]
    fn token_owns_v2_pools_and_reports_summary() {
        let mut token = token();
        token.create_uniswap_v2_pool(
            "0x0000000000000000000000000000000000000002",
            "0x0000000000000000000000000000000000000003",
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
            std::iter::empty::<&str>(),
        );

        assert!(token.has_pool());
        assert_eq!(
            token.token_life_cycle_status,
            Some(TokenLifecycleState::PairCreation)
        );
        assert_eq!(token.pool_addresses().len(), 1);
        assert_eq!(
            token.get_token_summary().protocols,
            vec![UNISWAP_V2_PROTOCOL]
        );
    }

    #[test]
    fn token_reports_non_uniswap_pool_protocols() {
        let mut token = token();
        token.create_sushiswap_v2_pool(
            "0x0000000000000000000000000000000000000002",
            "0x0000000000000000000000000000000000000003",
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
            std::iter::empty::<&str>(),
        );
        token.create_curve_pool(
            "0x0000000000000000000000000000000000000004",
            "0x0000000000000000000000000000000000000003",
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
            Some("curve-test".to_string()),
            Some("0x0000000000000000000000000000000000000005".to_string()),
            0,
            1,
            vec![CurvePoolToken {
                symbol: Some("TKN".to_string()),
                address: token.contract_address.clone(),
                decimals: token.decimals,
                index: 0,
            }],
        );
        token.create_balancer_pool(
            "0x0000000000000000000000000000000000000006",
            "0x0000000000000000000000000000000000000003",
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
            "0x1111111111111111111111111111111111111111111111111111111111111111",
            "0x0000000000000000000000000000000000000007",
            Some(30),
            vec![BalancerPoolToken {
                symbol: Some("TKN".to_string()),
                address: token.contract_address.clone(),
                decimals: token.decimals,
                index: 0,
                weight: Some("80".to_string()),
            }],
        );

        let summary = token.get_token_summary();

        assert_eq!(summary.pool_count, 3);
        assert!(summary
            .protocols
            .contains(&SUSHISWAP_V2_PROTOCOL.to_string()));
        assert!(summary.protocols.contains(&CURVE_V1_PROTOCOL.to_string()));
        assert!(summary
            .protocols
            .contains(&BALANCER_V2_PROTOCOL.to_string()));
    }

    #[test]
    fn token_trading_enabled_is_pool_derived() {
        let mut token = token();
        token.status_manager.trading_enabled = true;
        token.status_manager.trading_enabled_block = Some(100);
        token.refresh_lifecycle_status();

        assert!(!token.trading_enabled());
        assert_eq!(token.trading_enabled_block(), None);
        assert_eq!(token.trading_enabled_tx(), None);
        assert_ne!(
            token.token_life_cycle_status,
            Some(TokenLifecycleState::TradingEnabled)
        );

        {
            let pool = token.create_uniswap_v2_pool(
                "0x0000000000000000000000000000000000000002",
                "0x0000000000000000000000000000000000000003",
                BasePoolConfig {
                    denom_decimals: Some(18),
                    token1_is_denom: Some(true),
                    ..BasePoolConfig::new(18)
                },
                std::iter::empty::<&str>(),
            );
            pool.base.mark_can_buy_from_event(200, "0xBUY", 1_700);
        }
        token.refresh_lifecycle_status();

        assert!(token.trading_enabled());
        assert_eq!(token.trading_enabled_block(), Some(200));
        assert_eq!(token.trading_enabled_tx(), Some("0xBUY".to_string()));
        assert_eq!(
            token.token_life_cycle_status,
            Some(TokenLifecycleState::TradingEnabled)
        );
    }

    #[test]
    fn updates_v2_pool_from_event_batch() {
        let mut token = token();
        let pool_address = "0x0000000000000000000000000000000000000002";
        token.create_uniswap_v2_pool(
            pool_address,
            "0x0000000000000000000000000000000000000003",
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
            std::iter::empty::<&str>(),
        );

        let tx = UniswapV2TxContext {
            block_number: 10,
            block_timestamp: 1_700,
            tx_hash: "0xTX".to_string(),
            from_address: Some("0xMAKER".to_string()),
        };
        let events = UniswapV2TransactionEvents {
            syncs: vec![UniswapV2SyncEvent {
                pair_address: pool_address.to_string(),
                reserve0: "100000000000000000000".to_string(),
                reserve1: "2000000000000000000".to_string(),
            }],
            ..Default::default()
        };

        token
            .update_uniswap_v2_pool_events(pool_address, &events, &tx)
            .unwrap();

        let pool = token.uniswap_v2_pool(pool_address).unwrap();
        assert_eq!(pool.base.token_reserve(), 100.0);
        assert_eq!(pool.base.denom_reserve(), 2.0);
        assert_eq!(token.latest_block_number, Some(10));
    }
}
