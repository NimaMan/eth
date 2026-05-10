use serde::Serialize;
use warp::http::StatusCode;
use warp::Reply;

use crate::error::ApiError;

pub(crate) fn json_response<T: Serialize>(value: &T, status: StatusCode) -> warp::reply::Response {
    warp::reply::with_status(warp::reply::json(value), status).into_response()
}

pub(crate) fn error_response(
    message: impl Into<String>,
    status: StatusCode,
) -> warp::reply::Response {
    json_response(&ApiError::new(message), status)
}
