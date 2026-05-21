use std::convert::Infallible;

use serde::Deserialize;
use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::prices::{PriceVenue, SwapQuoteRequest};
use crate::read_models::price::{
    self, MultiPriceResponse, SpotPriceResponse, StablecoinPriceResponse,
};

#[derive(Debug, Deserialize)]
pub(super) struct SpotPriceQuery {
    pair: String,
    venue: String,
    block: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct MultiPriceQuery {
    pair: String,
    block: Option<u64>,
    venues: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct StablecoinPriceQuery {
    block: Option<u64>,
}

pub(super) async fn spot(
    query: SpotPriceQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let venue = match PriceVenue::parse(&query.venue) {
        Ok(venue) => venue,
        Err(error) => return Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST)),
    };

    match state
        .price_service
        .spot(venue, &query.pair, query.block)
        .await
    {
        Ok(result) => Ok(json_response(
            &SpotPriceResponse::from(result),
            StatusCode::OK,
        )),
        Err(error) => Ok(error_response(
            format!("price lookup failed: {error}"),
            price_error_status(&error),
        )),
    }
}

pub(super) async fn multi(
    query: MultiPriceQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .price_service
        .multi(&query.pair, query.block, query.venues.as_deref())
        .await
    {
        Ok(result) => Ok(json_response(
            &MultiPriceResponse::from(result),
            StatusCode::OK,
        )),
        Err(error) => Ok(error_response(
            format!("multi-price lookup failed: {error}"),
            price_error_status(&error),
        )),
    }
}

pub(super) async fn stablecoins(
    query: StablecoinPriceQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.price_service.stablecoins(query.block).await {
        Ok(result) => Ok(json_response(
            &StablecoinPriceResponse::from(result),
            StatusCode::OK,
        )),
        Err(error) => Ok(error_response(
            format!("stablecoin price lookup failed: {error}"),
            price_error_status(&error),
        )),
    }
}

pub(super) async fn swap_quote(
    request: SwapQuoteRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.price_service.swap_quote(request.clone()).await {
        Ok(quote) => Ok(json_response(
            &price::swap_quote_response(request, quote),
            StatusCode::OK,
        )),
        Err(error) => Ok(error_response(
            format!("swap quote failed: {error}"),
            price_error_status(&error),
        )),
    }
}

fn price_error_status(error: &eyre::Report) -> StatusCode {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("invalid")
        || message.contains("unsupported")
        || message.contains("must")
        || message.contains("required")
        || message.contains("empty")
    {
        StatusCode::BAD_REQUEST
    } else if message.contains("not available") || message.contains("not found") {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
