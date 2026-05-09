use std::collections::{BTreeMap, BTreeSet};

use alloy_primitives::{Address, U256};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_processor::tx_processor::data_models::{
    ERC721TransferEvent, UniswapV3BurnEvent as ProcessedV3BurnEvent,
    UniswapV3DecreaseLiquidityEvent as ProcessedV3DecreaseLiquidityEvent,
    UniswapV3IncreaseLiquidityEvent as ProcessedV3IncreaseLiquidityEvent,
    UniswapV3InitializeEvent as ProcessedV3InitializeEvent,
    UniswapV3MintEvent as ProcessedV3MintEvent, UniswapV3SwapEvent as ProcessedV3SwapEvent,
};
use tx_processor::ProcessedTransaction;

use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};
use crate::utils::append_with_history_limit;

use super::concentrated::{
    current_tick_in_range, scale_i128, token_price_from_sqrt_price_x96, u256_to_u128_saturating,
    update_tick_delta, virtual_reserves_from_liquidity, VirtualReserves,
};
use super::v2::{LPHolderSnapshot, UniswapV2TxContext};

pub const UNISWAP_V3_PROTOCOL: &str = "UNISWAP-V3";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3Pool {
    pub base: BasePool,
    pub token0: String,
    pub token1: String,
    pub fee_tier: u32,
    pub tick_spacing: i32,
    pub current_tick: Option<i32>,
    pub sqrt_price_x96: Option<String>,
    pub active_liquidity: u128,
    pub tick_liquidity_net: BTreeMap<i32, i128>,
    pub position_manager_address: Option<String>,
    pub liquidity_positions: BTreeMap<String, UniswapV3LiquidityPosition>,
    pub initialize_events: Vec<Value>,
    pub ignored_position_liquidity_events: Vec<Value>,
    pub liquidity_position_events: Vec<Value>,
    pub last_virtual_reserves: Option<ConcentratedVirtualReserveSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3LiquidityPosition {
    pub position_id: String,
    pub owner: String,
    pub position_manager_address: Option<String>,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub last_update_block: u64,
    pub last_update_tx: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConcentratedVirtualReserveSnapshot {
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub token0_reserve: f64,
    pub token1_reserve: f64,
}

impl From<VirtualReserves> for ConcentratedVirtualReserveSnapshot {
    fn from(reserves: VirtualReserves) -> Self {
        Self {
            token_reserve: reserves.token_reserve,
            denom_reserve: reserves.denom_reserve,
            token0_reserve: reserves.token0_reserve,
            token1_reserve: reserves.token1_reserve,
        }
    }
}

impl UniswapV3Pool {
    pub fn new(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        token0: impl Into<String>,
        token1: impl Into<String>,
        fee_tier: u32,
        tick_spacing: i32,
        config: BasePoolConfig,
    ) -> Self {
        let token_address = token_address.into();
        let denom_address = denom_address.into();
        let identity = PoolIdentity::new(
            pool_address,
            token_address,
            denom_address,
            UNISWAP_V3_PROTOCOL,
        );
        Self {
            base: BasePool::new(identity, config),
            token0: normalize_address_string(token0),
            token1: normalize_address_string(token1),
            fee_tier,
            tick_spacing,
            current_tick: None,
            sqrt_price_x96: None,
            active_liquidity: 0,
            tick_liquidity_net: BTreeMap::new(),
            position_manager_address: None,
            liquidity_positions: BTreeMap::new(),
            initialize_events: Vec::new(),
            ignored_position_liquidity_events: Vec::new(),
            liquidity_position_events: Vec::new(),
            last_virtual_reserves: None,
        }
    }

    pub fn update_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let mut events = Vec::new();

        for event in &transaction.uniswap_v3_initializations {
            if same_address(&event.pool_address, &self.base.identity.pool_address) {
                events.push(V3PoolAction::Initialize(event));
            }
        }
        for event in &transaction.uniswap_v3_mints {
            if same_address(&event.pool_address, &self.base.identity.pool_address) {
                events.push(V3PoolAction::Mint(event));
            }
        }
        for event in &transaction.uniswap_v3_burns {
            if same_address(&event.pool_address, &self.base.identity.pool_address) {
                events.push(V3PoolAction::Burn(event));
            }
        }
        for event in &transaction.uniswap_v3_swaps {
            if same_address(&event.pool_address, &self.base.identity.pool_address) {
                events.push(V3PoolAction::Swap(event));
            }
        }

        events.sort_by_key(V3PoolAction::log_index);
        let mut updated_position_ids = BTreeSet::new();
        for event in events {
            match event {
                V3PoolAction::Initialize(event) => self.process_initialize(event, tx),
                V3PoolAction::Mint(event) => {
                    let position = self.process_mint(event, tx, transaction);
                    updated_position_ids.insert(position.position_id);
                }
                V3PoolAction::Burn(event) => {
                    let position = self.process_burn(event, tx, transaction);
                    updated_position_ids.insert(position.position_id);
                }
                V3PoolAction::Swap(event) => self.process_swap(event, tx),
            }
        }
        self.process_position_transfers(transaction, tx, &updated_position_ids);

        if !transaction.uniswap_v3_increases.is_empty()
            || !transaction.uniswap_v3_decreases.is_empty()
            || !transaction.uniswap_v3_positions.is_empty()
        {
            append_with_history_limit(
                &mut self.ignored_position_liquidity_events,
                json!({
                    "block_number": tx.block_number,
                    "tx_hash": tx.tx_hash,
                    "reason": "v3 position-manager liquidity events are reconciled with core Mint/Burn events when they match this pool",
                    "increase_count": transaction.uniswap_v3_increases.len(),
                    "decrease_count": transaction.uniswap_v3_decreases.len(),
                    "position_count": transaction.uniswap_v3_positions.len(),
                }),
                self.base.config.history_limit,
            );
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
        self.base
            .config
            .denom_decimals
            .unwrap_or(self.base.config.token_decimals)
    }

    pub fn lp_total_supply(&self) -> f64 {
        self.liquidity_positions
            .values()
            .map(|position| position.liquidity as f64)
            .sum()
    }

    pub fn lp_holders(&self) -> Vec<LPHolderSnapshot> {
        let mut balances = BTreeMap::<String, f64>::new();
        for position in self.liquidity_positions.values() {
            if position.liquidity == 0 {
                continue;
            }
            *balances.entry(position.owner.clone()).or_insert(0.0) += position.liquidity as f64;
        }

        let total: f64 = balances.values().sum();
        let mut holders = balances
            .into_iter()
            .map(|(address, balance)| LPHolderSnapshot {
                address,
                balance,
                share: if total > 0.0 {
                    (balance / total) * 100.0
                } else {
                    0.0
                },
                approvals: Default::default(),
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

    pub fn touches_position_transfer(&self, transaction: &ProcessedTransaction) -> bool {
        transaction
            .erc721_transfers
            .iter()
            .any(|transfer| self.position_transfer_matches_known_position(transfer))
    }

    fn process_initialize(&mut self, event: &ProcessedV3InitializeEvent, tx: &UniswapV2TxContext) {
        self.current_tick = Some(event.tick);
        self.sqrt_price_x96 = Some(event.sqrt_price_x96.to_string());
        append_with_history_limit(
            &mut self.initialize_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
    }

    fn process_mint(
        &mut self,
        event: &ProcessedV3MintEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) -> UniswapV3LiquidityPosition {
        let amount = u256_to_u128_saturating(event.amount);
        update_tick_delta(
            &mut self.tick_liquidity_net,
            event.tick_lower,
            i128::try_from(amount).unwrap_or(i128::MAX),
        );
        update_tick_delta(
            &mut self.tick_liquidity_net,
            event.tick_upper,
            -i128::try_from(amount).unwrap_or(i128::MAX),
        );
        if current_tick_in_range(self.current_tick, event.tick_lower, event.tick_upper) {
            self.active_liquidity = self.active_liquidity.saturating_add(amount);
        }
        self.base.state.total_mints += 1;
        append_with_history_limit(
            &mut self.base.mint_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
        let position = self.record_mint_position(event, tx, transaction);
        self.refresh_virtual_reserves(tx);
        position
    }

    fn process_burn(
        &mut self,
        event: &ProcessedV3BurnEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) -> UniswapV3LiquidityPosition {
        let amount = u256_to_u128_saturating(event.amount);
        let signed = i128::try_from(amount).unwrap_or(i128::MAX);
        update_tick_delta(&mut self.tick_liquidity_net, event.tick_lower, -signed);
        update_tick_delta(&mut self.tick_liquidity_net, event.tick_upper, signed);
        if current_tick_in_range(self.current_tick, event.tick_lower, event.tick_upper) {
            self.active_liquidity = self.active_liquidity.saturating_sub(amount);
        }
        self.base.state.total_burns += 1;
        append_with_history_limit(
            &mut self.base.burn_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
        let position = self.record_burn_position(event, tx, transaction);
        self.refresh_virtual_reserves(tx);
        position
    }

    fn record_mint_position(
        &mut self,
        event: &ProcessedV3MintEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) -> UniswapV3LiquidityPosition {
        let matched = position_increase_for_mint_event(event, transaction);
        let position_id = matched
            .map(|increase| position_id_from_token_id(increase.token_id))
            .unwrap_or_else(|| direct_position_id(event.owner, event.tick_lower, event.tick_upper));
        let transfer = matched.and_then(|increase| {
            position_transfer_for_position_event(
                increase.token_id,
                increase.pool_address,
                transaction,
            )
        });
        let previous = self.liquidity_positions.get(&position_id);
        let position_manager_address =
            matched.map(|increase| address_string(&increase.pool_address));
        if self.position_manager_address.is_none() {
            self.position_manager_address = position_manager_address.clone();
        }
        let owner = transfer
            .and_then(owner_from_position_transfer)
            .or_else(|| previous.map(|position| position.owner.clone()))
            .or_else(|| {
                let owner = address_string(&event.owner);
                if owner == position_manager_address.clone().unwrap_or_default() {
                    tx.from_address.clone()
                } else {
                    Some(owner)
                }
            })
            .unwrap_or_else(|| address_string(&event.owner));
        let previous_liquidity = previous.map(|position| position.liquidity).unwrap_or(0);
        let liquidity = previous_liquidity.saturating_add(u256_to_u128_saturating(event.amount));
        let position = UniswapV3LiquidityPosition {
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
        self.record_position_event("position_increase", &position, tx);
        position
    }

    fn record_burn_position(
        &mut self,
        event: &ProcessedV3BurnEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) -> UniswapV3LiquidityPosition {
        let matched = position_decrease_for_burn_event(event, transaction);
        let position_id = matched
            .map(|decrease| position_id_from_token_id(decrease.token_id))
            .unwrap_or_else(|| direct_position_id(event.owner, event.tick_lower, event.tick_upper));
        let previous = self.liquidity_positions.get(&position_id);
        let position_manager_address = matched
            .map(|decrease| address_string(&decrease.pool_address))
            .or_else(|| previous.and_then(|position| position.position_manager_address.clone()));
        if self.position_manager_address.is_none() {
            self.position_manager_address = position_manager_address.clone();
        }
        let owner = previous
            .map(|position| position.owner.clone())
            .unwrap_or_else(|| address_string(&event.owner));
        let previous_liquidity = previous.map(|position| position.liquidity).unwrap_or(0);
        let liquidity = previous_liquidity.saturating_sub(u256_to_u128_saturating(event.amount));
        let position = UniswapV3LiquidityPosition {
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
        self.record_position_event("position_decrease", &position, tx);
        position
    }

    fn process_position_transfers(
        &mut self,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
        updated_position_ids: &BTreeSet<String>,
    ) {
        let transfers = transaction
            .erc721_transfers
            .iter()
            .filter_map(|transfer| {
                let position_id = self.position_transfer_id(transfer)?;
                if updated_position_ids.contains(&position_id) {
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

    fn record_position_event(
        &mut self,
        event_name: &str,
        position: &UniswapV3LiquidityPosition,
        tx: &UniswapV2TxContext,
    ) {
        append_with_history_limit(
            &mut self.liquidity_position_events,
            json!({
                "event": event_name,
                "block_number": tx.block_number,
                "block_timestamp": tx.block_timestamp,
                "tx_hash": tx.tx_hash,
                "liquidity_provider": position.owner,
                "position_id": position.position_id,
                "position_manager_address": position.position_manager_address,
                "position_liquidity": position.liquidity.to_string(),
                "tick_lower": position.tick_lower,
                "tick_upper": position.tick_upper,
            }),
            self.base.config.history_limit,
        );
    }

    fn position_transfer_matches_known_position(&self, transfer: &ERC721TransferEvent) -> bool {
        self.position_transfer_id(transfer).is_some()
    }

    fn position_transfer_id(&self, transfer: &ERC721TransferEvent) -> Option<String> {
        let position_manager = self.position_manager_address.as_ref()?;
        if !same_address(&transfer.token_address, position_manager) {
            return None;
        }
        let position_id = position_id_from_token_id(transfer.token_id);
        self.liquidity_positions
            .contains_key(&position_id)
            .then_some(position_id)
    }

    fn process_swap(&mut self, event: &ProcessedV3SwapEvent, tx: &UniswapV2TxContext) {
        self.current_tick = Some(event.tick);
        self.sqrt_price_x96 = Some(event.sqrt_price_x96.to_string());
        self.active_liquidity = event.liquidity;

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
}

enum V3PoolAction<'a> {
    Initialize(&'a ProcessedV3InitializeEvent),
    Mint(&'a ProcessedV3MintEvent),
    Burn(&'a ProcessedV3BurnEvent),
    Swap(&'a ProcessedV3SwapEvent),
}

impl V3PoolAction<'_> {
    fn log_index(&self) -> u64 {
        match self {
            Self::Initialize(event) => event.log_index,
            Self::Mint(event) => event.log_index,
            Self::Burn(event) => event.log_index,
            Self::Swap(event) => event.log_index,
        }
    }
}

fn token0_decimals(pool: &UniswapV3Pool) -> u8 {
    if pool.base.config.token1_is_denom.unwrap_or(false) {
        pool.base.config.token_decimals
    } else {
        pool.denom_decimals()
    }
}

fn token1_decimals(pool: &UniswapV3Pool) -> u8 {
    if pool.base.config.token1_is_denom.unwrap_or(false) {
        pool.denom_decimals()
    } else {
        pool.base.config.token_decimals
    }
}

fn event_json<T: Serialize>(event: &T, tx: &UniswapV2TxContext) -> Value {
    let mut value = serde_json::to_value(event).unwrap_or_else(|_| json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert("block_number".to_string(), json!(tx.block_number));
        object.insert("block_timestamp".to_string(), json!(tx.block_timestamp));
        object.insert("tx_hash".to_string(), json!(tx.tx_hash));
    }
    value
}

fn position_increase_for_mint_event<'a>(
    event: &ProcessedV3MintEvent,
    transaction: &'a ProcessedTransaction,
) -> Option<&'a ProcessedV3IncreaseLiquidityEvent> {
    transaction
        .uniswap_v3_increases
        .iter()
        .filter(|increase| increase.liquidity == event.amount)
        .filter(|increase| increase.amount0 == event.amount0 && increase.amount1 == event.amount1)
        .min_by_key(|increase| increase.log_index.abs_diff(event.log_index))
}

fn position_decrease_for_burn_event<'a>(
    event: &ProcessedV3BurnEvent,
    transaction: &'a ProcessedTransaction,
) -> Option<&'a ProcessedV3DecreaseLiquidityEvent> {
    transaction
        .uniswap_v3_decreases
        .iter()
        .filter(|decrease| decrease.liquidity == event.amount)
        .filter(|decrease| decrease.amount0 == event.amount0 && decrease.amount1 == event.amount1)
        .min_by_key(|decrease| decrease.log_index.abs_diff(event.log_index))
}

fn position_transfer_for_position_event(
    token_id: U256,
    position_manager: Address,
    transaction: &ProcessedTransaction,
) -> Option<&ERC721TransferEvent> {
    transaction
        .erc721_transfers
        .iter()
        .filter(|transfer| transfer.token_address == position_manager)
        .filter(|transfer| transfer.token_id == token_id)
        .min_by_key(|transfer| transfer.log_index)
}

fn owner_from_position_transfer(transfer: &ERC721TransferEvent) -> Option<String> {
    if !transfer.to_address.is_zero() {
        Some(address_string(&transfer.to_address))
    } else if !transfer.from_address.is_zero() {
        Some(address_string(&transfer.from_address))
    } else {
        None
    }
}

fn position_id_from_token_id(token_id: U256) -> String {
    format!("{token_id:#x}")
}

fn direct_position_id(owner: Address, tick_lower: i32, tick_upper: i32) -> String {
    format!(
        "direct:{}:{tick_lower}:{tick_upper}",
        address_string(&owner)
    )
}

fn same_address(address: &Address, value: &str) -> bool {
    address_string(address) == normalize_address(value)
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256, Address, U256};
    use tx_processor::ProcessedTransaction;

    use super::*;

    fn pool() -> UniswapV3Pool {
        UniswapV3Pool::new(
            "0x0000000000000000000000000000000000000003",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            3000,
            60,
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
        )
    }

    fn tx() -> (ProcessedTransaction, UniswapV2TxContext) {
        (
            ProcessedTransaction::new(
                b256!("0000000000000000000000000000000000000000000000000000000000000001"),
                100,
                1_700,
                1,
                address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                None,
                U256::ZERO,
                true,
                0,
                0,
                Vec::new(),
            ),
            UniswapV2TxContext::new(100, 1_700, "0xTX"),
        )
    }

    #[test]
    fn swap_sets_post_swap_price_tick_and_liquidity() {
        let mut pool = pool();
        let (mut processed, ctx) = tx();
        let sqrt = U256::from(1u128) << 96;
        processed
            .uniswap_v3_initializations
            .push(ProcessedV3InitializeEvent {
                pool_address: address!("0000000000000000000000000000000000000003"),
                sqrt_price_x96: sqrt,
                tick: 0,
                log_index: 1,
            });
        processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
            pool_address: address!("0000000000000000000000000000000000000003"),
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            owner: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            tick_lower: -60,
            tick_upper: 60,
            amount: U256::from(1_000_000_000_000_000_000u128),
            amount0: U256::ZERO,
            amount1: U256::ZERO,
            log_index: 2,
        });
        processed.uniswap_v3_swaps.push(ProcessedV3SwapEvent {
            pool_address: address!("0000000000000000000000000000000000000003"),
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            recipient: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            amount0: -100,
            amount1: 100,
            sqrt_price_x96: sqrt,
            liquidity: 1_000_000_000_000_000_000u128,
            tick: 0,
            log_index: 3,
        });

        pool.update_from_processed_transaction(&processed, &ctx)
            .unwrap();

        assert_eq!(pool.current_tick, Some(0));
        assert_eq!(pool.active_liquidity, 1_000_000_000_000_000_000u128);
        assert_eq!(pool.base.price(), 1.0);
        assert!(!pool.base.state.can_buy);
        assert_eq!(pool.base.state.total_swaps, 1);
        assert_eq!(pool.base.swap_events[0]["is_buy"], true);
        assert_eq!(pool.base.swap_events[0]["is_sell"], false);
    }

    #[test]
    fn mint_tracks_position_nft_owner_as_lp_holder() {
        let mut pool = pool();
        let (mut processed, ctx) = tx();
        let position_manager = address!("c36442b4a4522e871399cd717abdd847ab11fe88");
        let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
        let token_id = U256::from(42);
        processed.erc721_transfers.push(ERC721TransferEvent {
            token_address: position_manager,
            from_address: Address::ZERO,
            to_address: owner,
            token_id,
            log_index: 1,
        });
        processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
            pool_address: address!("0000000000000000000000000000000000000003"),
            sender: position_manager,
            owner: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            amount: U256::from(100),
            amount0: U256::from(1),
            amount1: U256::from(2),
            log_index: 2,
        });
        processed
            .uniswap_v3_increases
            .push(ProcessedV3IncreaseLiquidityEvent {
                token_id,
                liquidity: U256::from(100),
                amount0: U256::from(1),
                amount1: U256::from(2),
                pool_address: position_manager,
                log_index: 3,
            });

        pool.update_from_processed_transaction(&processed, &ctx)
            .unwrap();

        let holders = pool.lp_holders();
        assert_eq!(holders.len(), 1);
        assert_eq!(
            holders[0].address,
            "0x0d82a9f1ae5b693c9b00c8e874057fb78824cfd3"
        );
        assert_eq!(holders[0].balance, 100.0);
        assert_eq!(
            pool.position_manager_address.as_deref(),
            Some("0xc36442b4a4522e871399cd717abdd847ab11fe88")
        );
        assert_eq!(
            pool.liquidity_position_events[0]["liquidity_provider"],
            "0x0d82a9f1ae5b693c9b00c8e874057fb78824cfd3"
        );
    }

    #[test]
    fn burn_decreases_existing_position_owner_liquidity() {
        let mut pool = pool();
        let (mut processed, ctx) = tx();
        let position_manager = address!("c36442b4a4522e871399cd717abdd847ab11fe88");
        let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
        let token_id = U256::from(42);
        processed.erc721_transfers.push(ERC721TransferEvent {
            token_address: position_manager,
            from_address: Address::ZERO,
            to_address: owner,
            token_id,
            log_index: 1,
        });
        processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
            pool_address: address!("0000000000000000000000000000000000000003"),
            sender: position_manager,
            owner: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            amount: U256::from(100),
            amount0: U256::from(1),
            amount1: U256::from(2),
            log_index: 2,
        });
        processed
            .uniswap_v3_increases
            .push(ProcessedV3IncreaseLiquidityEvent {
                token_id,
                liquidity: U256::from(100),
                amount0: U256::from(1),
                amount1: U256::from(2),
                pool_address: position_manager,
                log_index: 3,
            });
        pool.update_from_processed_transaction(&processed, &ctx)
            .unwrap();

        let (mut processed, ctx) = tx();
        processed.uniswap_v3_burns.push(ProcessedV3BurnEvent {
            pool_address: address!("0000000000000000000000000000000000000003"),
            owner: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            amount: U256::from(25),
            amount0: U256::from(1),
            amount1: U256::from(2),
            log_index: 2,
        });
        processed
            .uniswap_v3_decreases
            .push(ProcessedV3DecreaseLiquidityEvent {
                token_id,
                liquidity: U256::from(25),
                amount0: U256::from(1),
                amount1: U256::from(2),
                pool_address: position_manager,
                log_index: 1,
            });
        pool.update_from_processed_transaction(&processed, &ctx)
            .unwrap();

        let holders = pool.lp_holders();
        assert_eq!(holders.len(), 1);
        assert_eq!(
            holders[0].address,
            "0x0d82a9f1ae5b693c9b00c8e874057fb78824cfd3"
        );
        assert_eq!(holders[0].balance, 75.0);
    }

    #[test]
    fn position_transfer_updates_existing_lp_holder() {
        let mut pool = pool();
        let (mut processed, ctx) = tx();
        let position_manager = address!("c36442b4a4522e871399cd717abdd847ab11fe88");
        let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
        let next_owner = address!("7ca2d5fa2c6b3e01294a74e353c89141837ad784");
        let token_id = U256::from(42);
        processed.erc721_transfers.push(ERC721TransferEvent {
            token_address: position_manager,
            from_address: Address::ZERO,
            to_address: owner,
            token_id,
            log_index: 1,
        });
        processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
            pool_address: address!("0000000000000000000000000000000000000003"),
            sender: position_manager,
            owner: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            amount: U256::from(100),
            amount0: U256::from(1),
            amount1: U256::from(2),
            log_index: 2,
        });
        processed
            .uniswap_v3_increases
            .push(ProcessedV3IncreaseLiquidityEvent {
                token_id,
                liquidity: U256::from(100),
                amount0: U256::from(1),
                amount1: U256::from(2),
                pool_address: position_manager,
                log_index: 3,
            });
        pool.update_from_processed_transaction(&processed, &ctx)
            .unwrap();

        let (mut transfer_tx, ctx) = tx();
        transfer_tx.erc721_transfers.push(ERC721TransferEvent {
            token_address: position_manager,
            from_address: owner,
            to_address: next_owner,
            token_id,
            log_index: 1,
        });

        assert!(pool.touches_position_transfer(&transfer_tx));
        pool.update_from_processed_transaction(&transfer_tx, &ctx)
            .unwrap();

        let holders = pool.lp_holders();
        assert_eq!(holders.len(), 1);
        assert_eq!(
            holders[0].address,
            "0x7ca2d5fa2c6b3e01294a74e353c89141837ad784"
        );
        assert_eq!(holders[0].balance, 100.0);
    }
}
