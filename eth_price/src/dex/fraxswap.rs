//! Fraxswap Price Reader.
//!
//! Implements direct contract calls to Fraxswap pools using Time-Weighted AMM (TWAMM) model.
//! Fraxswap extends Uniswap V2 with time-weighted average market maker functionality.
//!
//! # Implementation Details
//!
//! *   **TWAMM:** Prioritizes `getTwammReserves()` for time-weighted price discovery, falling back to standard reserves.
//! *   **Compatibility:** Fully compatible with Uniswap V2 interface mechanics.
//! *   **Factory:** Uses Frax-specific factory (`0x43eC...`) for pool discovery.

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

/// Fraxswap price reader
pub struct FraxswapReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    factory_address: Address,
    simulator: Arc<TxSimulator>,
}

impl FraxswapReader {
    /// Fraxswap Factory on Ethereum mainnet
    const FACTORY_ADDRESS: &'static str = "43eC799eAdd63848443E2347C49f5f52e8Fe0F6f";

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
        // Parse pair string: "FRAX/WETH" -> (FRAX, WETH)
        let (token0_symbol, token1_symbol) = self.parse_pair(pair)?;

        // For Fraxswap, we primarily use FRAX/WETH pool for all ETH pricing
        // Since FRAX is pegged to $1 USD, FRAX/WETH gives us ETH price in USD

        // Use hardcoded FRAX/WETH pool which has the best liquidity
        // FRAX and WETH addresses for price calculation
        let frax = get_address_by_name("FRAX")
            .ok_or_else(|| PriceError::ConversionError("Missing FRAX address".into()))?;
        let weth = get_address_by_name("WETH")
            .ok_or_else(|| PriceError::ConversionError("Missing WETH address".into()))?;

        // Discover pair dynamically via factory
        let pair_address = self.get_pair_address(frax, weth, block_number).await?;

        // First try to get TWAMM reserves (Fraxswap-specific)
        let pair_label = format!("{}/{}", token0_symbol, token1_symbol);

        if let Ok((reserve0, reserve1)) = self.get_twamm_reserves(pair_address, block_number).await
        {
            return self.build_price_data(
                &pair_label,
                block_number,
                pair_address,
                frax,
                weth,
                &token0_symbol,
                &token1_symbol,
                reserve0,
                reserve1,
            );
        }

        let (reserve0, reserve1) = self.get_reserves(pair_address, block_number).await?;
        self.build_price_data(
            &pair_label,
            block_number,
            pair_address,
            frax,
            weth,
            &token0_symbol,
            &token1_symbol,
            reserve0,
            reserve1,
        )
    }

    fn build_price_data(
        &self,
        pair_label: &str,
        block_number: u64,
        pair_address: Address,
        quote_address: Address,
        base_address: Address,
        base_symbol: &str,
        quote_symbol: &str,
        reserve0: u128,
        reserve1: u128,
    ) -> Result<PriceData, PriceError> {
        if reserve0 == 0 || reserve1 == 0 {
            return Err(PriceError::PriceNotAvailable(
                "Fraxswap pool has no liquidity".into(),
            ));
        }

        let reserve0_u256 = U256::from(reserve0);
        let reserve1_u256 = U256::from(reserve1);

        let base_decimals = 18;
        let quote_decimals = 18;

        let base_token = Token::new(base_address, base_symbol.to_string(), base_decimals);
        let quote_token = Token::new(quote_address, quote_symbol.to_string(), quote_decimals);

        let pool_kind = PoolKind::V2 {
            reserve0: reserve0_u256,
            reserve1: reserve1_u256,
            fee_bps: 30,
        };

        let pool = Pool::new(
            pair_address,
            Protocol::Fraxswap,
            quote_token.clone(),
            base_token.clone(),
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
            pair_label.to_string(),
            base_token.clone(),
            quote_token.clone(),
        );
        let precise_price = RawChainPrice::from_reserves(reserve0_u256, reserve1_u256);
        let price_id = pool.create_price_id(base_token.address, quote_token.address);

        let source = PriceSource::Amm(crate::core::AmmPriceSource {
            protocol: Protocol::Fraxswap,
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

    /// Get pair address from factory
    async fn get_pair_address(
        &self,
        token_a: Address,
        token_b: Address,
        block_number: u64,
    ) -> Result<Address, PriceError> {
        // getPair(address,address) selector: 0xe6a43905
        let selector =
            hex::decode("e6a43905").map_err(|e| PriceError::ConversionError(e.to_string()))?;

        let mut call_data = selector;
        // Add token_a (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(token_a.as_slice());
        // Add token_b (32 bytes)
        call_data.extend_from_slice(&[0u8; 12]); // Padding
        call_data.extend_from_slice(token_b.as_slice());

        let result = self
            .simulator
            .simulate_view_function(
                self.factory_address,
                Bytes::from(call_data),
                Some(block_number),
            )
            .await
            .map_err(|e| PriceError::DatabaseError(format!("Failed to call getPair(): {}", e)))?;

        if !result.success || result.output.len() < 32 {
            return Err(PriceError::DatabaseError(
                "getPair() call failed or returned invalid data".to_string(),
            ));
        }

        // Extract address from last 20 bytes of the 32-byte response
        let pair_bytes = &result.output[12..32];
        let pair_address = Address::from_slice(pair_bytes);

        // Check if address is zero (pair doesn't exist)
        if pair_address == Address::ZERO {
            return Err(PriceError::DatabaseError(
                "Fraxswap pair does not exist".to_string(),
            ));
        }

        Ok(pair_address)
    }

    /// Get TWAMM reserves (Fraxswap-specific)
    async fn get_twamm_reserves(
        &self,
        pair_address: Address,
        block_number: u64,
    ) -> Result<(u128, u128), PriceError> {
        // getTwammReserves() selector: 0x0c44e581
        let selector =
            hex::decode("0c44e581").map_err(|e| PriceError::ConversionError(e.to_string()))?;

        let result = self
            .simulator
            .simulate_view_function(pair_address, Bytes::from(selector), Some(block_number))
            .await
            .map_err(|e| {
                PriceError::DatabaseError(format!("Failed to call getTwammReserves(): {}", e))
            })?;

        if !result.success || result.output.len() < 64 {
            return Err(PriceError::DatabaseError(
                "getTwammReserves() call failed or returned invalid data".to_string(),
            ));
        }

        // Extract reserves (first 32 bytes = reserve0, next 32 bytes = reserve1)
        let reserve0_bytes = &result.output[0..32];
        let reserve1_bytes = &result.output[32..64];

        let reserve0_u256 = U256::from_be_slice(reserve0_bytes);
        let reserve1_u256 = U256::from_be_slice(reserve1_bytes);

        Ok((reserve0_u256.to::<u128>(), reserve1_u256.to::<u128>()))
    }

    /// Get regular reserves (fallback)
    async fn get_reserves(
        &self,
        pair_address: Address,
        block_number: u64,
    ) -> Result<(u128, u128), PriceError> {
        // getReserves() selector: 0x0902f1ac
        let selector =
            hex::decode("0902f1ac").map_err(|e| PriceError::ConversionError(e.to_string()))?;

        let result = self
            .simulator
            .simulate_view_function(pair_address, Bytes::from(selector), Some(block_number))
            .await
            .map_err(|e| {
                PriceError::DatabaseError(format!("Failed to call getReserves(): {}", e))
            })?;

        if !result.success || result.output.len() < 96 {
            return Err(PriceError::DatabaseError(
                "getReserves() call failed or returned invalid data".to_string(),
            ));
        }

        // Extract reserves (first 32 bytes = reserve0, next 32 bytes = reserve1, last 32 bytes = timestamp)
        let reserve0_bytes = &result.output[0..32];
        let reserve1_bytes = &result.output[32..64];

        let reserve0_u256 = U256::from_be_slice(reserve0_bytes);
        let reserve1_u256 = U256::from_be_slice(reserve1_bytes);

        Ok((reserve0_u256.to::<u128>(), reserve1_u256.to::<u128>()))
    }

    /// Parse pair string like "FRAX/WETH" into components
    fn parse_pair(&self, pair: &str) -> Result<(String, String), PriceError> {
        let tokens: Vec<&str> = pair.split('/').collect();
        if tokens.len() != 2 {
            return Err(PriceError::ConversionError(format!(
                "Invalid pair format: {}",
                pair
            )));
        }

        // For Fraxswap, we always return FRAX/WETH since that's our main pool
        // This allows us to accept different pair formats but use the same pool
        Ok(("FRAX".to_string(), "WETH".to_string()))
    }
}
