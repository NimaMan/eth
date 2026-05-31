use axum::{http::StatusCode, response::IntoResponse};
use std::fmt;

#[derive(Debug)]
pub enum EthTxServiceError {
    Validation(String),
    Auth(String),
    Network(String),
    Internal(String),
    Unavailable(String),
    NotFound(String),
}

impl EthTxServiceError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn unavailable(msg: impl Into<String>) -> Self {
        Self::Unavailable(msg.into())
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Auth(_) => StatusCode::UNAUTHORIZED,
            Self::Network(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}

impl fmt::Display for EthTxServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
            Self::Auth(msg) => write!(f, "auth error: {msg}"),
            Self::Network(msg) => write!(f, "network error: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::Unavailable(msg) => write!(f, "unavailable: {msg}"),
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
        }
    }
}

impl std::error::Error for EthTxServiceError {}

impl From<anyhow::Error> for EthTxServiceError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl IntoResponse for EthTxServiceError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let body = axum::Json(serde_json::json!({
            "error": self.to_string()
        }));
        (status, body).into_response()
    }
}
