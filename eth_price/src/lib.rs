//! Ethereum price and liquidity readers.

pub mod base_reader;
pub mod core;
pub mod dex;
pub mod errors;
pub mod liquidity;
pub mod oracles;
pub mod price_data;
pub mod price_data_models;
pub mod shared_simulator;
pub mod stablecoins;
pub mod utils;

pub use core::PriceError;
pub use dex::{
    BalancerReader, BuySwapReader, CurveReader, DODOReader, FraxswapReader, MultiVenuePriceReader,
    PancakeswapV3Reader, SushiSwapReader, UniswapV2Reader, UniswapV3Reader,
};
pub use liquidity::{
    assess_denom_liquidity, DenomClass, LiquidityReference, PoolLiquidityAssessment,
    PoolLiquidityLevel,
};
pub use oracles::ChainlinkReader;
pub use price_data::{
    AggregatorPriceSource, AmmPriceSource, OraclePriceSource, PriceData, PricePair, PriceSource,
    SimulationDirection, SimulationPriceSource,
};
pub use price_data_models::{
    LiquidityMetrics, Pool, PoolKind, PoolState, PriceId, PriceObservation, Protocol,
    RawChainPrice, Token,
};
pub use stablecoins::StablecoinPriceReader;
pub use utils::DatabaseConfig;

use std::collections::HashMap;
use std::sync::Arc;

/// Main client for fetching Ethereum price data from supported on-chain sources.
pub struct EthPrices {
    db_config: DatabaseConfig,
    shared_provider: Option<Arc<crate::utils::EthPriceProviderFactory>>,
    pub uniswap_v2: Option<UniswapV2Reader>,
    pub uniswap_v3: Option<UniswapV3Reader>,
    pub chainlink: Option<ChainlinkReader>,
    pub sushiswap: Option<SushiSwapReader>,
    pub curve: Option<CurveReader>,
    pub balancer: Option<BalancerReader>,
}

impl EthPrices {
    pub fn new(shared_provider: Option<Arc<crate::utils::EthPriceProviderFactory>>) -> Self {
        Self {
            db_config: DatabaseConfig::default_mainnet(),
            shared_provider,
            uniswap_v2: None,
            uniswap_v3: None,
            chainlink: None,
            sushiswap: None,
            curve: None,
            balancer: None,
        }
    }

    pub fn mainnet() -> Self {
        Self::new(None)
    }

    pub fn from_provider(provider: Arc<crate::utils::EthPriceProviderFactory>) -> Self {
        Self::new(Some(provider))
    }

    pub fn with_uniswap_v2(mut self) -> Result<Self, PriceError> {
        self.uniswap_v2 = Some(if let Some(provider) = &self.shared_provider {
            UniswapV2Reader::from_provider(provider.clone())?
        } else {
            UniswapV2Reader::new(&self.db_config.reth_db_path)?
        });
        Ok(self)
    }

    pub fn with_uniswap_v3(mut self) -> Result<Self, PriceError> {
        self.uniswap_v3 = Some(if let Some(provider) = &self.shared_provider {
            UniswapV3Reader::from_provider(provider.clone())?
        } else {
            UniswapV3Reader::new(&self.db_config.reth_db_path)?
        });
        Ok(self)
    }

    pub fn with_chainlink(mut self) -> Result<Self, PriceError> {
        self.chainlink = Some(if let Some(provider) = &self.shared_provider {
            ChainlinkReader::from_provider(provider.clone())?
        } else {
            ChainlinkReader::new(&self.db_config.reth_db_path)?
        });
        Ok(self)
    }

    pub fn with_sushiswap(mut self) -> Result<Self, PriceError> {
        self.sushiswap = Some(if let Some(provider) = &self.shared_provider {
            SushiSwapReader::from_provider(provider.clone())?
        } else {
            SushiSwapReader::new(&self.db_config.reth_db_path)?
        });
        Ok(self)
    }

    pub fn with_curve(mut self) -> Result<Self, PriceError> {
        self.curve = Some(if let Some(provider) = &self.shared_provider {
            CurveReader::from_provider(provider.clone())?
        } else {
            CurveReader::new(&self.db_config.reth_db_path)?
        });
        Ok(self)
    }

    pub fn with_balancer(mut self) -> Result<Self, PriceError> {
        self.balancer = Some(if let Some(provider) = &self.shared_provider {
            BalancerReader::from_provider(provider.clone())?
        } else {
            BalancerReader::new(&self.db_config.reth_db_path)?
        });
        Ok(self)
    }

    pub fn get_uniswap_v2_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        match &self.uniswap_v2 {
            Some(reader) => reader.get_latest_price(pair),
            None => Err(PriceError::ReaderNotInitialized("UniswapV2".to_string())),
        }
    }

    pub fn get_uniswap_v3_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        match &self.uniswap_v3 {
            Some(reader) => reader.get_latest_price(pair),
            None => Err(PriceError::ReaderNotInitialized("UniswapV3".to_string())),
        }
    }

    pub async fn get_chainlink_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        match &self.chainlink {
            Some(reader) => reader.get_latest_price(pair).await,
            None => Err(PriceError::ReaderNotInitialized("Chainlink".to_string())),
        }
    }

    pub async fn get_chainlink_price_at_block(
        &self,
        pair: &str,
        block_number: u64,
    ) -> Result<PriceData, PriceError> {
        match &self.chainlink {
            Some(reader) => reader.get_price_at_block(pair, block_number).await,
            None => Err(PriceError::ReaderNotInitialized("Chainlink".to_string())),
        }
    }

    pub fn get_sushiswap_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        match &self.sushiswap {
            Some(reader) => reader.get_latest_price(pair),
            None => Err(PriceError::ReaderNotInitialized("SushiSwap".to_string())),
        }
    }

    pub async fn get_curve_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        match &self.curve {
            Some(reader) => reader.get_latest_price(pair).await,
            None => Err(PriceError::ReaderNotInitialized("Curve".to_string())),
        }
    }

    pub async fn get_balancer_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        match &self.balancer {
            Some(reader) => reader.get_latest_price(pair).await,
            None => Err(PriceError::ReaderNotInitialized("Balancer".to_string())),
        }
    }

    pub async fn get_all_prices(&self) -> HashMap<String, Vec<PriceData>> {
        let mut all_prices = HashMap::new();

        if let Some(reader) = &self.uniswap_v2 {
            let mut prices = Vec::new();
            for pair in reader.supported_pairs() {
                if let Ok(price_data) = reader.get_latest_price(&pair) {
                    prices.push(price_data);
                }
            }
            if !prices.is_empty() {
                all_prices.insert("UniswapV2".to_string(), prices);
            }
        }

        if let Some(reader) = &self.uniswap_v3 {
            let mut prices = Vec::new();
            for pair in reader.supported_pairs() {
                if let Ok(price_data) = reader.get_latest_price(&pair) {
                    prices.push(price_data);
                }
            }
            if !prices.is_empty() {
                all_prices.insert("UniswapV3".to_string(), prices);
            }
        }

        if let Some(reader) = &self.chainlink {
            let mut prices = Vec::new();
            for pair in reader.supported_pairs() {
                if let Ok(price_data) = reader.get_latest_price(&pair).await {
                    prices.push(price_data);
                }
            }
            if !prices.is_empty() {
                all_prices.insert("Chainlink".to_string(), prices);
            }
        }

        if let Some(reader) = &self.sushiswap {
            let mut prices = Vec::new();
            for pair in reader.supported_pairs() {
                if let Ok(price_data) = reader.get_latest_price(&pair) {
                    prices.push(price_data);
                }
            }
            if !prices.is_empty() {
                all_prices.insert("SushiSwap".to_string(), prices);
            }
        }

        if let Some(reader) = &self.curve {
            let mut prices = Vec::new();
            for pair in reader.supported_pairs() {
                if let Ok(price_data) = reader.get_latest_price(&pair).await {
                    prices.push(price_data);
                }
            }
            if !prices.is_empty() {
                all_prices.insert("Curve".to_string(), prices);
            }
        }

        if let Some(reader) = &self.balancer {
            let mut prices = Vec::new();
            for pair in reader.supported_pairs() {
                if let Ok(price_data) = reader.get_latest_price(&pair).await {
                    prices.push(price_data);
                }
            }
            if !prices.is_empty() {
                all_prices.insert("Balancer".to_string(), prices);
            }
        }

        all_prices
    }
}
