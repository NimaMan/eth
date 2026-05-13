use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::read_models as views;
use crate::read_models::activity::{TokenActivityBlocksQuery, TokenActivityError};

pub(super) async fn activity_blocks(
    token_address: String,
    query: TokenActivityBlocksQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match views::activity::token_activity_blocks(state.provider.as_ref(), &token_address, query) {
        Ok(response) => Ok(json_response(&response, StatusCode::OK)),
        Err(TokenActivityError::InvalidAddress(message)) => {
            Ok(error_response(message, StatusCode::BAD_REQUEST))
        }
        Err(TokenActivityError::IndexUnavailable(message)) => {
            Ok(error_response(message, StatusCode::SERVICE_UNAVAILABLE))
        }
        Err(TokenActivityError::Lookup(message)) => {
            Ok(error_response(message, StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}
