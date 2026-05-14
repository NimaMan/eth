mod events;

use std::collections::{HashMap, HashSet};

use eyre::{eyre, Result};
use reth_chain_query::common_addresses::{KnownV2Protocol, DENOM_ADDRESSES};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_processor::ProcessedTransaction;

use crate::pools::balancer::{BalancerPool, BalancerPoolToken};
use crate::pools::base::{BasePool, BasePoolConfig};
use crate::pools::curve::{CurvePool, CurvePoolToken};
use crate::pools::sushiswap::new_sushiswap_v2_pool;
use crate::pools::uniswap::v2::{UniswapV2Pool, UniswapV2TransactionEvents, UniswapV2TxContext};
use crate::pools::uniswap::{v4_event_display_key, UniswapV3Pool, UniswapV4Pool};
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
            activity: TokenActivityTracker::default(),
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

    pub fn create_known_v2_pool(
        &mut self,
        protocol: KnownV2Protocol,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut UniswapV2Pool {
        config.token_decimals = self.decimals;
        let pool = UniswapV2Pool::new_with_protocol(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            protocol.label(),
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
        self.record_v2_swap_activity(&pool_address, events, tx)?;
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
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_v2_swap_activity(&pool_address, &events, &tx_context)?;
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
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_v3_swap_activity(&pool_address, transaction, &tx_context)?;
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
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_v4_swap_activity(&pool_key, transaction, &tx_context)?;
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
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_transfer_activity_from_processed_transaction(transaction);
        self.transfer_tracker
            .update_from_processed_transaction(transaction)?;
        self.record_pair_token_transfers_from_processed_transaction(transaction)?;
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
        self.activity.record_transaction(
            tx_hash.as_ref(),
            from_address,
            block_number,
            Some(block_timestamp),
        );
        if let Some(from_address) = from_address {
            self.tx_hashes_to_makers.insert(
                tx_hash.as_ref().to_string(),
                normalize_address(from_address),
            );
        }
        self.latest_block_number = Some(block_number);
        self.latest_block_timestamp = Some(block_timestamp);
    }

    fn record_bribe_activity_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        if transaction.bribe_amount.is_zero() {
            return Ok(());
        }

        let amount_eth = scale_raw_units(transaction.bribe_amount.to_string(), 18)?;
        self.activity.record_bribe_eth(
            hash_string(&transaction.hash),
            transaction.block_number,
            Some(transaction.block_timestamp),
            amount_eth,
        );
        Ok(())
    }

    fn record_transfer_activity_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) {
        let mut token_transfer_count = 0u32;
        let mut denom_transfer_count = 0u32;
        let token_address = normalize_address_string(&self.contract_address);

        for transfer in &transaction.erc20_transfers {
            if same_address(&transfer.token_address, &token_address) {
                token_transfer_count = token_transfer_count.saturating_add(1);
            } else if DENOM_ADDRESSES.contains_key(&transfer.token_address) {
                denom_transfer_count = denom_transfer_count.saturating_add(1);
            }
        }

        if token_transfer_count > 0 {
            self.activity.record_token_transfer(
                hash_string(&transaction.hash),
                transaction.block_number,
                Some(transaction.block_timestamp),
                token_transfer_count,
            );
        }
        if denom_transfer_count > 0 {
            self.activity.record_denom_transfer(
                hash_string(&transaction.hash),
                transaction.block_number,
                Some(transaction.block_timestamp),
                denom_transfer_count,
            );
        }
    }

    fn record_pair_token_transfers_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        if self.v2_pools.is_empty() {
            return Ok(());
        }

        let token_address = normalize_address_string(&self.contract_address);
        let pool_addresses: Vec<String> = self.v2_pools.keys().cloned().collect();
        let tx_hash = hash_string(&transaction.hash);
        let mut records = Vec::new();

        for transfer in &transaction.erc20_transfers {
            if !same_address(&transfer.token_address, &token_address) {
                continue;
            }

            let from_address = address_string(&transfer.from_address);
            let to_address = address_string(&transfer.to_address);
            let from_normalized = normalize_address(&from_address);
            let to_normalized = normalize_address(&to_address);
            let touched_pool = pool_addresses.iter().find(|pool_address| {
                from_normalized == **pool_address || to_normalized == **pool_address
            });
            let Some(pool_address) = touched_pool else {
                continue;
            };

            let amount = scale_raw_units(transfer.amount.to_string(), self.decimals)?;
            records.push((
                pool_address.clone(),
                from_address,
                to_address,
                amount,
                transfer.log_index,
                transaction_has_v2_pool_event(transaction, pool_address),
            ));
        }

        for (pool_address, from_address, to_address, amount, log_index, has_pool_event) in records {
            if let Some(pool) = self.v2_pools.get_mut(&pool_address) {
                pool.base.record_pair_token_transfer(
                    from_address,
                    to_address,
                    amount,
                    transaction.block_number,
                    transaction.block_timestamp,
                    tx_hash.clone(),
                    Some(log_index),
                    has_pool_event,
                );
            }
        }

        Ok(())
    }

    fn record_v2_swap_activity(
        &mut self,
        pool_address: &str,
        events: &UniswapV2TransactionEvents,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool = self
            .uniswap_v2_pool(pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V2 pool {pool_address}"))?;
        let denom_address = pool.base.identity.denom_address.clone();
        let denom_decimals = pool.base.config.denom_decimals.unwrap_or(18);
        let token1_is_denom = pool.base.config.token1_is_denom.unwrap_or(false);

        for swap in &events.swaps {
            if !same_address(&parse_address_lossy(pool_address), &swap.pair_address) {
                continue;
            }

            let denom_in = if token1_is_denom {
                scale_raw_units(&swap.amount1_in, denom_decimals)?
            } else {
                scale_raw_units(&swap.amount0_in, denom_decimals)?
            };
            let denom_out = if token1_is_denom {
                scale_raw_units(&swap.amount1_out, denom_decimals)?
            } else {
                scale_raw_units(&swap.amount0_out, denom_decimals)?
            };
            self.record_swap_activity(
                &tx.tx_hash,
                tx.block_number,
                tx.block_timestamp,
                &denom_address,
                denom_in,
                denom_out,
            );
        }

        Ok(())
    }

    fn record_v3_swap_activity(
        &mut self,
        pool_address: &str,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool = self
            .uniswap_v3_pool(pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V3 pool {pool_address}"))?;
        let denom_address = pool.base.identity.denom_address.clone();
        let denom_decimals = pool.base.config.denom_decimals.unwrap_or(18);
        let token1_is_denom = pool.base.config.token1_is_denom.unwrap_or(false);

        for event in &transaction.uniswap_v3_swaps {
            if !same_address(&event.pool_address, pool_address) {
                continue;
            }
            let (denom_in, denom_out) = signed_denom_swap_amounts(
                event.amount0,
                event.amount1,
                self.decimals,
                denom_decimals,
                token1_is_denom,
            );
            self.record_swap_activity(
                &tx.tx_hash,
                tx.block_number,
                tx.block_timestamp,
                &denom_address,
                denom_in,
                denom_out,
            );
        }

        Ok(())
    }

    fn record_v4_swap_activity(
        &mut self,
        pool_key: &str,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool = self
            .uniswap_v4_pool(pool_key)
            .ok_or_else(|| eyre!("unknown Uniswap V4 pool {pool_key}"))?;
        let denom_address = pool.base.identity.denom_address.clone();
        let denom_decimals = pool.base.config.denom_decimals.unwrap_or(18);
        let token1_is_denom = pool.base.config.token1_is_denom.unwrap_or(false);

        for event in &transaction.uniswap_v4_swaps {
            if normalize_address(v4_event_display_key(
                event.pool_manager_address,
                event.event_id,
            )) != normalize_address(pool_key)
            {
                continue;
            }
            let (denom_in, denom_out) = signed_denom_swap_amounts(
                event.amount0,
                event.amount1,
                self.decimals,
                denom_decimals,
                token1_is_denom,
            );
            self.record_swap_activity(
                &tx.tx_hash,
                tx.block_number,
                tx.block_timestamp,
                &denom_address,
                denom_in,
                denom_out,
            );
        }

        Ok(())
    }

    fn record_swap_activity(
        &mut self,
        tx_hash: &str,
        block_number: u64,
        block_timestamp: u64,
        denom_address: &str,
        denom_in: f64,
        denom_out: f64,
    ) {
        if denom_in > 0.0 {
            self.activity.record_buy(
                tx_hash,
                block_number,
                Some(block_timestamp),
                denom_address,
                denom_in,
            );
        }
        if denom_out > 0.0 {
            self.activity.record_sell(
                tx_hash,
                block_number,
                Some(block_timestamp),
                denom_address,
                denom_out,
            );
        }
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
        self.hidden_mint_detected()
            || self.all_pool_bases().iter().any(|pool| {
                pool.has_liquidity_removal() || pool.inferred_scam_mechanism().is_some()
            })
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

fn transaction_has_v2_pool_event(transaction: &ProcessedTransaction, pool_address: &str) -> bool {
    transaction
        .uniswap_v2_syncs
        .iter()
        .any(|event| same_address(&event.pair_address, pool_address))
        || transaction
            .uniswap_v2_swaps
            .iter()
            .any(|event| same_address(&event.pair_address, pool_address))
        || transaction
            .uniswap_v2_mints
            .iter()
            .any(|event| same_address(&event.pair_address, pool_address))
        || transaction
            .uniswap_v2_burns
            .iter()
            .any(|event| same_address(&event.pair_address, pool_address))
}

#[cfg(test)]
mod tests;
