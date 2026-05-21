//! Multi-Venue Price Aggregator.
//!
//! Orchestrates price reading across all configured DEXs and Oracles.
//!
//! # Implementation Details
//!
//! *   **Normalization:** Uses `PriceTickerMapper` to translate generic pairs (e.g., "ETH/USD") into protocol-specific keys.
//! *   **Aggregation:** Computes median and weighted average prices across sources.
//! *   **Categorization:** Splits results into AMM vs Oracle sources for analysis.
//! *   **Parallelism:** Supports async fetching for high-throughput reading.

use crate::core::{PriceData, PriceError};
use crate::dex::{BalancerReader, CurveReader, SushiSwapReader, UniswapV2Reader, UniswapV3Reader};
use crate::oracles::ChainlinkReader;
use crate::utils::price_ticker_mapping::{PriceTickerMapper, Protocol};
use std::collections::HashMap;
use std::sync::Arc;

pub struct MultiVenuePriceReader {
    // AMM readers
    pub uniswap_v2: Option<UniswapV2Reader>,
    pub uniswap_v3: Option<UniswapV3Reader>,
    pub sushiswap: Option<SushiSwapReader>,
    pub curve: Option<CurveReader>,
    pub balancer: Option<BalancerReader>,

    // Oracle readers
    pub chainlink: Option<ChainlinkReader>,

    // Price ticker mapping system
    mapper: PriceTickerMapper,
}

impl MultiVenuePriceReader {
    /// Create new aggregated reader with shared provider
    pub fn new(_provider: Arc<crate::utils::EthPriceProviderFactory>) -> Self {
        Self {
            uniswap_v2: None,
            uniswap_v3: None,
            sushiswap: None,
            curve: None,
            balancer: None,
            chainlink: None,
            mapper: PriceTickerMapper::new(),
        }
    }

    /// Initialize all AMM readers
    pub fn with_all_amms(
        mut self,
        provider: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        self.uniswap_v2 = Some(UniswapV2Reader::from_provider(provider.clone())?);
        self.uniswap_v3 = Some(UniswapV3Reader::from_provider(provider.clone())?);
        self.sushiswap = Some(SushiSwapReader::from_provider(provider.clone())?);
        self.curve = Some(CurveReader::from_provider(provider.clone())?);
        self.balancer = Some(BalancerReader::from_provider(provider.clone())?);
        Ok(self)
    }

    /// Initialize all oracle readers
    pub fn with_all_oracles(
        mut self,
        provider: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        self.chainlink = Some(ChainlinkReader::from_provider(provider.clone())?);
        Ok(self)
    }

    /// Get all available prices for a pair from all sources (sync version - AMM only)
    pub fn get_all_prices(
        &self,
        generic_pair: &str,
    ) -> HashMap<String, Result<PriceData, PriceError>> {
        let mut all_prices = HashMap::new();

        // Get AMM prices with mapped pair names
        if let Some(reader) = &self.uniswap_v2 {
            match self
                .mapper
                .get_protocol_pair(generic_pair, Protocol::UniswapV2)
            {
                Ok(mapped_pair) => {
                    all_prices.insert(
                        "UniswapV2".to_string(),
                        reader.get_latest_price(&mapped_pair),
                    );
                }
                Err(e) => {
                    all_prices.insert("UniswapV2".to_string(), Err(e));
                }
            }
        }

        if let Some(reader) = &self.uniswap_v3 {
            match self
                .mapper
                .get_protocol_pairs(generic_pair, Protocol::UniswapV3)
            {
                Ok(mapped_pairs) => {
                    for (i, mapped_pair) in mapped_pairs.iter().enumerate() {
                        let key = if mapped_pairs.len() > 1 {
                            format!("UniswapV3_{}", i + 1) // UniswapV3_1, UniswapV3_2, etc.
                        } else {
                            "UniswapV3".to_string()
                        };
                        all_prices.insert(key, reader.get_latest_price(mapped_pair));
                    }
                }
                Err(e) => {
                    all_prices.insert("UniswapV3".to_string(), Err(e));
                }
            }
        }

        if let Some(reader) = &self.sushiswap {
            match self
                .mapper
                .get_protocol_pair(generic_pair, Protocol::SushiSwap)
            {
                Ok(mapped_pair) => {
                    all_prices.insert(
                        "SushiSwap".to_string(),
                        reader.get_latest_price(&mapped_pair),
                    );
                }
                Err(e) => {
                    all_prices.insert("SushiSwap".to_string(), Err(e));
                }
            }
        }

        // Note: Curve and Balancer require async, so they're handled in get_all_prices_async

        all_prices
    }

    /// Get all available prices (async version for protocols that need it)
    pub async fn get_all_prices_async(
        &self,
        generic_pair: &str,
    ) -> HashMap<String, Result<PriceData, PriceError>> {
        let mut all_prices = HashMap::new();

        // Get AMM prices with mapped pair names
        if let Some(reader) = &self.uniswap_v2 {
            match self
                .mapper
                .get_protocol_pair(generic_pair, Protocol::UniswapV2)
            {
                Ok(mapped_pair) => {
                    all_prices.insert(
                        "UniswapV2".to_string(),
                        reader.get_latest_price(&mapped_pair),
                    );
                }
                Err(e) => {
                    all_prices.insert("UniswapV2".to_string(), Err(e));
                }
            }
        }

        if let Some(reader) = &self.uniswap_v3 {
            match self
                .mapper
                .get_protocol_pairs(generic_pair, Protocol::UniswapV3)
            {
                Ok(mapped_pairs) => {
                    for (i, mapped_pair) in mapped_pairs.iter().enumerate() {
                        let key = if mapped_pairs.len() > 1 {
                            format!("UniswapV3_{}", i + 1) // UniswapV3_1, UniswapV3_2, etc.
                        } else {
                            "UniswapV3".to_string()
                        };
                        all_prices.insert(key, reader.get_latest_price(mapped_pair));
                    }
                }
                Err(e) => {
                    all_prices.insert("UniswapV3".to_string(), Err(e));
                }
            }
        }

        if let Some(reader) = &self.sushiswap {
            match self
                .mapper
                .get_protocol_pair(generic_pair, Protocol::SushiSwap)
            {
                Ok(mapped_pair) => {
                    all_prices.insert(
                        "SushiSwap".to_string(),
                        reader.get_latest_price(&mapped_pair),
                    );
                }
                Err(e) => {
                    all_prices.insert("SushiSwap".to_string(), Err(e));
                }
            }
        }

        if let Some(reader) = &self.curve {
            match self.mapper.get_protocol_pair(generic_pair, Protocol::Curve) {
                Ok(mapped_pair) => {
                    all_prices.insert(
                        "Curve".to_string(),
                        reader.get_latest_price(&mapped_pair).await,
                    );
                }
                Err(e) => {
                    all_prices.insert("Curve".to_string(), Err(e));
                }
            }
        }

        if let Some(reader) = &self.balancer {
            match self
                .mapper
                .get_protocol_pair(generic_pair, Protocol::Balancer)
            {
                Ok(mapped_pair) => {
                    all_prices.insert(
                        "Balancer".to_string(),
                        reader.get_latest_price(&mapped_pair).await,
                    );
                }
                Err(e) => {
                    all_prices.insert("Balancer".to_string(), Err(e));
                }
            }
        }

        // Get Oracle prices
        if let Some(reader) = &self.chainlink {
            match self
                .mapper
                .get_protocol_pair(generic_pair, Protocol::Chainlink)
            {
                Ok(mapped_pair) => {
                    all_prices.insert(
                        "Chainlink".to_string(),
                        reader.get_latest_price(&mapped_pair).await,
                    );
                }
                Err(e) => {
                    all_prices.insert("Chainlink".to_string(), Err(e));
                }
            }
        }

        all_prices
    }

    /// Get median price across all available sources
    pub async fn get_median_price(&self, generic_pair: &str) -> Result<f64, PriceError> {
        let all_prices = self.get_all_prices_async(generic_pair).await;

        // Collect successful prices
        let mut prices: Vec<f64> = all_prices
            .values()
            .filter_map(|result| result.as_ref().ok())
            .map(|data| data.price_as_f64())
            .collect();

        if prices.is_empty() {
            return Err(PriceError::PriceNotAvailable(format!(
                "No prices available for {}",
                generic_pair
            )));
        }

        // Sort and find median
        prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = prices.len() / 2;

        let median = if prices.len() % 2 == 0 {
            (prices[mid - 1] + prices[mid]) / 2.0
        } else {
            prices[mid]
        };

        Ok(median)
    }

    /// Get weighted average price
    pub async fn get_weighted_average(
        &self,
        generic_pair: &str,
        weights: Option<HashMap<String, f64>>,
    ) -> Result<f64, PriceError> {
        let all_prices = self.get_all_prices_async(generic_pair).await;

        // Default weights (equal for all sources)
        let default_weight = 1.0;

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (source, result) in all_prices {
            if let Ok(price_data) = result {
                let weight = weights
                    .as_ref()
                    .and_then(|w| w.get(&source))
                    .unwrap_or(&default_weight);

                weighted_sum += price_data.price_as_f64() * weight;
                total_weight += weight;
            }
        }

        if total_weight == 0.0 {
            return Err(PriceError::PriceNotAvailable(format!(
                "No prices available for {}",
                generic_pair
            )));
        }

        Ok(weighted_sum / total_weight)
    }

    /// Get prices grouped by type (AMM vs Oracle)
    pub async fn get_prices_by_type(
        &self,
        generic_pair: &str,
    ) -> (Vec<(String, PriceData)>, Vec<(String, PriceData)>) {
        let all_prices = self.get_all_prices_async(generic_pair).await;

        let mut amm_prices = Vec::new();
        let mut oracle_prices = Vec::new();

        for (source, result) in all_prices {
            if let Ok(price_data) = result {
                if source == "Chainlink" {
                    oracle_prices.push((source, price_data));
                } else {
                    amm_prices.push((source, price_data));
                }
            }
        }

        (amm_prices, oracle_prices)
    }

    /// Get price statistics across all sources
    pub async fn get_price_stats(&self, generic_pair: &str) -> Result<PriceStats, PriceError> {
        let all_prices = self.get_all_prices_async(generic_pair).await;

        let prices: Vec<f64> = all_prices
            .values()
            .filter_map(|result| result.as_ref().ok())
            .map(|data| data.price_as_f64())
            .collect();

        if prices.is_empty() {
            return Err(PriceError::PriceNotAvailable(format!(
                "No prices available for {}",
                generic_pair
            )));
        }

        let min = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg = prices.iter().sum::<f64>() / prices.len() as f64;

        // Calculate standard deviation
        let variance = prices.iter().map(|p| (*p - avg).powi(2)).sum::<f64>() / prices.len() as f64;
        let std_dev = variance.sqrt();

        Ok(PriceStats {
            min,
            max,
            avg,
            median: self.get_median_price(generic_pair).await?,
            std_dev,
            count: prices.len(),
        })
    }
}

/// Statistics for prices across multiple sources
#[derive(Debug, Clone)]
pub struct PriceStats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub median: f64,
    pub std_dev: f64,
    pub count: usize,
}
