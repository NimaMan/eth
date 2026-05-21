//! Uniswap V3 Price Reader.
//!
//! specific pool state reading for Uniswap V3.
//!
//! # Implementation Details
//!
//! *   **Storage Layout:** Reads `slot0` (slot 0) directly from the EVM state to get `sqrtPriceX96` and `tick`.
//! *   **Address Computation:** Uses `reth_chain_query` to deterministically compute pool addresses from tokens and fee tiers.
//! *   **Smart Lookup:** Implements heuristic lookup to find pools even if the pair order is inverted or fee tier is unspecified (defaults to 500).
//! *   **Math:** Converts `sqrtPriceX96` to a usable price using fixed-point arithmetic.

use crate::core::price_data_models::{PoolKind, RawChainPrice, Token};
use crate::core::{
    AmmPriceSource, LiquidityMetrics, Pool, PoolState, PriceData, PriceError, PricePair,
    PriceSource, Protocol,
};
use alloy_primitives::{Address, B256, U256};
use reth_chain_query::common_addresses::dex_token_denom_pairs::uniswap_v3_tokens;
use reth_chain_query::dex::compute_uniswap_v3_pool;
use reth_provider::{BlockNumReader, BlockReader};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
struct PoolInfo {
    label: String,
    address: Address,
    token0: Token,
    token1: Token,
    base_token: Token,
    quote_token: Token,
    base_is_token0: bool,
    fee_tier: u32,
}

#[derive(Debug)]
struct Slot0Data {
    sqrt_price_x96: U256,
    tick: i32,
}

pub struct UniswapV3Reader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    pools: HashMap<String, PoolInfo>,
}

impl UniswapV3Reader {
    pub fn new(db_path: &str) -> Result<Self, PriceError> {
        Self::from_provider(crate::utils::open_provider_factory(db_path)?)
    }

    pub fn from_provider(
        provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        let mut pools = HashMap::new();
        Self::register_default_pools(&mut pools)?;

        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            pools,
        })
    }

    fn register_default_pools(pools: &mut HashMap<String, PoolInfo>) -> Result<(), PriceError> {
        for info in uniswap_v3_tokens() {
            let token = Token::new(info.token_address, info.symbol, info.decimals);
            let denom = Token::new(info.denom_address, info.denom_symbol, info.denom_decimals);
            let label = format!("{}/{}_V3_{}", info.symbol, info.denom_symbol, info.fee_tier);

            let (token0, token1, base_is_token0) = if info.token_address < info.denom_address {
                (token.clone(), denom.clone(), true)
            } else {
                (denom.clone(), token.clone(), false)
            };

            let address =
                compute_uniswap_v3_pool(info.token_address, info.denom_address, info.fee_tier);

            pools.insert(
                label.clone(),
                PoolInfo {
                    label,
                    address,
                    token0,
                    token1,
                    base_token: token.clone(),
                    quote_token: denom.clone(),
                    base_is_token0,
                    fee_tier: info.fee_tier,
                },
            );
        }

        Ok(())
    }

    pub fn supported_pairs(&self) -> Vec<String> {
        self.pools.keys().cloned().collect()
    }

    pub fn get_latest_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        self.get_price_at_block(pair, latest_block)
    }

    pub fn get_price_at_block(
        &self,
        pair: &str,
        block_number: u64,
    ) -> Result<PriceData, PriceError> {
        // Attempt smart lookup if direct match fails
        let pool_info = if let Some(info) = self.pools.get(pair) {
            info
        } else {
            // Smart lookup:
            // 1. Try appending default fee tier (500) if missing
            // 2. Try inverse pair with default fee tier
            // "ETH/USDC" -> "USDC/WETH_V3_500" or "WETH/USDC_V3_500"

            let parts: Vec<&str> = pair.split('/').collect();
            if parts.len() == 2 {
                let (base, quote) = (parts[0], parts[1]);
                let mapped_base = if base == "ETH" { "WETH" } else { base };
                let mapped_quote = if quote == "ETH" { "WETH" } else { quote };

                // Try constructing expected keys
                // Standard: "BASE/QUOTE_V3_500"
                let key_std = format!("{}/{}_V3_500", mapped_base, mapped_quote);
                if let Some(info) = self.pools.get(&key_std) {
                    info
                } else {
                    // Inverse: "QUOTE/BASE_V3_500"
                    let key_inv = format!("{}/{}_V3_500", mapped_quote, mapped_base);
                    if let Some(info) = self.pools.get(&key_inv) {
                        info
                    } else {
                        return Err(PriceError::PriceNotAvailable(pair.to_string()));
                    }
                }
            } else {
                return Err(PriceError::PriceNotAvailable(pair.to_string()));
            }
        };

        let state_provider = self
            .provider_factory
            .history_by_block_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;

        let slot0_storage = state_provider
            .storage(pool_info.address, B256::from(U256::ZERO))
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .unwrap_or_default();

        let slot0 = self.decode_slot0(slot0_storage)?;

        let block = self
            .provider_factory
            .block_by_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .ok_or(PriceError::InvalidBlockNumber(block_number))?;

        let liquidity_slot = U256::from(4);
        let liquidity_storage = state_provider
            .storage(pool_info.address, B256::from(liquidity_slot))
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .unwrap_or_default();
        let liquidity = U256::from_be_bytes(liquidity_storage.to_be_bytes::<32>());

        let price = self.precise_price_from_slot(pool_info, slot0.sqrt_price_x96);

        let pool_kind = PoolKind::V3 {
            sqrt_price_x96: slot0.sqrt_price_x96,
            tick: slot0.tick,
            fee_bps: pool_info.fee_tier,
            liquidity,
        };

        let pool = Pool::new(
            pool_info.address,
            Protocol::UniswapV3,
            pool_info.token0.clone(),
            pool_info.token1.clone(),
            pool_kind,
        );
        let pool_state = PoolState::new(pool.clone(), block_number, block.timestamp);
        let liquidity_metrics = LiquidityMetrics::from_pool(&pool);

        let price_id =
            pool.create_price_id(pool_info.base_token.address, pool_info.quote_token.address);

        Ok(PriceData::new(
            PricePair::new(
                pool_info.label.clone(),
                pool_info.base_token.clone(),
                pool_info.quote_token.clone(),
            ),
            price,
            block_number,
            block.timestamp,
            PriceSource::Amm(AmmPriceSource {
                protocol: Protocol::UniswapV3,
                price_id,
                pool,
                pool_state,
                liquidity: liquidity_metrics,
            }),
        ))
    }

    pub fn get_latest_price_by_pool(
        &self,
        pool: Address,
        token0_decimals: u8,
        token1_decimals: u8,
        base_is_token0: bool,
    ) -> Result<(RawChainPrice, u64), PriceError> {
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        let price = self.get_price_by_pool_at_block(
            pool,
            token0_decimals,
            token1_decimals,
            base_is_token0,
            latest_block,
        )?;
        Ok((price, latest_block))
    }

    pub fn get_price_by_pool_at_block(
        &self,
        pool: Address,
        token0_decimals: u8,
        token1_decimals: u8,
        base_is_token0: bool,
        block_number: u64,
    ) -> Result<RawChainPrice, PriceError> {
        let state_provider = self
            .provider_factory
            .history_by_block_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;

        let slot0_storage = state_provider
            .storage(pool, B256::from(U256::ZERO))
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .unwrap_or_default();

        let slot0 = self.decode_slot0(slot0_storage)?;
        let raw = RawChainPrice::from_sqrt_price_x96(slot0.sqrt_price_x96);
        let mut price = if base_is_token0 { raw } else { raw.inverse() };
        price.decimals_adjusted = false;

        // Track decimals by storing in price struct metadata
        // (display helpers will adjust using provided decimals)
        let _ = (token0_decimals, token1_decimals); // retained for compatibility

        Ok(price)
    }

    fn decode_slot0(&self, storage: U256) -> Result<Slot0Data, PriceError> {
        if storage.is_zero() {
            return Err(PriceError::PriceNotAvailable(
                "Uniswap V3 slot0 empty".to_string(),
            ));
        }

        let sqrt_price_x96 = storage & (U256::from(u128::MAX) << 64 | U256::from(u64::MAX));

        let tick_mask = U256::from((1u32 << 24) - 1);
        let tick_raw: U256 = (storage >> 160) & tick_mask;
        let tick = if tick_raw >= U256::from(1u32 << 23) {
            let tick_u32 = tick_raw.to::<u32>();
            let tick_signed = (tick_u32 as i64) - (1i64 << 24);
            tick_signed as i32
        } else {
            tick_raw.to::<i32>()
        };

        Ok(Slot0Data {
            sqrt_price_x96,
            tick,
        })
    }

    fn precise_price_from_slot(&self, info: &PoolInfo, sqrt_price_x96: U256) -> RawChainPrice {
        let raw = RawChainPrice::from_sqrt_price_x96(sqrt_price_x96);
        if info.base_is_token0 {
            raw
        } else {
            raw.inverse()
        }
    }
}
