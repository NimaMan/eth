use std::collections::BTreeMap;

use eth_price::{
    AggregatorPriceSource, AmmPriceSource, OraclePriceSource, PriceData, PriceSource,
    SimulationDirection, SimulationPriceSource, Token,
};
use serde::Serialize;

use crate::prices::{MultiPriceResult, SpotPriceResult, StablecoinPriceResult, SwapQuoteRequest};

#[derive(Debug, Clone, Serialize)]
pub struct TokenView {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

impl From<&Token> for TokenView {
    fn from(token: &Token) -> Self {
        Self {
            address: token.address.to_string(),
            symbol: token.symbol.clone(),
            decimals: token.decimals,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RawPriceView {
    pub numerator: String,
    pub denominator: String,
    pub decimals_adjusted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceDataView {
    pub pair: String,
    pub base: TokenView,
    pub quote: TokenView,
    pub price: f64,
    pub inverse_price: f64,
    pub raw_price: RawPriceView,
    pub raw_inverse_price: RawPriceView,
    pub block_number: u64,
    pub timestamp: u64,
    pub protocol: String,
    pub source: PriceSourceView,
}

impl From<PriceData> for PriceDataView {
    fn from(price: PriceData) -> Self {
        Self {
            pair: price.pair.label.clone(),
            base: TokenView::from(&price.pair.base),
            quote: TokenView::from(&price.pair.quote),
            price: price.price_as_f64(),
            inverse_price: price.inverse_price_as_f64(),
            raw_price: RawPriceView {
                numerator: price.price.numerator.to_string(),
                denominator: price.price.denominator.to_string(),
                decimals_adjusted: price.price.decimals_adjusted,
            },
            raw_inverse_price: RawPriceView {
                numerator: price.inverse_price.numerator.to_string(),
                denominator: price.inverse_price.denominator.to_string(),
                decimals_adjusted: price.inverse_price.decimals_adjusted,
            },
            block_number: price.block_number,
            timestamp: price.timestamp,
            protocol: price.protocol().as_str().to_string(),
            source: PriceSourceView::from(price.source),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PriceSourceView {
    Amm {
        protocol: String,
        pool: String,
        fee_tier: Option<u32>,
        raw_liquidity: String,
        liquidity_tier: u8,
    },
    Oracle {
        protocol: String,
        feed: String,
        answer: String,
        answer_decimals: u8,
        round_id: Option<String>,
        updated_at: u64,
    },
    Aggregator {
        protocol: String,
        routes: Vec<String>,
        amount_in: String,
        amount_out: String,
        estimated_gas: Option<String>,
    },
    Simulation {
        protocol: String,
        direction: String,
        route: String,
        account: String,
        input_token: TokenView,
        output_token: TokenView,
        input_amount: String,
        output_amount: String,
        tax_percent: Option<f64>,
    },
}

impl From<PriceSource> for PriceSourceView {
    fn from(source: PriceSource) -> Self {
        match source {
            PriceSource::Amm(source) => amm_source_view(source),
            PriceSource::Oracle(source) => oracle_source_view(source),
            PriceSource::Aggregator(source) => aggregator_source_view(source),
            PriceSource::Simulation(source) => simulation_source_view(source),
        }
    }
}

fn amm_source_view(source: AmmPriceSource) -> PriceSourceView {
    PriceSourceView::Amm {
        protocol: source.protocol.as_str().to_string(),
        pool: source.price_id.pool_address.to_string(),
        fee_tier: source.price_id.fee_tier,
        raw_liquidity: source.liquidity.raw_liquidity.to_string(),
        liquidity_tier: source.liquidity.liquidity_tier,
    }
}

fn oracle_source_view(source: OraclePriceSource) -> PriceSourceView {
    PriceSourceView::Oracle {
        protocol: source.price_id.protocol.as_str().to_string(),
        feed: source.feed_address.to_string(),
        answer: source.answer.to_string(),
        answer_decimals: source.answer_decimals,
        round_id: source.round_id.map(|round| round.to_string()),
        updated_at: source.updated_at,
    }
}

fn aggregator_source_view(source: AggregatorPriceSource) -> PriceSourceView {
    PriceSourceView::Aggregator {
        protocol: source.protocol.as_str().to_string(),
        routes: source.routes,
        amount_in: source.amount_in.to_string(),
        amount_out: source.amount_out.to_string(),
        estimated_gas: source.estimated_gas.map(|gas| gas.to_string()),
    }
}

fn simulation_source_view(source: SimulationPriceSource) -> PriceSourceView {
    PriceSourceView::Simulation {
        protocol: source.price_id.protocol.as_str().to_string(),
        direction: match source.direction {
            SimulationDirection::Buy => "buy".to_string(),
            SimulationDirection::Sell => "sell".to_string(),
        },
        route: source.route,
        account: source.account.to_string(),
        input_token: TokenView::from(&source.input_token),
        output_token: TokenView::from(&source.output_token),
        input_amount: source.input_amount.to_string(),
        output_amount: source.output_amount.to_string(),
        tax_percent: source.tax_percent,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotPriceResponse {
    pub schema: &'static str,
    pub venue: String,
    pub price: PriceDataView,
}

impl From<SpotPriceResult> for SpotPriceResponse {
    fn from(result: SpotPriceResult) -> Self {
        Self {
            schema: "eth_chain_price_spot_v1",
            venue: result.venue.as_str().to_string(),
            price: PriceDataView::from(result.price),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MultiPriceResponse {
    pub schema: &'static str,
    pub pair: String,
    pub block: Option<u64>,
    pub successful_count: usize,
    pub median_price: Option<f64>,
    pub venues: Vec<PriceVenueResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceVenueResponse {
    pub venue: String,
    pub price: Option<PriceDataView>,
    pub error: Option<String>,
}

impl From<MultiPriceResult> for MultiPriceResponse {
    fn from(result: MultiPriceResult) -> Self {
        let mut successful_prices = Vec::new();
        let venues: Vec<PriceVenueResponse> = result
            .results
            .into_iter()
            .map(|(venue, result)| match result {
                Ok(price) => {
                    successful_prices.push(price.price_as_f64());
                    PriceVenueResponse {
                        venue: venue.as_str().to_string(),
                        price: Some(PriceDataView::from(price)),
                        error: None,
                    }
                }
                Err(error) => PriceVenueResponse {
                    venue: venue.as_str().to_string(),
                    price: None,
                    error: Some(error),
                },
            })
            .collect();
        let median_price = median(successful_prices);
        let successful_count = venues
            .iter()
            .filter(|venue: &&PriceVenueResponse| venue.price.is_some())
            .count();

        Self {
            schema: "eth_chain_price_multi_v1",
            pair: result.pair,
            block: result.block,
            successful_count,
            median_price,
            venues,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinPriceResponse {
    pub schema: &'static str,
    pub block: Option<u64>,
    pub prices: BTreeMap<String, BTreeMap<String, PriceDataView>>,
}

impl From<StablecoinPriceResult> for StablecoinPriceResponse {
    fn from(result: StablecoinPriceResult) -> Self {
        let prices = result
            .by_quote
            .into_iter()
            .map(|(quote, venue_prices)| {
                (
                    quote,
                    venue_prices
                        .into_iter()
                        .map(|(venue, price)| (venue, PriceDataView::from(price)))
                        .collect(),
                )
            })
            .collect();

        Self {
            schema: "eth_chain_price_stablecoins_v1",
            block: result.block,
            prices,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapQuoteResponse {
    pub schema: &'static str,
    pub request: SwapQuoteRequestView,
    pub ask: PriceDataView,
    pub bid: Option<PriceDataView>,
    pub tokens_acquired: String,
    pub eth_spent: String,
    pub eth_received: Option<String>,
    pub buy_tax_percent: Option<f64>,
    pub sell_tax_percent: Option<f64>,
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapQuoteRequestView {
    pub protocol: String,
    pub pool: String,
    pub token_out: String,
    pub eth_in_wei: String,
    pub buyer: String,
    pub block: Option<u64>,
    pub fee_tier: Option<u32>,
    pub router: Option<String>,
}

impl From<SwapQuoteRequest> for SwapQuoteRequestView {
    fn from(request: SwapQuoteRequest) -> Self {
        Self {
            protocol: request.protocol,
            pool: request.pool,
            token_out: request.token_out,
            eth_in_wei: request.eth_in_wei,
            buyer: request.buyer,
            block: request.block,
            fee_tier: request.fee_tier,
            router: request.router,
        }
    }
}

pub fn swap_quote_response(
    request: SwapQuoteRequest,
    quote: eth_price::dex::swap_sim::SwapQuote,
) -> SwapQuoteResponse {
    SwapQuoteResponse {
        schema: "eth_chain_price_swap_quote_v1",
        request: SwapQuoteRequestView::from(request),
        ask: PriceDataView::from(quote.ask),
        bid: quote.bid.map(PriceDataView::from),
        tokens_acquired: quote.tokens_acquired.to_string(),
        eth_spent: quote.eth_spent.to_string(),
        eth_received: quote.eth_received.map(|amount| amount.to_string()),
        buy_tax_percent: quote.buy_tax_percent,
        sell_tax_percent: quote.sell_tax_percent,
        block_number: quote.block_number,
    }
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.total_cmp(right));
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}
