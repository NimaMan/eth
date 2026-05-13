use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::stores::mempool_signals::{MempoolSignalKind, MempoolSignalQuery};

pub(super) async fn signals(
    query: MempoolSignalQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .mempool_signals
        .list(MempoolSignalKind::All, query)
        .await
    {
        Ok(signals) => Ok(json_response(&signals, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load mempool signals: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn signals_by_type(
    signal_type: String,
    query: MempoolSignalQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let Some(kind) = MempoolSignalKind::from_path(&signal_type) else {
        return Ok(error_response("unknown signal type", StatusCode::NOT_FOUND));
    };

    match state.mempool_signals.list(kind, query).await {
        Ok(signals) => Ok(json_response(&signals, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load mempool signals: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
