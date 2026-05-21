use std::collections::{BTreeMap, BTreeSet};

use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_processor::tx_processor::data_models::{
    ApprovalForAllEvent, ERC721ApprovalEvent, ERC721TransferEvent,
    UniswapV4DonateEvent as ProcessedV4DonateEvent,
    UniswapV4DynamicLPFeeUpdatedEvent as ProcessedV4DynamicLPFeeUpdatedEvent,
    UniswapV4FeeUpdatedEvent as ProcessedV4FeeUpdatedEvent,
    UniswapV4InitializeEvent as ProcessedV4InitializeEvent,
    UniswapV4ModifyLiquidityEvent as ProcessedV4ModifyLiquidityEvent,
    UniswapV4SwapEvent as ProcessedV4SwapEvent,
};
use tx_processor::ProcessedTransaction;

use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};
use crate::utils::append_with_history_limit;

use super::concentrated::{
    current_tick_in_range, scale_i128, token_price_from_sqrt_price_x96, update_tick_delta,
    virtual_reserves_from_liquidity, VirtualReserves,
};
use super::v2::{LPApprovalSnapshot, LPHolderSnapshot, UniswapV2TxContext};
use super::v3::ConcentratedVirtualReserveSnapshot;

mod approvals;
mod events;
mod helpers;
mod liquidity;
mod positions;
mod swaps;

pub use self::approvals::{UniswapV4OperatorApproval, UniswapV4PositionApproval};
use self::events::V4PoolAction;
pub use self::positions::UniswapV4LiquidityPosition;

use self::helpers::{
    add_lp_approval_amount, address_string, apply_liquidity_delta, event_json, hash_string,
    normalize_address, normalize_address_string, normalize_hash_string,
    owner_from_position_transfer, parse_address, parse_hash, position_token_id,
    position_transfer_for_modify_event, same_address, set_lp_approval_amount_at_least,
    token0_decimals, token1_decimals,
};

pub const UNISWAP_V4_PROTOCOL: &str = "UNISWAP-V4";
pub const V4_NATIVE_ETH_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4PoolKey {
    pub currency0: String,
    pub currency1: String,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4Pool {
    pub base: BasePool,
    pub pool_manager_address: String,
    pub position_manager_address: Option<String>,
    pub pool_id: String,
    pub pool_key: UniswapV4PoolKey,
    pub current_tick: Option<i32>,
    pub sqrt_price_x96: Option<String>,
    pub active_liquidity: u128,
    pub tick_liquidity_net: BTreeMap<i32, i128>,
    #[serde(default)]
    pub known_routers: BTreeSet<String>,
    pub liquidity_positions: BTreeMap<String, UniswapV4LiquidityPosition>,
    #[serde(default)]
    pub position_approvals: BTreeMap<String, UniswapV4PositionApproval>,
    #[serde(default)]
    pub operator_approvals: BTreeMap<String, BTreeMap<String, UniswapV4OperatorApproval>>,
    pub protocol_fee: Option<u32>,
    pub dynamic_lp_fee: Option<u32>,
    pub last_swap_fee: Option<u32>,
    pub protocol_fee_controller: Option<String>,
    pub initialize_events: Vec<Value>,
    pub modify_liquidity_events: Vec<Value>,
    pub liquidity_position_events: Vec<Value>,
    #[serde(default)]
    pub lp_approval_events: Vec<Value>,
    pub donate_events: Vec<Value>,
    pub fee_update_events: Vec<Value>,
    pub last_virtual_reserves: Option<ConcentratedVirtualReserveSnapshot>,
}
impl UniswapV4Pool {
    pub fn new(
        pool_manager_address: impl Into<String>,
        pool_id: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        pool_key: UniswapV4PoolKey,
        config: BasePoolConfig,
    ) -> Self {
        let pool_manager_address = normalize_address_string(pool_manager_address);
        let pool_id = normalize_hash_string(pool_id.into());
        let display_key = v4_pool_display_key(&pool_manager_address, &pool_id);
        let token_address = token_address.into();
        let denom_address = denom_address.into();
        let identity = PoolIdentity::new(
            display_key,
            token_address,
            denom_address,
            UNISWAP_V4_PROTOCOL,
        );
        Self {
            base: BasePool::new(identity, config),
            pool_manager_address,
            position_manager_address: None,
            pool_id,
            pool_key: pool_key.normalized(),
            current_tick: None,
            sqrt_price_x96: None,
            active_liquidity: 0,
            tick_liquidity_net: BTreeMap::new(),
            known_routers: BTreeSet::new(),
            liquidity_positions: BTreeMap::new(),
            position_approvals: BTreeMap::new(),
            operator_approvals: BTreeMap::new(),
            protocol_fee: None,
            dynamic_lp_fee: None,
            last_swap_fee: None,
            protocol_fee_controller: None,
            initialize_events: Vec::new(),
            modify_liquidity_events: Vec::new(),
            liquidity_position_events: Vec::new(),
            lp_approval_events: Vec::new(),
            donate_events: Vec::new(),
            fee_update_events: Vec::new(),
            last_virtual_reserves: None,
        }
    }

    pub fn from_initialize_event(
        event: &ProcessedV4InitializeEvent,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        config: BasePoolConfig,
    ) -> Self {
        let pool_key = UniswapV4PoolKey {
            currency0: address_string(&event.currency0),
            currency1: address_string(&event.currency1),
            fee: event.fee,
            tick_spacing: event.tick_spacing,
            hooks: address_string(&event.hooks),
        };
        let mut pool = Self::new(
            address_string(&event.pool_manager_address),
            hash_string(&event.event_id),
            token_address,
            denom_address,
            pool_key,
            config,
        );
        pool.current_tick = Some(event.tick);
        pool.sqrt_price_x96 = Some(event.sqrt_price_x96.to_string());
        pool
    }

    pub fn set_known_routers(&mut self, routers: impl IntoIterator<Item = impl AsRef<str>>) {
        self.known_routers = routers.into_iter().map(normalize_address).collect();
    }

    pub fn update_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let mut events = Vec::new();
        for event in &transaction.uniswap_v4_initializes {
            if self.matches_event(event.pool_manager_address, event.event_id) {
                events.push(V4PoolAction::Initialize(event));
            }
        }
        for event in &transaction.uniswap_v4_modifies {
            if self.matches_event(event.pool_manager_address, event.event_id) {
                events.push(V4PoolAction::ModifyLiquidity(event));
            }
        }
        for event in &transaction.uniswap_v4_swaps {
            if self.matches_event(event.pool_manager_address, event.event_id) {
                events.push(V4PoolAction::Swap(event));
            }
        }
        for event in &transaction.uniswap_v4_donates {
            if self.matches_event(event.pool_manager_address, event.event_id) {
                events.push(V4PoolAction::Donate(event));
            }
        }
        for event in &transaction.uniswap_v4_protocol_fee_updates {
            if self.matches_event(event.pool_manager_address, event.event_id) {
                events.push(V4PoolAction::ProtocolFeeUpdate(event));
            }
        }
        for event in &transaction.uniswap_v4_dynamic_lp_fee_updates {
            if self.matches_event(event.pool_manager_address, event.event_id) {
                events.push(V4PoolAction::DynamicFeeUpdate(event));
            }
        }

        events.sort_by_key(V4PoolAction::log_index);
        let mut modified_position_ids = BTreeSet::new();
        for event in events {
            match event {
                V4PoolAction::Initialize(event) => self.process_initialize(event, tx),
                V4PoolAction::ModifyLiquidity(event) => {
                    modified_position_ids.insert(normalize_hash_string(hash_string(&event.salt)));
                    self.process_modify_liquidity(event, tx, transaction)
                }
                V4PoolAction::Swap(event) => self.process_swap(event, tx),
                V4PoolAction::Donate(event) => self.process_donate(event, tx),
                V4PoolAction::ProtocolFeeUpdate(event) => {
                    self.process_protocol_fee_update(event, tx)
                }
                V4PoolAction::DynamicFeeUpdate(event) => self.process_dynamic_fee_update(event, tx),
            }
        }
        self.process_position_transfers(transaction, tx, &modified_position_ids);
        self.process_position_approvals(transaction, tx);

        for event in &transaction.uniswap_v4_protocol_fee_controller_updates {
            if same_address(&event.pool_manager_address, &self.pool_manager_address) {
                self.protocol_fee_controller = Some(address_string(&event.protocol_fee_controller));
                append_with_history_limit(
                    &mut self.fee_update_events,
                    event_json(event, tx),
                    self.base.config.history_limit,
                );
            }
        }

        Ok(())
    }

    pub fn build_simulator_pool_config(
        &self,
    ) -> Result<tx_processor::trade_simulation::types::UniswapV4PoolConfig> {
        Ok(tx_processor::trade_simulation::types::UniswapV4PoolConfig {
            pool_manager: parse_address(&self.pool_manager_address)?,
            pool_id: parse_hash(&self.pool_id)?,
            currency0: parse_address(&self.pool_key.currency0)?,
            currency1: parse_address(&self.pool_key.currency1)?,
            fee: self.pool_key.fee,
            tick_spacing: self.pool_key.tick_spacing,
            hooks: parse_address(&self.pool_key.hooks)?,
            hook_data: Vec::new(),
        })
    }

    fn matches_event(&self, pool_manager: Address, pool_id: B256) -> bool {
        same_address(&pool_manager, &self.pool_manager_address)
            && normalize_hash_string(hash_string(&pool_id)) == self.pool_id
    }
}

impl UniswapV4PoolKey {
    pub fn normalized(self) -> Self {
        Self {
            currency0: normalize_address_string(self.currency0),
            currency1: normalize_address_string(self.currency1),
            fee: self.fee,
            tick_spacing: self.tick_spacing,
            hooks: normalize_address_string(self.hooks),
        }
    }
}

pub fn v4_pool_display_key(pool_manager: impl AsRef<str>, pool_id: impl AsRef<str>) -> String {
    format!(
        "{}#{}",
        normalize_address(pool_manager.as_ref()),
        normalize_hash_string(pool_id.as_ref())
    )
}

pub fn v4_event_display_key(pool_manager: Address, pool_id: B256) -> String {
    v4_pool_display_key(address_string(&pool_manager), hash_string(&pool_id))
}

pub fn display_denom_for_v4_currency(currency: Address) -> String {
    if currency.is_zero() {
        WETH_ADDRESS.to_string()
    } else {
        address_string(&currency)
    }
}

pub fn is_native_eth_currency(address: &str) -> bool {
    normalize_address(address) == V4_NATIVE_ETH_ADDRESS
}

#[cfg(test)]
mod tests;
