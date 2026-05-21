//! Chainlink Oracle Price Reader.
//!
//! Reads price data directly from Chainlink Aggregator contracts on Ethereum using the Reth database.
//!
//! # Implementation Details
//!
//! *   **Proxy Pattern:** Resolves the actual Aggregator contract from the Proxy address using `aggregator()`.
//! *   **Simulation:** Uses `TxSimulator` to execute `latestRoundData()` view functions against the local DB state.
//! *   **Historical Access:** Can query prices at any block height available in the database.
//! *   **Data Normalization:** Converts Chainlink's 8-decimal int256 answers into standard `PriceData`.

use crate::core::{
    OraclePriceSource, PriceData, PriceError, PriceId, PricePair, PriceSource, Protocol,
    RawChainPrice, Token,
};
use alloy_primitives::{Address, Bytes, U256};
use reth_provider::{BlockNumReader, BlockReader};
use std::collections::HashMap;
use std::sync::Arc;
use tx_simulator::TxSimulator;

// Use the shared simulator module to avoid multiple instances
use crate::shared_simulator::get_or_create_simulator_with_provider;

/// Chainlink oracle price reader
#[derive(Clone)]
pub struct ChainlinkReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    oracle_addresses: HashMap<String, Address>,
    simulator: Arc<TxSimulator>,
}

impl ChainlinkReader {
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

        let mut oracle_addresses = HashMap::new();

        // Major price feeds on Ethereum mainnet (these are proxy addresses)
        oracle_addresses.insert(
            "ETH/USD".to_string(),
            Address::from_slice(
                &hex::decode("5f4eC3Df9cbd43714FE2740f5E3616155c5b8419")
                    .map_err(|e| PriceError::ConversionError(e.to_string()))?,
            ),
        );
        oracle_addresses.insert(
            "BTC/USD".to_string(),
            Address::from_slice(
                &hex::decode("F4030086522a5bEEa4988F8cA5B36dbC97BeE88c")
                    .map_err(|e| PriceError::ConversionError(e.to_string()))?,
            ),
        );

        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            oracle_addresses,
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

    /// Get price at a specific block
    pub async fn get_price_at_block(
        &self,
        pair: &str,
        block_number: u64,
    ) -> Result<PriceData, PriceError> {
        let proxy_address = self
            .oracle_addresses
            .get(pair)
            .ok_or_else(|| PriceError::PriceNotAvailable(pair.to_string()))?;

        // Step 1: Call aggregator() on the proxy to get the actual aggregator address
        // Function selector for aggregator(): 0x245a7bfc
        let aggregator_selector =
            hex::decode("245a7bfc").map_err(|e| PriceError::ConversionError(e.to_string()))?;
        let aggregator_call_data = Bytes::from(aggregator_selector);

        let aggregator_result = self
            .simulator
            .simulate_view_function(*proxy_address, aggregator_call_data, Some(block_number))
            .await
            .map_err(|e| {
                PriceError::DatabaseError(format!("Failed to call aggregator(): {}", e))
            })?;

        if !aggregator_result.success || aggregator_result.output.len() < 32 {
            return Err(PriceError::PriceNotAvailable(format!(
                "Failed to get aggregator for {}",
                pair
            )));
        }

        // Extract aggregator address from the result (last 20 bytes of the 32-byte word)
        let aggregator_bytes = &aggregator_result.output[12..32];
        let aggregator_address = Address::from_slice(aggregator_bytes);

        // Step 2: Call latestRoundData() on the aggregator to get the price
        // Function selector for latestRoundData(): 0xfeaf968c
        let latest_round_selector =
            hex::decode("feaf968c").map_err(|e| PriceError::ConversionError(e.to_string()))?;
        let latest_round_call_data = Bytes::from(latest_round_selector);

        let price_result = self
            .simulator
            .simulate_view_function(
                aggregator_address,
                latest_round_call_data,
                Some(block_number),
            )
            .await
            .map_err(|e| {
                PriceError::DatabaseError(format!("Failed to call latestRoundData(): {}", e))
            })?;

        if !price_result.success || price_result.output.len() < 160 {
            return Err(PriceError::PriceNotAvailable(format!(
                "Failed to get price data for {}",
                pair
            )));
        }

        // latestRoundData returns (roundId, answer, startedAt, updatedAt, answeredInRound)
        // Each value is 32 bytes, answer is at offset 32-64
        let round_id_bytes = &price_result.output[0..32];
        let round_id = U256::from_be_slice(round_id_bytes);
        let answer_bytes = &price_result.output[32..64];
        let answer = U256::from_be_slice(answer_bytes);

        // Extract updatedAt timestamp (offset 96-128)
        let updated_at_bytes = &price_result.output[96..128];
        let updated_at = U256::from_be_slice(updated_at_bytes).to::<u64>();

        // Get block timestamp
        let block = self
            .provider_factory
            .block_by_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .ok_or(PriceError::InvalidBlockNumber(block_number))?;

        let (base_token, quote_token) = Self::tokens_for_pair(pair)?;
        let price_pair = PricePair::new(pair.to_string(), base_token.clone(), quote_token.clone());

        // Represent quote/base using raw units: quote = answer * 10^8, base = 10^18 (wei)
        let base_scale = U256::from(10).pow(U256::from(18u8));
        let precise_price = RawChainPrice::new(answer, base_scale);

        let price_id = PriceId::new(
            base_token.address,
            quote_token.address,
            Protocol::Chainlink,
            aggregator_address,
            None,
        );

        let source = PriceSource::Oracle(OraclePriceSource {
            price_id,
            feed_address: aggregator_address,
            answer,
            answer_decimals: 8,
            round_id: Some(round_id),
            updated_at,
        });

        Ok(PriceData::new(
            price_pair,
            precise_price,
            block_number,
            block.timestamp,
            source,
        ))
    }

    /// Get list of supported price pairs
    pub fn supported_pairs(&self) -> Vec<String> {
        self.oracle_addresses.keys().cloned().collect()
    }

    fn tokens_for_pair(pair: &str) -> Result<(Token, Token), PriceError> {
        match pair {
            "ETH/USD" => {
                let weth = Token::new(
                    Address::from_slice(
                        &hex::decode("C02aaa39b223FE8D0A0e5C4F27eAD9083C756Cc2")
                            .map_err(|e| PriceError::ConversionError(e.to_string()))?,
                    ),
                    "WETH",
                    18,
                );
                // Represent USD as pseudo-token with 8 decimals (Chainlink standard)
                let usd = Token::new(Address::ZERO, "USD", 8);
                Ok((weth, usd))
            }
            other => Err(PriceError::PriceNotAvailable(other.to_string())),
        }
    }
}
