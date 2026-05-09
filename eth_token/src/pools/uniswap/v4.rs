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

mod helpers;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4LiquidityPosition {
    pub position_id: String,
    pub owner: String,
    pub position_manager_address: String,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub last_update_block: u64,
    pub last_update_tx: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4PositionApproval {
    pub position_id: String,
    pub owner: String,
    pub spender: String,
    pub block_number: u64,
    pub tx_hash: String,
    pub block_timestamp: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4OperatorApproval {
    pub owner: String,
    pub operator: String,
    pub approved: bool,
    pub block_number: u64,
    pub tx_hash: String,
    pub block_timestamp: Option<u64>,
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

    pub fn price_from_sqrt_price(&self) -> Option<f64> {
        token_price_from_sqrt_price_x96(
            self.parsed_sqrt_price_x96()?,
            self.base.config.token_decimals,
            self.denom_decimals(),
            self.base.config.token1_is_denom.unwrap_or(false),
        )
    }

    pub fn virtual_reserves(&self) -> Option<VirtualReserves> {
        virtual_reserves_from_liquidity(
            self.active_liquidity,
            self.parsed_sqrt_price_x96()?,
            self.base.config.token_decimals,
            self.denom_decimals(),
            self.base.config.token1_is_denom.unwrap_or(false),
        )
    }

    pub fn denom_decimals(&self) -> u8 {
        self.base.config.denom_decimals.unwrap_or(18)
    }

    pub fn lp_total_supply(&self) -> f64 {
        self.liquidity_positions
            .values()
            .map(|position| position.liquidity as f64)
            .sum()
    }

    pub fn lp_holders(&self) -> Vec<LPHolderSnapshot> {
        let balances = self.lp_balances_by_holder();
        let approvals_by_holder = self.lp_approvals_by_holder(&balances);
        let total: f64 = balances.values().sum();
        let mut holders = balances
            .into_iter()
            .filter(|(_, balance)| *balance > 0.0)
            .map(|(address, balance)| LPHolderSnapshot {
                approvals: approvals_by_holder
                    .get(&address)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
                address,
                balance,
                share: if total > 0.0 {
                    (balance / total) * 100.0
                } else {
                    0.0
                },
            })
            .collect::<Vec<_>>();
        holders.sort_by(|left, right| {
            right
                .balance
                .partial_cmp(&left.balance)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.address.cmp(&right.address))
        });
        holders
    }

    pub fn total_approved_to_routers(&self) -> f64 {
        self.lp_holders()
            .into_iter()
            .map(|holder| {
                holder
                    .approvals
                    .values()
                    .filter(|approval| approval.is_router)
                    .map(|approval| approval.amount.min(holder.balance))
                    .sum::<f64>()
            })
            .sum()
    }

    pub fn lp_approved_percentage(&self) -> f64 {
        let total = self.lp_total_supply();
        if total == 0.0 {
            0.0
        } else {
            (self.total_approved_to_routers() / total) * 100.0
        }
    }

    pub fn last_lp_approval_block(&self) -> Option<u64> {
        self.lp_approval_events
            .last()
            .and_then(|event| event.get("block_number"))
            .and_then(Value::as_u64)
    }

    pub fn last_lp_approval_event(&self) -> Option<Value> {
        self.lp_approval_events.last().cloned()
    }

    pub fn holders_with_approvals(&self) -> Vec<String> {
        self.lp_holders()
            .into_iter()
            .filter(|holder| !holder.approvals.is_empty())
            .map(|holder| holder.address)
            .collect()
    }

    pub fn touches_position_transfer(&self, transaction: &ProcessedTransaction) -> bool {
        transaction
            .erc721_transfers
            .iter()
            .any(|transfer| self.position_transfer_matches_known_position(transfer))
    }

    pub fn touches_position_approval(&self, transaction: &ProcessedTransaction) -> bool {
        transaction
            .erc721_approval_events
            .iter()
            .any(|approval| self.position_approval_matches_known_position(approval))
            || transaction
                .approval_for_all_events
                .iter()
                .any(|approval| self.operator_approval_matches_known_owner(approval))
    }

    pub fn build_simulator_pool_config(
        &self,
    ) -> Result<tx_processor::simulator::types::UniswapV4PoolConfig> {
        Ok(tx_processor::simulator::types::UniswapV4PoolConfig {
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

    fn process_initialize(&mut self, event: &ProcessedV4InitializeEvent, tx: &UniswapV2TxContext) {
        self.pool_key = UniswapV4PoolKey {
            currency0: address_string(&event.currency0),
            currency1: address_string(&event.currency1),
            fee: event.fee,
            tick_spacing: event.tick_spacing,
            hooks: address_string(&event.hooks),
        }
        .normalized();
        self.current_tick = Some(event.tick);
        self.sqrt_price_x96 = Some(event.sqrt_price_x96.to_string());
        append_with_history_limit(
            &mut self.initialize_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
    }

    fn process_modify_liquidity(
        &mut self,
        event: &ProcessedV4ModifyLiquidityEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) {
        update_tick_delta(
            &mut self.tick_liquidity_net,
            event.tick_lower,
            event.liquidity_delta,
        );
        update_tick_delta(
            &mut self.tick_liquidity_net,
            event.tick_upper,
            -event.liquidity_delta,
        );
        if current_tick_in_range(self.current_tick, event.tick_lower, event.tick_upper) {
            self.apply_active_liquidity_delta(event.liquidity_delta);
        }
        if event.liquidity_delta >= 0 {
            self.base.state.total_mints += 1;
        } else {
            self.base.state.total_burns += 1;
        }
        let position_update = self.record_liquidity_position(event, tx, transaction);
        let mut modify_event = event_json(event, tx);
        if let Some(object) = modify_event.as_object_mut() {
            object.insert(
                "liquidity_provider".to_string(),
                json!(position_update.owner.clone()),
            );
            object.insert(
                "position_id".to_string(),
                json!(position_update.position_id.clone()),
            );
            object.insert(
                "position_manager_address".to_string(),
                json!(position_update.position_manager_address.clone()),
            );
            object.insert(
                "position_liquidity".to_string(),
                json!(position_update.liquidity.to_string()),
            );
        }
        append_with_history_limit(
            &mut self.modify_liquidity_events,
            modify_event.clone(),
            self.base.config.history_limit,
        );
        append_with_history_limit(
            &mut self.liquidity_position_events,
            modify_event,
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
    }

    fn record_liquidity_position(
        &mut self,
        event: &ProcessedV4ModifyLiquidityEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) -> UniswapV4LiquidityPosition {
        let position_id = normalize_hash_string(hash_string(&event.salt));
        let transfer = position_transfer_for_modify_event(event, transaction);
        let previous = self.liquidity_positions.get(&position_id);
        let previous_owner = previous.map(|position| position.owner.clone());
        let position_manager_address = transfer
            .map(|transfer| address_string(&transfer.token_address))
            .unwrap_or_else(|| address_string(&event.sender));
        self.position_manager_address = Some(position_manager_address.clone());
        let owner = transfer
            .and_then(owner_from_position_transfer)
            .or_else(|| previous.map(|position| position.owner.clone()))
            .or_else(|| tx.from_address.clone())
            .unwrap_or_else(|| address_string(&event.sender));
        let previous_liquidity = previous.map(|position| position.liquidity).unwrap_or(0);
        let liquidity = apply_liquidity_delta(previous_liquidity, event.liquidity_delta);
        let position = UniswapV4LiquidityPosition {
            position_id: position_id.clone(),
            owner,
            position_manager_address,
            liquidity,
            tick_lower: event.tick_lower,
            tick_upper: event.tick_upper,
            last_update_block: tx.block_number,
            last_update_tx: tx.tx_hash.clone(),
        };
        self.liquidity_positions
            .insert(position_id.clone(), position.clone());
        if previous_owner
            .as_deref()
            .is_some_and(|previous_owner| previous_owner != position.owner)
        {
            self.position_approvals.remove(&position_id);
        }
        position
    }

    fn process_position_transfers(
        &mut self,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
        modified_position_ids: &BTreeSet<String>,
    ) {
        let transfers = transaction
            .erc721_transfers
            .iter()
            .filter_map(|transfer| {
                let position_id = self.position_transfer_id(transfer)?;
                if modified_position_ids.contains(&position_id) {
                    return None;
                }
                Some((position_id, transfer))
            })
            .collect::<Vec<_>>();

        for (position_id, transfer) in transfers {
            let Some(owner) = owner_from_position_transfer(transfer) else {
                continue;
            };
            let Some(position) = self.liquidity_positions.get_mut(&position_id) else {
                continue;
            };
            position.owner = owner.clone();
            position.last_update_block = tx.block_number;
            position.last_update_tx = tx.tx_hash.clone();
            self.position_approvals.remove(&position_id);

            let transfer_event = json!({
                "event": "position_transfer",
                "block_number": tx.block_number,
                "block_timestamp": tx.block_timestamp,
                "tx_hash": tx.tx_hash,
                "position_id": position_id,
                "position_manager_address": address_string(&transfer.token_address),
                "from": address_string(&transfer.from_address),
                "to": owner,
                "position_liquidity": position.liquidity.to_string(),
            });
            append_with_history_limit(
                &mut self.liquidity_position_events,
                transfer_event,
                self.base.config.history_limit,
            );
        }
    }

    fn process_position_approvals(
        &mut self,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) {
        let mut events = Vec::new();
        for event in &transaction.erc721_approval_events {
            if self.position_approval_matches_known_position(event) {
                events.push(V4ApprovalAction::Position(event));
            }
        }
        for event in &transaction.approval_for_all_events {
            if self.operator_approval_matches_known_owner(event) {
                events.push(V4ApprovalAction::Operator(event));
            }
        }

        events.sort_by_key(V4ApprovalAction::log_index);
        for event in events {
            match event {
                V4ApprovalAction::Position(event) => self.process_position_approval(event, tx),
                V4ApprovalAction::Operator(event) => self.process_operator_approval(event, tx),
            }
        }
    }

    fn process_position_approval(&mut self, event: &ERC721ApprovalEvent, tx: &UniswapV2TxContext) {
        let Some(position_id) = self.position_id_for_token_id(event.token_id) else {
            return;
        };
        let owner = address_string(&event.owner);
        let spender = address_string(&event.approved_address);
        let amount = self
            .liquidity_positions
            .get(&position_id)
            .map(|position| position.liquidity as f64)
            .unwrap_or(0.0);

        if event.approved_address.is_zero() {
            self.position_approvals.remove(&position_id);
        } else {
            self.position_approvals.insert(
                position_id.clone(),
                UniswapV4PositionApproval {
                    position_id: position_id.clone(),
                    owner: owner.clone(),
                    spender: spender.clone(),
                    block_number: tx.block_number,
                    tx_hash: tx.tx_hash.clone(),
                    block_timestamp: Some(tx.block_timestamp),
                },
            );
        }

        append_with_history_limit(
            &mut self.lp_approval_events,
            json!({
                "approval_type": "erc721_position",
                "block_number": tx.block_number,
                "block_timestamp": tx.block_timestamp,
                "tx_hash": tx.tx_hash,
                "position_id": position_id,
                "owner": owner,
                "spender": spender,
                "amount": amount,
                "approved": !event.approved_address.is_zero(),
                "is_router": self.known_routers.contains(&address_string(&event.approved_address)),
                "log_index": event.log_index,
            }),
            self.base.config.history_limit,
        );
    }

    fn process_operator_approval(&mut self, event: &ApprovalForAllEvent, tx: &UniswapV2TxContext) {
        let owner = address_string(&event.owner);
        let operator = address_string(&event.operator);
        let amount = self
            .lp_balances_by_holder()
            .get(&owner)
            .copied()
            .unwrap_or(0.0);

        if event.approved {
            self.operator_approvals
                .entry(owner.clone())
                .or_default()
                .insert(
                    operator.clone(),
                    UniswapV4OperatorApproval {
                        owner: owner.clone(),
                        operator: operator.clone(),
                        approved: true,
                        block_number: tx.block_number,
                        tx_hash: tx.tx_hash.clone(),
                        block_timestamp: Some(tx.block_timestamp),
                    },
                );
        } else if let Some(approvals) = self.operator_approvals.get_mut(&owner) {
            approvals.remove(&operator);
            if approvals.is_empty() {
                self.operator_approvals.remove(&owner);
            }
        }

        append_with_history_limit(
            &mut self.lp_approval_events,
            json!({
                "approval_type": "erc721_approval_for_all",
                "block_number": tx.block_number,
                "block_timestamp": tx.block_timestamp,
                "tx_hash": tx.tx_hash,
                "owner": owner,
                "spender": operator,
                "operator": address_string(&event.operator),
                "amount": amount,
                "approved": event.approved,
                "is_router": self.known_routers.contains(&address_string(&event.operator)),
                "log_index": event.log_index,
            }),
            self.base.config.history_limit,
        );
    }

    fn position_transfer_matches_known_position(&self, transfer: &ERC721TransferEvent) -> bool {
        self.position_transfer_id(transfer).is_some()
    }

    fn position_approval_matches_known_position(&self, approval: &ERC721ApprovalEvent) -> bool {
        let Some(position_manager) = self.position_manager_address.as_ref() else {
            return false;
        };
        same_address(&approval.token_address, position_manager)
            && self.position_id_for_token_id(approval.token_id).is_some()
    }

    fn operator_approval_matches_known_owner(&self, approval: &ApprovalForAllEvent) -> bool {
        let Some(position_manager) = self.position_manager_address.as_ref() else {
            return false;
        };
        let owner = address_string(&approval.owner);
        same_address(&approval.token_address, position_manager)
            && (self.operator_approvals.contains_key(&owner)
                || self
                    .liquidity_positions
                    .values()
                    .any(|position| position.owner == owner))
    }

    fn position_transfer_id(&self, transfer: &ERC721TransferEvent) -> Option<String> {
        let position_manager = self.position_manager_address.as_ref()?;
        if !same_address(&transfer.token_address, position_manager) {
            return None;
        }
        self.position_id_for_token_id(transfer.token_id)
    }

    fn position_id_for_token_id(&self, token_id: U256) -> Option<String> {
        self.liquidity_positions
            .keys()
            .find(|position_id| {
                parse_hash(position_id)
                    .map(position_token_id)
                    .is_ok_and(|position_token_id| position_token_id == token_id)
            })
            .cloned()
    }

    fn lp_balances_by_holder(&self) -> BTreeMap<String, f64> {
        let mut balances = BTreeMap::<String, f64>::new();
        for position in self.liquidity_positions.values() {
            if position.liquidity == 0 {
                continue;
            }
            *balances.entry(position.owner.clone()).or_insert(0.0) += position.liquidity as f64;
        }
        balances
    }

    fn lp_approvals_by_holder(
        &self,
        balances: &BTreeMap<String, f64>,
    ) -> BTreeMap<String, BTreeMap<String, LPApprovalSnapshot>> {
        let mut approvals = BTreeMap::<String, BTreeMap<String, LPApprovalSnapshot>>::new();
        for position in self.liquidity_positions.values() {
            if position.liquidity == 0 {
                continue;
            }
            let Some(approval) = self.position_approvals.get(&position.position_id) else {
                continue;
            };
            if approval.owner != position.owner || approval.spender == V4_NATIVE_ETH_ADDRESS {
                continue;
            }
            add_lp_approval_amount(
                &mut approvals,
                &self.known_routers,
                &position.owner,
                &approval.spender,
                position.liquidity as f64,
                approval.block_number,
                &approval.tx_hash,
            );
        }

        for (owner, operators) in &self.operator_approvals {
            let Some(balance) = balances
                .get(owner)
                .copied()
                .filter(|balance| *balance > 0.0)
            else {
                continue;
            };
            for approval in operators.values().filter(|approval| approval.approved) {
                set_lp_approval_amount_at_least(
                    &mut approvals,
                    &self.known_routers,
                    owner,
                    &approval.operator,
                    balance,
                    approval.block_number,
                    &approval.tx_hash,
                );
            }
        }
        approvals
    }

    fn process_swap(&mut self, event: &ProcessedV4SwapEvent, tx: &UniswapV2TxContext) {
        self.current_tick = Some(event.tick);
        self.sqrt_price_x96 = Some(event.sqrt_price_x96.to_string());
        self.active_liquidity = event.liquidity;
        self.last_swap_fee = Some(event.fee);

        let token0_amount = scale_i128(event.amount0, token0_decimals(self));
        let token1_amount = scale_i128(event.amount1, token1_decimals(self));
        let (token_amount, denom_amount) =
            self.base.map_token_and_denom(token0_amount, token1_amount);
        self.base.state.record_swap(
            denom_amount.max(0.0),
            token_amount.max(0.0),
            (-denom_amount).max(0.0),
            (-token_amount).max(0.0),
        );
        let mut swap_event = event_json(event, tx);
        if let Some(object) = swap_event.as_object_mut() {
            object.insert("token_amount".to_string(), json!(token_amount));
            object.insert("denom_amount".to_string(), json!(denom_amount));
            object.insert(
                "is_buy".to_string(),
                json!(token_amount < 0.0 && denom_amount > 0.0),
            );
            object.insert(
                "is_sell".to_string(),
                json!(token_amount > 0.0 && denom_amount < 0.0),
            );
        }
        append_with_history_limit(
            &mut self.base.swap_events,
            swap_event,
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
    }

    fn process_donate(&mut self, event: &ProcessedV4DonateEvent, tx: &UniswapV2TxContext) {
        append_with_history_limit(
            &mut self.donate_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
    }

    fn process_protocol_fee_update(
        &mut self,
        event: &ProcessedV4FeeUpdatedEvent,
        tx: &UniswapV2TxContext,
    ) {
        self.protocol_fee = Some(event.protocol_fee);
        append_with_history_limit(
            &mut self.fee_update_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
    }

    fn process_dynamic_fee_update(
        &mut self,
        event: &ProcessedV4DynamicLPFeeUpdatedEvent,
        tx: &UniswapV2TxContext,
    ) {
        self.dynamic_lp_fee = Some(event.dynamic_lp_fee);
        append_with_history_limit(
            &mut self.fee_update_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
    }

    fn apply_active_liquidity_delta(&mut self, delta: i128) {
        if delta >= 0 {
            self.active_liquidity = self.active_liquidity.saturating_add(delta as u128);
        } else {
            self.active_liquidity = self.active_liquidity.saturating_sub(delta.unsigned_abs());
        }
    }

    fn refresh_virtual_reserves(&mut self, tx: &UniswapV2TxContext) {
        let Some(reserves) = self.virtual_reserves() else {
            return;
        };
        self.last_virtual_reserves = Some(reserves.into());
        self.base.update_reserves(
            reserves.token_reserve,
            reserves.denom_reserve,
            tx.block_number,
            tx.block_timestamp,
            tx.tx_hash.clone(),
        );
    }

    fn parsed_sqrt_price_x96(&self) -> Option<U256> {
        let value = self.sqrt_price_x96.as_deref()?;
        U256::from_str_radix(value.trim_start_matches("0x"), 10).ok()
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

enum V4PoolAction<'a> {
    Initialize(&'a ProcessedV4InitializeEvent),
    ModifyLiquidity(&'a ProcessedV4ModifyLiquidityEvent),
    Swap(&'a ProcessedV4SwapEvent),
    Donate(&'a ProcessedV4DonateEvent),
    ProtocolFeeUpdate(&'a ProcessedV4FeeUpdatedEvent),
    DynamicFeeUpdate(&'a ProcessedV4DynamicLPFeeUpdatedEvent),
}

impl V4PoolAction<'_> {
    fn log_index(&self) -> u64 {
        match self {
            Self::Initialize(event) => event.log_index,
            Self::ModifyLiquidity(event) => event.log_index,
            Self::Swap(event) => event.log_index,
            Self::Donate(event) => event.log_index,
            Self::ProtocolFeeUpdate(event) => event.log_index,
            Self::DynamicFeeUpdate(event) => event.log_index,
        }
    }
}

enum V4ApprovalAction<'a> {
    Position(&'a ERC721ApprovalEvent),
    Operator(&'a ApprovalForAllEvent),
}

impl V4ApprovalAction<'_> {
    fn log_index(&self) -> u64 {
        match self {
            Self::Position(event) => event.log_index,
            Self::Operator(event) => event.log_index,
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
