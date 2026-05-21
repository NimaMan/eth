//! DODO Price Reader.
//!
//! Implements direct contract calls to DODO pools using Proactive Market Maker (PMM) model.
//! DODO is different from traditional AMMs as it uses an active price-making algorithm.
//!
//! # Implementation Details
//!
//! *   **PMM Model:** Uses DODO's Proactive Market Maker formula for active price discovery.
//! *   **Registry:** Dynamically discovers pools via the DODO Registry (`0x8c9d...`).
//! *   **Pool Types:** Supports multiple pool variations (DPP, DSP, DVM).

use crate::core::{
    LiquidityMetrics, Pool, PoolKind, PoolState, PriceData, PriceError, PricePair, PriceSource,
    Protocol, RawChainPrice, Token,
};
use alloy_primitives::{Address, Bytes, U256};
use reth_chain_query::common_addresses::get_address_by_name;
use reth_provider::{BlockNumReader, BlockReader};
use std::sync::Arc;
use tx_simulator::TxSimulator;

// Use the shared simulator module to avoid multiple instances
use crate::shared_simulator::get_or_create_simulator_with_provider;

/// DODO price reader
pub struct DODOReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    registry_address: Address,
    simulator: Arc<TxSimulator>,
}

impl DODOReader {
    /// DODO Registry on Ethereum mainnet
    const REGISTRY_ADDRESS: &'static str = "8c9d230D45d6CfeE39a6680Fb7CB7E8DE7Ea8E71";

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

        let registry_address = Address::from_slice(
            &hex::decode(Self::REGISTRY_ADDRESS)
                .map_err(|e| PriceError::ConversionError(e.to_string()))?,
        );

        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            registry_address,
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
        // Parse pair string: "WETH/USDC" -> (WETH, USDC)
        let (token0_symbol, token1_symbol) = self.parse_pair(pair)?;
        let token0_addr = self.get_token_address(&token0_symbol)?;
        let token1_addr = self.get_token_address(&token1_symbol)?;

        // Prefer dynamic lookup via registry; fallback to known mapping
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        let pool_address = match self
            .get_dodo_pool(token0_addr, token1_addr, latest_block)
            .await
        {
            Ok(addr) => addr,
            Err(_) => self.get_known_pool_address(&token0_symbol, &token1_symbol)?,
        };

        // Get price from DODO pool
        let mid_price_raw = self.get_pool_mid_price(pool_address, block_number).await?;

        let base_decimals = self.get_token_decimals(&token0_symbol);
        let quote_decimals = self.get_token_decimals(&token1_symbol);

        let base_token = Token::new(token0_addr, token0_symbol.clone(), base_decimals);
        let quote_token = Token::new(token1_addr, token1_symbol.clone(), quote_decimals);

        let base_scale = U256::from(10).pow(U256::from(base_decimals));
        let quote_amount = U256::from(mid_price_raw);

        let (reserve0, reserve1) = if base_token.address < quote_token.address {
            (base_scale, quote_amount)
        } else {
            (quote_amount, base_scale)
        };

        let pool_kind = PoolKind::V2 {
            reserve0,
            reserve1,
            fee_bps: 30,
        };

        let pool = Pool::new(
            pool_address,
            Protocol::Dodo,
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
        let liquidity = LiquidityMetrics::from_pool(&pool);

        let price_pair = PricePair::new(
            format!("{}/{}", token0_symbol, token1_symbol),
            base_token.clone(),
            quote_token.clone(),
        );
        let precise_price = RawChainPrice::from_reserves(quote_amount, base_scale);
        let price_id = pool.create_price_id(base_token.address, quote_token.address);

        let source = PriceSource::Amm(crate::core::AmmPriceSource {
            protocol: Protocol::Dodo,
            price_id,
            pool,
            pool_state,
            liquidity,
        });

        Ok(PriceData::new(
            price_pair,
            precise_price,
            block_number,
            block.timestamp,
            source,
        ))
    }

    /// Find DODO pool for token pair using registry
    async fn get_dodo_pool(
        &self,
        base_token: Address,
        quote_token: Address,
        block_number: u64,
    ) -> Result<Address, PriceError> {
        // getDODOPoolBidirection(address,address) selector: 0x1fe0456a
        let selector =
            hex::decode("1fe0456a").map_err(|e| PriceError::ConversionError(e.to_string()))?;

        let mut call_data = selector;
        // Add base_token (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(base_token.as_slice());
        // Add quote_token (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(quote_token.as_slice());

        let result = self
            .simulator
            .simulate_view_function(
                self.registry_address,
                Bytes::from(call_data),
                Some(block_number),
            )
            .await
            .map_err(|e| {
                PriceError::DatabaseError(format!("Failed to call getDODOPoolBidirection(): {}", e))
            })?;

        if !result.success {
            return Err(PriceError::DatabaseError(
                "getDODOPoolBidirection() call failed".to_string(),
            ));
        }

        // Check if we got a valid response
        if !result.success || result.output.len() < 32 {
            // Try reversed order
            return self
                .get_dodo_pool_single(quote_token, base_token, block_number)
                .await;
        }

        // Extract pool addresses from result
        // The result contains multiple addresses, we take the first valid one
        let pool_bytes = &result.output[12..32];
        let pool_address = Address::from_slice(pool_bytes);

        // Check if address is zero (no pool found)
        if pool_address == Address::ZERO {
            // Try single direction lookup with reversed tokens
            return self
                .get_dodo_pool_single(quote_token, base_token, block_number)
                .await;
        }

        Ok(pool_address)
    }

    /// Find DODO pool single direction
    async fn get_dodo_pool_single(
        &self,
        base_token: Address,
        quote_token: Address,
        block_number: u64,
    ) -> Result<Address, PriceError> {
        // getDODOPool(address,address) selector: 0x30f8cf9b
        let selector =
            hex::decode("30f8cf9b").map_err(|e| PriceError::ConversionError(e.to_string()))?;

        let mut call_data = selector;
        // Add base_token (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(base_token.as_slice());
        // Add quote_token (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(quote_token.as_slice());

        let result = self
            .simulator
            .simulate_view_function(
                self.registry_address,
                Bytes::from(call_data),
                Some(block_number),
            )
            .await
            .map_err(|e| {
                PriceError::DatabaseError(format!("Failed to call getDODOPool(): {}", e))
            })?;

        if !result.success || result.output.len() < 32 {
            return Err(PriceError::DatabaseError(
                "No DODO pool found for this pair".to_string(),
            ));
        }

        // Extract address from last 20 bytes of the 32-byte response
        let pool_bytes = &result.output[12..32];
        let pool_address = Address::from_slice(pool_bytes);

        // Check if address is zero (no pool found)
        if pool_address == Address::ZERO {
            return Err(PriceError::DatabaseError(
                "DODO pool address is zero - no pool exists for this pair".to_string(),
            ));
        }

        Ok(pool_address)
    }

    /// Get mid price from DODO pool
    async fn get_pool_mid_price(
        &self,
        pool_address: Address,
        block_number: u64,
    ) -> Result<u128, PriceError> {
        // Use DODO's getMidPrice() function
        // getMidPrice() selector: 0xee27c689
        let selector =
            hex::decode("ee27c689").map_err(|e| PriceError::ConversionError(e.to_string()))?;

        let result = self
            .simulator
            .simulate_view_function(pool_address, Bytes::from(selector), Some(block_number))
            .await
            .map_err(|e| PriceError::DatabaseError(format!("Failed to call getMidPrice: {}", e)))?;

        if !result.success || result.output.len() < 32 {
            return Err(PriceError::DatabaseError(
                "getMidPrice() failed or returned invalid data".to_string(),
            ));
        }

        // Extract the mid price (uint256)
        let price_bytes = &result.output[0..32];
        let mid_price = U256::from_be_slice(price_bytes);

        // Convert to u128 for our usage
        Ok(mid_price.to::<u128>())
    }

    /// Parse pair string like "WETH/USDC" into components
    fn parse_pair(&self, pair: &str) -> Result<(String, String), PriceError> {
        let tokens: Vec<&str> = pair.split('/').collect();
        if tokens.len() != 2 {
            return Err(PriceError::ConversionError(format!(
                "Invalid pair format: {}",
                pair
            )));
        }

        Ok((tokens[0].to_string(), tokens[1].to_string()))
    }

    /// Get token address from symbol
    fn get_token_address(&self, symbol: &str) -> Result<Address, PriceError> {
        if let Some(addr) = get_address_by_name(symbol) {
            return Ok(addr);
        }
        Err(PriceError::UnsupportedPair(format!(
            "Unknown token: {}",
            symbol
        )))
    }

    /// Get known pool address for specific pairs
    fn get_known_pool_address(&self, token0: &str, token1: &str) -> Result<Address, PriceError> {
        let pool_addr = match (token0, token1) {
            ("WETH", "USDC") | ("ETH", "USDC") => "75c23271661d9d143DCb617222BC4BEc783eff34", // DODO V2 WETH-USDC
            ("WETH", "USDT") | ("ETH", "USDT") => {
                // Would need to find the actual DODO WETH/USDT pool address
                // For now, return an error
                return Err(PriceError::UnsupportedPair(
                    "DODO WETH/USDT pool not configured".to_string(),
                ));
            }
            ("WETH", "DAI") | ("ETH", "DAI") => {
                // Would need to find the actual DODO WETH/DAI pool address
                return Err(PriceError::UnsupportedPair(
                    "DODO WETH/DAI pool not configured".to_string(),
                ));
            }
            _ => {
                return Err(PriceError::UnsupportedPair(format!(
                    "No DODO pool for {}/{}",
                    token0, token1
                )))
            }
        };

        Ok(Address::from_slice(
            &hex::decode(pool_addr).map_err(|e| PriceError::ConversionError(e.to_string()))?,
        ))
    }

    /// Get token decimals
    fn get_token_decimals(&self, symbol: &str) -> u8 {
        match symbol {
            "WETH" | "ETH" | "DAI" | "FRAX" => 18,
            "USDC" | "USDT" => 6,
            _ => 18, // Default to 18
        }
    }
}
