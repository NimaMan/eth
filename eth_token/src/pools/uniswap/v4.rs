use std::collections::BTreeMap;

use alloy_primitives::{Address, B256, U256};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_processor::tx_processor::data_models::{
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
use super::v2::UniswapV2TxContext;
use super::v3::ConcentratedVirtualReserveSnapshot;

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
    pub pool_id: String,
    pub pool_key: UniswapV4PoolKey,
    pub current_tick: Option<i32>,
    pub sqrt_price_x96: Option<String>,
    pub active_liquidity: u128,
    pub tick_liquidity_net: BTreeMap<i32, i128>,
    pub protocol_fee: Option<u32>,
    pub dynamic_lp_fee: Option<u32>,
    pub last_swap_fee: Option<u32>,
    pub protocol_fee_controller: Option<String>,
    pub initialize_events: Vec<Value>,
    pub modify_liquidity_events: Vec<Value>,
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
            pool_id,
            pool_key: pool_key.normalized(),
            current_tick: None,
            sqrt_price_x96: None,
            active_liquidity: 0,
            tick_liquidity_net: BTreeMap::new(),
            protocol_fee: None,
            dynamic_lp_fee: None,
            last_swap_fee: None,
            protocol_fee_controller: None,
            initialize_events: Vec::new(),
            modify_liquidity_events: Vec::new(),
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
        for event in events {
            match event {
                V4PoolAction::Initialize(event) => self.process_initialize(event, tx),
                V4PoolAction::ModifyLiquidity(event) => self.process_modify_liquidity(event, tx),
                V4PoolAction::Swap(event) => self.process_swap(event, tx),
                V4PoolAction::Donate(event) => self.process_donate(event, tx),
                V4PoolAction::ProtocolFeeUpdate(event) => {
                    self.process_protocol_fee_update(event, tx)
                }
                V4PoolAction::DynamicFeeUpdate(event) => self.process_dynamic_fee_update(event, tx),
            }
        }

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
        append_with_history_limit(
            &mut self.modify_liquidity_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
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
        append_with_history_limit(
            &mut self.base.swap_events,
            event_json(event, tx),
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

fn token0_decimals(pool: &UniswapV4Pool) -> u8 {
    if pool.base.config.token1_is_denom.unwrap_or(false) {
        pool.base.config.token_decimals
    } else {
        pool.denom_decimals()
    }
}

fn token1_decimals(pool: &UniswapV4Pool) -> u8 {
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

fn parse_address(value: &str) -> Result<Address> {
    value
        .parse()
        .map_err(|err| eyre!("invalid address {value}: {err}"))
}

fn parse_hash(value: &str) -> Result<B256> {
    value
        .parse()
        .map_err(|err| eyre!("invalid pool id {value}: {err}"))
}

fn same_address(address: &Address, value: &str) -> bool {
    address_string(address) == normalize_address(value)
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

fn normalize_hash_string(value: impl AsRef<str>) -> String {
    let value = value.as_ref().trim().to_ascii_lowercase();
    if value.starts_with("0x") {
        value
    } else {
        format!("0x{value}")
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256, U256};
    use tx_processor::ProcessedTransaction;

    use super::*;

    fn initialize_event() -> ProcessedV4InitializeEvent {
        ProcessedV4InitializeEvent {
            pool_manager_address: address!("000000000004444c5dc75cb358380d2e3de08a90"),
            event_id: b256!("1111111111111111111111111111111111111111111111111111111111111111"),
            currency0: address!("0000000000000000000000000000000000000000"),
            currency1: address!("0000000000000000000000000000000000000001"),
            fee: 3000,
            tick_spacing: 60,
            hooks: address!("0000000000000000000000000000000000000000"),
            sqrt_price_x96: U256::from(1u128) << 96,
            tick: 0,
            log_index: 1,
        }
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
    fn initialize_preserves_native_eth_currency_and_uses_weth_display_denom() {
        let event = initialize_event();
        let pool = UniswapV4Pool::from_initialize_event(
            &event,
            "0x0000000000000000000000000000000000000001",
            display_denom_for_v4_currency(event.currency0),
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(false),
                ..BasePoolConfig::new(18)
            },
        );

        assert_eq!(pool.pool_key.currency0, V4_NATIVE_ETH_ADDRESS);
        assert_eq!(pool.base.identity.denom_address, WETH_ADDRESS);
    }

    #[test]
    fn unknown_pool_id_is_not_a_match() {
        let event = initialize_event();
        let pool = UniswapV4Pool::from_initialize_event(
            &event,
            "0x0000000000000000000000000000000000000001",
            display_denom_for_v4_currency(event.currency0),
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(false),
                ..BasePoolConfig::new(18)
            },
        );

        assert!(!pool.matches_event(
            event.pool_manager_address,
            b256!("2222222222222222222222222222222222222222222222222222222222222222")
        ));
    }

    #[test]
    fn modifies_active_liquidity_only_when_current_tick_inside_range() {
        let event = initialize_event();
        let mut pool = UniswapV4Pool::from_initialize_event(
            &event,
            "0x0000000000000000000000000000000000000001",
            display_denom_for_v4_currency(event.currency0),
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(false),
                ..BasePoolConfig::new(18)
            },
        );
        let (mut processed, ctx) = tx();
        processed
            .uniswap_v4_modifies
            .push(ProcessedV4ModifyLiquidityEvent {
                pool_manager_address: event.pool_manager_address,
                event_id: event.event_id,
                sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                tick_lower: -60,
                tick_upper: 60,
                liquidity_delta: 1_000_000_000_000_000_000i128,
                salt: B256::ZERO,
                log_index: 2,
            });

        pool.update_from_processed_transaction(&processed, &ctx)
            .unwrap();

        assert_eq!(pool.active_liquidity, 1_000_000_000_000_000_000u128);
        assert_eq!(pool.base.price(), 1.0);
    }
}
