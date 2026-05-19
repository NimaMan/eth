use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::read_models::simulation::{self, VaultSimulationRequest};

pub(super) async fn vault_uniswap_v2(
    request: VaultSimulationRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match simulation::simulate_uniswap_v2_trading_vault_wallet_token(
        state.provider.simulator().as_ref(),
        request,
    )
    .await
    {
        Ok(report) => Ok(json_response(&report, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("vault simulation failed: {error}"),
            simulation_error_status(&error),
        )),
    }
}

fn simulation_error_status(error: &eyre::Report) -> StatusCode {
    let message = error.to_string();
    if message.contains("invalid ")
        || message.contains("below requested transfer amount")
        || message.contains("must")
    {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
