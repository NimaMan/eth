use crate::data::pairs::{PairRequest, PairSources};
use crate::env_core::EnvError;
use chrono::{DateTime, Utc};
use eth_prices::core::PriceData;
use eth_prices::price_readers::snapshot::{SushiSwapReader, UniswapV2Reader, UniswapV3Reader};
use eth_prices::{ChainlinkReader, StablecoinPriceReader};
use reth_chain_query::BlockTimeConverter;
use reth_db::mdbx::DatabaseEnv;
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_provider::{BlockNumReader, HeaderProvider, ProviderFactory};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tx_simulator::TxSimulator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSnapshot {
    pub timestamp_unix: i64,
    pub block_number: u64,
    pub base_fee_per_gas: u64,
    pub chainlink_price: PriceData,
    pub eth_usdc_prices: HashMap<String, PriceData>,
    pub eth_usdt_prices: HashMap<String, PriceData>,
    pub eth_dai_prices: HashMap<String, PriceData>,
    pub pair_prices: HashMap<String, PriceData>,
}

pub struct PriceFeeds {
    provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    chainlink: ChainlinkReader,
    stablecoins: StablecoinPriceReader,
    time_converter: Arc<BlockTimeConverter>,
    pair_sources: PairSources,
}

impl PriceFeeds {
    pub fn new(reth_datadir: &str) -> Result<Self, EnvError> {
        let simulator = Arc::new(
            TxSimulator::new(reth_datadir)
                .map_err(|e| EnvError::data(format!("simulator init failed: {e}")))?,
        );
        let provider_factory = simulator.provider_factory().clone();
        let provider_arc = Arc::new(provider_factory.clone());
        let chainlink = ChainlinkReader::from_provider(provider_arc.clone())
            .map_err(|e| EnvError::data(format!("chainlink init failed: {e}")))?;
        let stablecoins = StablecoinPriceReader::from_provider(provider_arc.clone())
            .map_err(|e| EnvError::data(format!("stablecoin reader init failed: {e}")))?;
        let uniswap_v2 = UniswapV2Reader::from_provider(provider_arc.clone())
            .map_err(|e| EnvError::data(format!("uniswap_v2 reader init failed: {e}")))?;
        let uniswap_v3 = UniswapV3Reader::from_provider(provider_arc.clone())
            .map_err(|e| EnvError::data(format!("uniswap_v3 reader init failed: {e}")))?;
        let sushiswap = SushiSwapReader::from_provider(provider_arc.clone())
            .map_err(|e| EnvError::data(format!("sushiswap reader init failed: {e}")))?;
        let pair_sources = PairSources::new(uniswap_v2, uniswap_v3, sushiswap);
        let time_converter = Arc::new(BlockTimeConverter::new(simulator));
        Ok(Self {
            provider_factory,
            chainlink,
            stablecoins,
            time_converter,
            pair_sources,
        })
    }

    pub fn latest_block_number(&self) -> Result<u64, EnvError> {
        self.provider_factory
            .last_block_number()
            .map_err(|e| EnvError::data(format!("latest block lookup failed: {e}")))
    }

    pub async fn fetch_snapshot(
        &self,
        pair_requests: Option<&[PairRequest]>,
    ) -> Result<FeedSnapshot, EnvError> {
        let latest_block = self.latest_block_number()?;
        self.fetch_snapshot_at_block(latest_block, pair_requests)
            .await
    }

    pub async fn fetch_snapshot_at_block(
        &self,
        block_number: u64,
        pair_requests: Option<&[PairRequest]>,
    ) -> Result<FeedSnapshot, EnvError> {
        let chainlink_price = self.chainlink_price_at_block(block_number).await?;
        let eth_usdc = self.stablecoins.eth_usdc_at_block(block_number);
        let eth_dai = self.stablecoins.eth_dai_at_block(block_number);
        let eth_usdt = self.stablecoins.eth_usdt_at_block(block_number).await;

        let timestamp_unix = chainlink_price.timestamp as i64;

        let base_fee_per_gas = self
            .provider_factory
            .provider()
            .map_err(|e| EnvError::data(format!("provider error: {e}")))?
            .header_by_number(block_number)
            .map_err(|e| EnvError::data(format!("header lookup failed: {e}")))?
            .and_then(|header| header.base_fee_per_gas)
            .unwrap_or(0);

        let pair_prices = if let Some(requests) = pair_requests {
            self.pair_sources.prices_at_block(requests, block_number)?
        } else {
            HashMap::new()
        };

        Ok(FeedSnapshot {
            timestamp_unix,
            block_number,
            base_fee_per_gas,
            chainlink_price,
            eth_usdc_prices: eth_usdc,
            eth_usdt_prices: eth_usdt,
            eth_dai_prices: eth_dai,
            pair_prices,
        })
    }

    pub async fn chainlink_price_at_block(&self, block_number: u64) -> Result<PriceData, EnvError> {
        self.chainlink
            .get_price_at_block("ETH/USD", block_number)
            .await
            .map_err(|e| EnvError::data(format!("chainlink fetch failed: {e}")))
    }

    pub async fn block_at_timestamp(&self, timestamp_unix: i64) -> Result<u64, EnvError> {
        let timestamp = DateTime::<Utc>::from_timestamp(timestamp_unix, 0).ok_or_else(|| {
            EnvError::data(format!("invalid unix timestamp provided: {timestamp_unix}"))
        })?;
        self.time_converter
            .timestamp_to_block(timestamp)
            .await
            .map_err(|e| EnvError::data(format!("timestamp conversion failed: {e}")))
    }

    pub async fn fetch_snapshot_at_timestamp(
        &self,
        timestamp_unix: i64,
        pair_requests: Option<&[PairRequest]>,
    ) -> Result<FeedSnapshot, EnvError> {
        let block = self.block_at_timestamp(timestamp_unix).await?;
        self.fetch_snapshot_at_block(block, pair_requests).await
    }
}
