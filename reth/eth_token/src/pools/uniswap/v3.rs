use std::collections::BTreeMap;

use alloy_primitives::U256;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_processor::tx_processor::data_models::{
    UniswapV3BurnEvent as ProcessedV3BurnEvent,
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
use super::v2::UniswapV2TxContext;

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
    pub initialize_events: Vec<Value>,
    pub ignored_position_liquidity_events: Vec<Value>,
    pub last_virtual_reserves: Option<ConcentratedVirtualReserveSnapshot>,
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
            initialize_events: Vec::new(),
            ignored_position_liquidity_events: Vec::new(),
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
        for event in events {
            match event {
                V3PoolAction::Initialize(event) => self.process_initialize(event, tx),
                V3PoolAction::Mint(event) => self.process_mint(event, tx),
                V3PoolAction::Burn(event) => self.process_burn(event, tx),
                V3PoolAction::Swap(event) => self.process_swap(event, tx),
            }
        }

        if !transaction.uniswap_v3_increases.is_empty()
            || !transaction.uniswap_v3_decreases.is_empty()
            || !transaction.uniswap_v3_positions.is_empty()
        {
            append_with_history_limit(
                &mut self.ignored_position_liquidity_events,
                json!({
                    "block_number": tx.block_number,
                    "tx_hash": tx.tx_hash,
                    "reason": "v3 position-manager liquidity events are informational; core Mint/Burn carries pool liquidity authority",
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

    fn process_mint(&mut self, event: &ProcessedV3MintEvent, tx: &UniswapV2TxContext) {
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
        self.refresh_virtual_reserves(tx);
    }

    fn process_burn(&mut self, event: &ProcessedV3BurnEvent, tx: &UniswapV2TxContext) {
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
        self.refresh_virtual_reserves(tx);
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
        self.base
            .mark_can_buy_from_event(tx.block_number, tx.tx_hash.clone(), tx.block_timestamp);
        append_with_history_limit(
            &mut self.base.swap_events,
            event_json(event, tx),
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

fn same_address(address: &alloy_primitives::Address, value: &str) -> bool {
    format!("{address:#x}") == normalize_address(value)
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256, U256};
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
        assert!(pool.base.state.can_buy);
        assert_eq!(pool.base.state.total_swaps, 1);
    }
}
