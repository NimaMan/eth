use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{Address, U256};
use eth_price::utils::price_ticker_mapping::{PriceTickerMapper, Protocol as PriceTickerProtocol};
use eth_price::{
    BalancerReader, BuySwapReader, ChainlinkReader, CurveReader, DODOReader, FraxswapReader,
    PancakeswapV3Reader, PriceData, StablecoinPriceReader, SushiSwapReader, UniswapV2Reader,
    UniswapV3Reader,
};
use eyre::{eyre, Result};
use serde::Deserialize;
use tx_simulator::tx_builders::amm_swap_route::AmmSwapRoute;

use super::cache::{PriceCache, PriceCacheKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriceVenue {
    UniswapV2,
    UniswapV3,
    SushiSwap,
    Curve,
    Balancer,
    Chainlink,
    Dodo,
    Fraxswap,
    PancakeswapV3,
}

impl PriceVenue {
    pub fn parse(value: &str) -> Result<Self> {
        match normalize_venue(value).as_str() {
            "uniswapv2" | "univ2" => Ok(Self::UniswapV2),
            "uniswapv3" | "univ3" => Ok(Self::UniswapV3),
            "sushiswap" | "sushiswapv2" | "sushi" => Ok(Self::SushiSwap),
            "curve" => Ok(Self::Curve),
            "balancer" | "balancerv2" => Ok(Self::Balancer),
            "chainlink" | "oracle" => Ok(Self::Chainlink),
            "dodo" => Ok(Self::Dodo),
            "fraxswap" => Ok(Self::Fraxswap),
            "pancakeswapv3" | "pancakev3" => Ok(Self::PancakeswapV3),
            _ => Err(eyre!("unsupported price venue '{}'", value)),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UniswapV2 => "uniswap_v2",
            Self::UniswapV3 => "uniswap_v3",
            Self::SushiSwap => "sushiswap",
            Self::Curve => "curve",
            Self::Balancer => "balancer",
            Self::Chainlink => "chainlink",
            Self::Dodo => "dodo",
            Self::Fraxswap => "fraxswap",
            Self::PancakeswapV3 => "pancakeswap_v3",
        }
    }

    fn mapper_protocol(self) -> Option<PriceTickerProtocol> {
        match self {
            Self::UniswapV2 => Some(PriceTickerProtocol::UniswapV2),
            Self::UniswapV3 => Some(PriceTickerProtocol::UniswapV3),
            Self::SushiSwap => Some(PriceTickerProtocol::SushiSwap),
            Self::Curve => Some(PriceTickerProtocol::Curve),
            Self::Balancer => Some(PriceTickerProtocol::Balancer),
            Self::Chainlink => Some(PriceTickerProtocol::Chainlink),
            Self::Dodo | Self::Fraxswap | Self::PancakeswapV3 => None,
        }
    }
}

#[derive(Clone)]
pub struct ChainPriceService {
    uniswap_v2: Arc<UniswapV2Reader>,
    uniswap_v3: Arc<UniswapV3Reader>,
    sushiswap: Arc<SushiSwapReader>,
    curve: Arc<CurveReader>,
    balancer: Arc<BalancerReader>,
    chainlink: Arc<ChainlinkReader>,
    dodo: Arc<DODOReader>,
    fraxswap: Arc<FraxswapReader>,
    pancakeswap_v3: Arc<PancakeswapV3Reader>,
    stablecoins: Arc<StablecoinPriceReader>,
    buy_swap: Arc<BuySwapReader>,
    mapper: Arc<PriceTickerMapper>,
    cache: Arc<PriceCache>,
}

impl ChainPriceService {
    pub fn new(provider_factory: Arc<eth_price::utils::EthPriceProviderFactory>) -> Result<Self> {
        Ok(Self {
            uniswap_v2: Arc::new(UniswapV2Reader::from_provider(provider_factory.clone())?),
            uniswap_v3: Arc::new(UniswapV3Reader::from_provider(provider_factory.clone())?),
            sushiswap: Arc::new(SushiSwapReader::from_provider(provider_factory.clone())?),
            curve: Arc::new(CurveReader::from_provider(provider_factory.clone())?),
            balancer: Arc::new(BalancerReader::from_provider(provider_factory.clone())?),
            chainlink: Arc::new(ChainlinkReader::from_provider(provider_factory.clone())?),
            dodo: Arc::new(DODOReader::from_provider(provider_factory.clone())?),
            fraxswap: Arc::new(FraxswapReader::from_provider(provider_factory.clone())?),
            pancakeswap_v3: Arc::new(PancakeswapV3Reader::from_provider(
                provider_factory.clone(),
            )?),
            stablecoins: Arc::new(StablecoinPriceReader::from_provider(
                provider_factory.clone(),
            )?),
            buy_swap: Arc::new(BuySwapReader::from_provider(provider_factory)?),
            mapper: Arc::new(PriceTickerMapper::new()),
            cache: Arc::new(PriceCache::default()),
        })
    }

    pub async fn spot(
        &self,
        venue: PriceVenue,
        pair: &str,
        block: Option<u64>,
    ) -> Result<SpotPriceResult> {
        let pair = pair.trim();
        if pair.is_empty() {
            return Err(eyre!("pair must not be empty"));
        }

        let cache_key = PriceCacheKey::new(venue.as_str(), pair, block);
        let price = match self.cache.get(&cache_key).await {
            Some(price) => price,
            None => {
                let price = self.fetch_spot(venue, pair, block).await?;
                self.cache.insert(cache_key, price.clone()).await;
                price
            }
        };

        Ok(SpotPriceResult { venue, price })
    }

    pub async fn multi(
        &self,
        pair: &str,
        block: Option<u64>,
        venues: Option<&str>,
    ) -> Result<MultiPriceResult> {
        let pair = pair.trim();
        if pair.is_empty() {
            return Err(eyre!("pair must not be empty"));
        }

        let venues = parse_venues(venues)?;
        let mut results = Vec::with_capacity(venues.len());
        for venue in venues {
            match self.spot(venue, pair, block).await {
                Ok(result) => results.push((venue, Ok(result.price))),
                Err(error) => results.push((venue, Err(error.to_string()))),
            }
        }

        Ok(MultiPriceResult {
            pair: pair.to_string(),
            block,
            results,
        })
    }

    pub async fn stablecoins(&self, block: Option<u64>) -> Result<StablecoinPriceResult> {
        let prices = match block {
            Some(block) => self.stablecoins.eth_stablecoin_prices_at_block(block).await,
            None => self.stablecoins.eth_stablecoin_prices().await,
        };

        let mut by_quote = BTreeMap::new();
        for (quote, venue_prices) in prices {
            by_quote.insert(
                quote.to_string(),
                venue_prices
                    .into_iter()
                    .map(|(venue, price)| (venue, price))
                    .collect(),
            );
        }

        Ok(StablecoinPriceResult { block, by_quote })
    }

    pub async fn swap_quote(
        &self,
        request: SwapQuoteRequest,
    ) -> Result<eth_price::dex::swap_sim::SwapQuote> {
        let route = request.route()?;
        let token_out = parse_address(&request.token_out, "token_out")?;
        let buyer = parse_address(&request.buyer, "buyer")?;
        let eth_in_wei = U256::from_str(request.eth_in_wei.trim())
            .map_err(|error| eyre!("invalid eth_in_wei '{}': {error}", request.eth_in_wei))?;

        self.buy_swap
            .quote_eth_for_token(route, token_out, eth_in_wei, buyer, request.block)
            .await
            .map_err(|error| eyre!(error.to_string()))
    }

    async fn fetch_spot(
        &self,
        venue: PriceVenue,
        pair: &str,
        block: Option<u64>,
    ) -> Result<PriceData> {
        let protocol_pairs = self.protocol_pairs(venue, pair);
        let mut last_error = None;

        for protocol_pair in protocol_pairs {
            let result = match venue {
                PriceVenue::UniswapV2 => match block {
                    Some(block) => self.uniswap_v2.get_price_at_block(&protocol_pair, block),
                    None => self.uniswap_v2.get_latest_price(&protocol_pair),
                },
                PriceVenue::UniswapV3 => match block {
                    Some(block) => self.uniswap_v3.get_price_at_block(&protocol_pair, block),
                    None => self.uniswap_v3.get_latest_price(&protocol_pair),
                },
                PriceVenue::SushiSwap => match block {
                    Some(block) => self.sushiswap.get_price_at_block(&protocol_pair, block),
                    None => self.sushiswap.get_latest_price(&protocol_pair),
                },
                PriceVenue::Curve => match block {
                    Some(block) => self.curve.get_price_at_block(&protocol_pair, block).await,
                    None => self.curve.get_latest_price(&protocol_pair).await,
                },
                PriceVenue::Balancer => match block {
                    Some(block) => {
                        self.balancer
                            .get_price_at_block(&protocol_pair, block)
                            .await
                    }
                    None => self.balancer.get_latest_price(&protocol_pair).await,
                },
                PriceVenue::Chainlink => match block {
                    Some(block) => {
                        self.chainlink
                            .get_price_at_block(&protocol_pair, block)
                            .await
                    }
                    None => self.chainlink.get_latest_price(&protocol_pair).await,
                },
                PriceVenue::Dodo => match block {
                    Some(block) => self.dodo.get_price_at_block(&protocol_pair, block).await,
                    None => self.dodo.get_latest_price(&protocol_pair).await,
                },
                PriceVenue::Fraxswap => match block {
                    Some(block) => {
                        self.fraxswap
                            .get_price_at_block(&protocol_pair, block)
                            .await
                    }
                    None => self.fraxswap.get_latest_price(&protocol_pair).await,
                },
                PriceVenue::PancakeswapV3 => match block {
                    Some(block) => {
                        self.pancakeswap_v3
                            .get_price_at_block(&protocol_pair, block)
                            .await
                    }
                    None => self.pancakeswap_v3.get_latest_price(&protocol_pair).await,
                },
            };

            match result {
                Ok(price) => return Ok(price),
                Err(error) => last_error = Some(error.to_string()),
            }
        }

        Err(eyre!(
            "price not available for pair '{}' on {}{}",
            pair,
            venue.as_str(),
            last_error
                .map(|error| format!(": {error}"))
                .unwrap_or_default()
        ))
    }

    fn protocol_pairs(&self, venue: PriceVenue, pair: &str) -> Vec<String> {
        let Some(protocol) = venue.mapper_protocol() else {
            return vec![pair.to_string()];
        };
        self.mapper
            .get_protocol_pairs(pair, protocol)
            .unwrap_or_else(|_| vec![pair.to_string()])
    }
}

pub struct SpotPriceResult {
    pub venue: PriceVenue,
    pub price: PriceData,
}

pub struct MultiPriceResult {
    pub pair: String,
    pub block: Option<u64>,
    pub results: Vec<(PriceVenue, std::result::Result<PriceData, String>)>,
}

pub struct StablecoinPriceResult {
    pub block: Option<u64>,
    pub by_quote: BTreeMap<String, BTreeMap<String, PriceData>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwapQuoteRequest {
    pub protocol: String,
    pub pool: String,
    pub token_out: String,
    pub eth_in_wei: String,
    pub buyer: String,
    pub block: Option<u64>,
    pub fee_tier: Option<u32>,
    pub router: Option<String>,
}

impl SwapQuoteRequest {
    fn route(&self) -> Result<AmmSwapRoute> {
        let pool = parse_address(&self.pool, "pool")?;
        match normalize_venue(&self.protocol).as_str() {
            "uniswapv2" | "univ2" => Ok(AmmSwapRoute::UniswapV2 { pool }),
            "sushiswap" | "sushiswapv2" | "sushi" => Ok(AmmSwapRoute::SushiswapV2 { pool }),
            "v2router" => Ok(AmmSwapRoute::V2Router {
                pool,
                router: parse_required_router(self.router.as_deref())?,
            }),
            "uniswapv3" | "univ3" => Ok(AmmSwapRoute::UniswapV3 {
                pool,
                fee_tier: required_fee_tier(self.fee_tier)?,
            }),
            "sushiswapv3" => Ok(AmmSwapRoute::SushiswapV3 {
                pool,
                fee_tier: required_fee_tier(self.fee_tier)?,
            }),
            "pancakeswapv3" | "pancakev3" => Ok(AmmSwapRoute::PancakeSwapV3 {
                pool,
                fee_tier: required_fee_tier(self.fee_tier)?,
            }),
            "v3router" => Ok(AmmSwapRoute::V3Router {
                pool,
                router: parse_required_router(self.router.as_deref())?,
                fee_tier: required_fee_tier(self.fee_tier)?,
            }),
            protocol => Err(eyre!(
                "unsupported swap quote protocol '{}'; supported protocols are uniswap_v2, sushiswap_v2, v2_router, uniswap_v3, sushiswap_v3, pancakeswap_v3, v3_router",
                protocol
            )),
        }
    }
}

fn parse_venues(venues: Option<&str>) -> Result<Vec<PriceVenue>> {
    let Some(venues) = venues else {
        return Ok(vec![
            PriceVenue::UniswapV2,
            PriceVenue::UniswapV3,
            PriceVenue::SushiSwap,
            PriceVenue::Curve,
            PriceVenue::Balancer,
            PriceVenue::Chainlink,
        ]);
    };

    let parsed: Result<Vec<_>> = venues
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PriceVenue::parse)
        .collect();
    let parsed = parsed?;
    if parsed.is_empty() {
        return Err(eyre!("venues must contain at least one venue"));
    }
    Ok(parsed)
}

fn parse_required_router(router: Option<&str>) -> Result<Address> {
    let Some(router) = router else {
        return Err(eyre!("router is required for explicit router swap routes"));
    };
    parse_address(router, "router")
}

fn required_fee_tier(fee_tier: Option<u32>) -> Result<u32> {
    fee_tier.ok_or_else(|| eyre!("fee_tier is required for v3 swap routes"))
}

fn parse_address(value: &str, field: &str) -> Result<Address> {
    Address::from_str(value.trim()).map_err(|error| eyre!("invalid {field} address: {error}"))
}

fn normalize_venue(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '_' && *ch != '-' && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
