//! PancakeSwap V3 Price Reader.
//!
//! Implements direct contract calls to PancakeSwap V3 pools on Ethereum.
//!
//! # Implementation Details
//!
//! *   **V3 Fork:** Identical math and storage layout to Uniswap V3.
//! *   **Fees:** Supports PancakeSwap-specific fee tiers [100, 500, 2500, 10000].
//! *   **Factory:** Uses PancakeSwap V3 Factory (`0x0BFb...`).

use crate::core::{
    LiquidityMetrics, Pool, PoolKind, PoolState, PriceData, PriceError, PricePair, PriceSource,
    Protocol, RawChainPrice, Token,
};
use alloy_primitives::{Address, Bytes, B256, U256};
use reth_chain_query::common_addresses::get_address_by_name;
use reth_provider::{BlockNumReader, BlockReader};
use std::sync::Arc;
use tx_simulator::TxSimulator;

// Use the shared simulator module to avoid multiple instances
use crate::shared_simulator::get_or_create_simulator_with_provider;

/// PancakeSwap V3 price reader
pub struct PancakeswapV3Reader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    factory_address: Address,
    simulator: Arc<TxSimulator>,
}

impl PancakeswapV3Reader {
    /// PancakeSwap V3 Factory on Ethereum mainnet
    const FACTORY_ADDRESS: &'static str = "0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865";

    /// Initialize reader with path to Reth database
    pub fn new(db_path: &str) -> Result<Self, PriceError> {
        Self::from_provider(crate::utils::open_provider_factory(db_path)?)
    }

    /// Initialize reader from shared provider factory (avoids creating new DB connections)
    pub fn from_provider(
        provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        // Get or create shared simulator using the SAME provider factory - no new DB connection!
        let simulator = get_or_create_simulator_with_provider(provider_factory.clone())?;

        let factory_address = Address::from_slice(
            &hex::decode(Self::FACTORY_ADDRESS)
                .map_err(|e| PriceError::ConversionError(e.to_string()))?,
        );

        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            factory_address,
            simulator,
        })
    }

    /// Get the latest price for a given pair
    pub async fn get_latest_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        self.get_price_at_block(pair, latest_block).await
    }

    /// Get price at specific block
    pub async fn get_price_at_block(
        &self,
        pair: &str,
        block_number: u64,
    ) -> Result<PriceData, PriceError> {
        // Parse pair string: "WETH/USDC_500" -> (WETH, USDC, 500)
        let (token0_symbol, token1_symbol, fee) = self.parse_pair(pair)?;

        // Get token addresses from symbols
        let token0_addr = self.get_token_address(&token0_symbol)?;
        let token1_addr = self.get_token_address(&token1_symbol)?;

        // Find pool address using factory.getPool()
        let pool_address = self
            .get_pool_address(token0_addr, token1_addr, fee, block_number)
            .await?;

        let state_provider = self
            .provider_factory
            .history_by_block_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;

        let slot0_storage = state_provider
            .storage(pool_address, B256::from(U256::ZERO))
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .unwrap_or_default();

        let slot0 = self.decode_slot0(slot0_storage)?;

        if slot0.sqrt_price_x96.is_zero() {
            return Err(PriceError::PriceNotAvailable(format!(
                "Pool 0x{:x} has zero sqrt price",
                pool_address
            )));
        }

        let liquidity_slot = U256::from(4);
        let liquidity_storage = state_provider
            .storage(pool_address, B256::from(liquidity_slot))
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .unwrap_or_default();
        let liquidity = U256::from_be_bytes(liquidity_storage.to_be_bytes::<32>());

        let token0_decimals = self.get_token_decimals(&token0_symbol);
        let token1_decimals = self.get_token_decimals(&token1_symbol);

        let base_token = Token::new(token0_addr, token0_symbol.clone(), token0_decimals);
        let quote_token = Token::new(token1_addr, token1_symbol.clone(), token1_decimals);

        let mut price = RawChainPrice::from_sqrt_price_x96(slot0.sqrt_price_x96);
        if token0_addr > token1_addr {
            price = price.inverse();
        }

        let pool_kind = PoolKind::V3 {
            sqrt_price_x96: slot0.sqrt_price_x96,
            tick: slot0.tick,
            fee_bps: fee,
            liquidity,
        };

        let pool = Pool::new(
            pool_address,
            Protocol::PancakeswapV3,
            base_token.clone(),
            quote_token.clone(),
            pool_kind,
        );

        let block = self
            .provider_factory
            .block_by_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .ok_or(PriceError::InvalidBlockNumber(block_number))?;

        let pool_state = PoolState::new(pool.clone(), block_number, block.timestamp);
        let liquidity_metrics = LiquidityMetrics::from_pool(&pool);
        let price_pair = PricePair::new(pair.to_string(), base_token.clone(), quote_token.clone());
        let price_id = pool.create_price_id(base_token.address, quote_token.address);

        Ok(PriceData::new(
            price_pair,
            price,
            block_number,
            block.timestamp,
            PriceSource::Amm(crate::core::AmmPriceSource {
                protocol: Protocol::PancakeswapV3,
                price_id,
                pool,
                pool_state,
                liquidity: liquidity_metrics,
            }),
        ))
    }

    /// Get pool address from factory
    async fn get_pool_address(
        &self,
        token_a: Address,
        token_b: Address,
        fee: u32,
        block_number: u64,
    ) -> Result<Address, PriceError> {
        // getPool(address,address,uint24) selector: 0x1698ee82
        let selector =
            hex::decode("1698ee82").map_err(|e| PriceError::ConversionError(e.to_string()))?;

        let mut call_data = selector;
        // Add token_a (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(token_a.as_slice());
        // Add token_b (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(token_b.as_slice());
        // Add fee (32 bytes)
        let fee_bytes = fee.to_be_bytes();
        call_data.extend_from_slice(&[0u8; 28]); // Padding
        call_data.extend_from_slice(&fee_bytes);

        let result = self
            .simulator
            .simulate_view_function(
                self.factory_address,
                Bytes::from(call_data),
                Some(block_number),
            )
            .await
            .map_err(|e| PriceError::DatabaseError(format!("Failed to call getPool(): {}", e)))?;

        if !result.success || result.output.len() < 32 {
            return Err(PriceError::DatabaseError(
                "getPool() call failed or returned invalid data".to_string(),
            ));
        }

        // Extract address from last 20 bytes of the 32-byte response
        let pool_bytes = &result.output[12..32];
        Ok(Address::from_slice(pool_bytes))
    }

    /// Parse pair string like "WETH/USDC_500" into components
    fn parse_pair(&self, pair: &str) -> Result<(String, String, u32), PriceError> {
        // Check if it has fee tier (e.g., "WETH/USDC_500")
        if let Some(underscore_pos) = pair.rfind('_') {
            let tokens_part = &pair[..underscore_pos];
            let fee_part = &pair[underscore_pos + 1..];

            let fee = fee_part.parse::<u32>().map_err(|_| {
                PriceError::ConversionError(format!("Invalid fee tier: {}", fee_part))
            })?;

            let tokens: Vec<&str> = tokens_part.split('/').collect();
            if tokens.len() != 2 {
                return Err(PriceError::ConversionError(format!(
                    "Invalid pair format: {}",
                    pair
                )));
            }

            Ok((tokens[0].to_string(), tokens[1].to_string(), fee))
        } else {
            // No fee specified, default to 500 (0.05%)
            let tokens: Vec<&str> = pair.split('/').collect();
            if tokens.len() != 2 {
                return Err(PriceError::ConversionError(format!(
                    "Invalid pair format: {}",
                    pair
                )));
            }

            Ok((tokens[0].to_string(), tokens[1].to_string(), 500))
        }
    }

    /// Get token address from symbol
    fn get_token_address(&self, symbol: &str) -> Result<Address, PriceError> {
        // Prefer centralized addresses; accept alias USD -> USDC
        let sym = if symbol == "USD" { "USDC" } else { symbol };
        if let Some(addr) = get_address_by_name(sym) {
            return Ok(addr);
        }
        Err(PriceError::UnsupportedPair(format!(
            "Unknown token: {}",
            symbol
        )))
    }

    /// Get token decimals
    fn get_token_decimals(&self, symbol: &str) -> u8 {
        match symbol {
            "WETH" | "ETH" | "DAI" | "FRAX" => 18,
            "USDC" | "USD" | "USDT" => 6, // USD defaults to USDC decimals
            _ => 18,                      // Default to 18
        }
    }

    fn decode_slot0(&self, storage: U256) -> Result<Slot0Data, PriceError> {
        if storage.is_zero() {
            return Err(PriceError::PriceNotAvailable(
                "PancakeSwap V3 slot0 empty".to_string(),
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
}

#[derive(Debug)]
struct Slot0Data {
    sqrt_price_x96: U256,
    tick: i32,
}
