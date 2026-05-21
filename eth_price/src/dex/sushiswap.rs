//! SushiSwap Price Reader.
//!
//! Specific pool state reading for SushiSwap (a Uniswap V2 fork).
//!
//! # Implementation Details
//!
//! *   **Storage Layout:** Identical to Uniswap V2; reads `reserves` (slot 8) directly from EVM storage.
//! *   **Address Computation:** Deterministically computes pool addresses using `compute_sushiswap_pool`.
//! *   **Price Calculation:** Derives price from the ratio of reserves.
//! *   **Clone Architecture:** Reuses the same logic patterns as `UniswapV2Reader` but with SushiSwap-specific factory addresses.

use crate::core::price_data_models::{PoolKind, RawChainPrice, Token};
use crate::core::{
    AmmPriceSource, LiquidityMetrics, Pool, PoolState, PriceData, PriceError, PricePair,
    PriceSource, Protocol,
};
use alloy_primitives::{Address, B256, U256};
use reth_chain_query::common_addresses::dex_token_denom_pairs::sushiswap_tokens;
use reth_chain_query::dex::compute_sushiswap_pool;
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
    fee_bps: u32,
}

pub struct SushiSwapReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    pools: HashMap<String, PoolInfo>,
}

impl SushiSwapReader {
    pub fn new(db_path: &str) -> Result<Self, PriceError> {
        Self::from_provider(crate::utils::open_provider_factory(db_path)?)
    }

    pub fn from_provider(
        provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        let pools = Self::build_pool_infos()?;
        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            pools,
        })
    }

    fn build_pool_infos() -> Result<HashMap<String, PoolInfo>, PriceError> {
        let mut pools = HashMap::new();

        for info in sushiswap_tokens() {
            let token = Token::new(info.token_address, info.symbol, info.decimals);
            let denom = Token::new(info.denom_address, info.denom_symbol, info.denom_decimals);
            let label = format!("{}/{}", info.symbol, info.denom_symbol);
            let (token0, token1, base_is_token0) = if info.token_address < info.denom_address {
                (token.clone(), denom.clone(), true)
            } else {
                (denom.clone(), token.clone(), false)
            };

            let address = compute_sushiswap_pool(info.token_address, info.denom_address);

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
                    fee_bps: 30,
                },
            );
        }

        Ok(pools)
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
        // Attempt to find exact match or inverse
        let pool_info = if let Some(info) = self.pools.get(pair) {
            info
        } else {
            // If "ETH/USDC" is requested, try to find "USDC/WETH" (common case)
            let parts: Vec<&str> = pair.split('/').collect();
            if parts.len() == 2 {
                let (base, quote) = (parts[0], parts[1]);
                let mapped_base = if base == "ETH" { "WETH" } else { base };
                let mapped_quote = if quote == "ETH" { "WETH" } else { quote };

                // Try inverse: "USDC/WETH"
                let inverse_pair = format!("{}/{}", mapped_quote, mapped_base);
                if let Some(info) = self.pools.get(&inverse_pair) {
                    info
                } else {
                    return Err(PriceError::PriceNotAvailable(pair.to_string()));
                }
            } else {
                return Err(PriceError::PriceNotAvailable(pair.to_string()));
            }
        };

        let state_provider = self
            .provider_factory
            .history_by_block_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;

        let reserves_slot = U256::from(8);
        let reserves_storage = state_provider
            .storage(pool_info.address, B256::from(reserves_slot))
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .unwrap_or_default();
        let reserves_packed = U256::from_be_bytes(reserves_storage.to_be_bytes::<32>());
        let mask = U256::from((1u128 << 112) - 1);
        let reserve0 = reserves_packed & mask;
        let reserve1 = (reserves_packed >> 112) & mask;

        let block = self
            .provider_factory
            .block_by_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .ok_or(PriceError::InvalidBlockNumber(block_number))?;

        let kind = PoolKind::V2 {
            reserve0,
            reserve1,
            fee_bps: pool_info.fee_bps,
        };
        let pool = Pool::new(
            pool_info.address,
            Protocol::SushiSwap,
            pool_info.token0.clone(),
            pool_info.token1.clone(),
            kind,
        );
        let pool_state = PoolState::new(pool.clone(), block_number, block.timestamp);
        let liquidity = LiquidityMetrics::from_pool(&pool);

        let (base_reserve, quote_reserve) = if pool_info.base_is_token0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };

        let price = RawChainPrice::from_reserves(quote_reserve, base_reserve);
        let pair_meta = PricePair::new(
            pool_info.label.clone(),
            pool_info.base_token.clone(),
            pool_info.quote_token.clone(),
        );
        let price_id =
            pool.create_price_id(pool_info.base_token.address, pool_info.quote_token.address);

        let source = PriceSource::Amm(AmmPriceSource {
            protocol: Protocol::SushiSwap,
            price_id,
            pool,
            pool_state,
            liquidity,
        });

        Ok(PriceData::new(
            pair_meta,
            price,
            block_number,
            block.timestamp,
            source,
        ))
    }

    pub fn get_reserves_latest_raw(&self, pool: Address) -> Result<(U256, U256), PriceError> {
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        self.get_reserves_at_block(pool, latest_block)
    }

    pub fn get_reserves_at_block(
        &self,
        pool: Address,
        block_number: u64,
    ) -> Result<(U256, U256), PriceError> {
        let state_provider = self
            .provider_factory
            .history_by_block_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        let reserves_slot = U256::from(8);
        let reserves_storage = state_provider
            .storage(pool, B256::from(reserves_slot))
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .unwrap_or_default();
        let reserves_packed = U256::from_be_bytes(reserves_storage.to_be_bytes::<32>());
        let mask = U256::from((1u128 << 112) - 1);
        let reserve0 = reserves_packed & mask;
        let reserve1 = (reserves_packed >> 112) & mask;
        Ok((reserve0, reserve1))
    }
}
