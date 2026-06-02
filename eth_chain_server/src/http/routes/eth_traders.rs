use std::convert::Infallible;

use eth_risk_atlas::EthTraderListParams;
use serde::Deserialize;
use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EthTraderListQuery {
    mode: Option<String>,
    tab: Option<String>,
    sort: Option<String>,
    #[serde(alias = "min_scam_ratio")]
    min_scam_ratio: Option<f64>,
    #[serde(alias = "min_trades")]
    min_trades: Option<i64>,
    page: Option<i64>,
    #[serde(alias = "page_size")]
    page_size: Option<i64>,
    mechanism: Option<String>,
    label: Option<String>,
    role: Option<String>,
}

impl From<EthTraderListQuery> for EthTraderListParams {
    fn from(query: EthTraderListQuery) -> Self {
        Self {
            mode: query.mode.or(query.tab),
            sort: query.sort,
            min_scam_ratio: query.min_scam_ratio,
            min_trades: query.min_trades,
            page: query.page,
            page_size: query.page_size,
            mechanism: query.mechanism,
            label: query.label,
            role: query.role,
        }
    }
}

pub(super) async fn list(
    query: EthTraderListQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.risk_atlas.eth_traders(query.into()).await {
        Ok(Some(payload)) => Ok(json_response(&payload, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "token PnL has no calculation run",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load ETH traders: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn detail(
    address: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let address = address.trim();
    if address.is_empty() {
        return Ok(error_response(
            "address is required",
            StatusCode::BAD_REQUEST,
        ));
    }

    match state.risk_atlas.eth_trader_profile(address).await {
        Ok(Some(payload)) => Ok(json_response(&payload, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "ETH trader address not found",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load ETH trader profile: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn trade(
    address: String,
    pool_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let address = address.trim();
    let pool_id = pool_id.trim();
    if address.is_empty() || pool_id.is_empty() {
        return Ok(error_response(
            "address and poolId are required",
            StatusCode::BAD_REQUEST,
        ));
    }

    match state.risk_atlas.eth_trader_trade(address, pool_id).await {
        Ok(Some(payload)) => Ok(json_response(&payload, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "ETH trader trade position not found",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load ETH trader trade position: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
