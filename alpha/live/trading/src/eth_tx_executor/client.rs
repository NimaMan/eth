use serde::de::DeserializeOwned;

use super::{
    policy_journal::EthTxPolicyDecisionList,
    wire::{
        EthTxExecutorStatus, EthTxSubmitDirectRawResult, EthTxSubmitTransactionRequest,
        EthTxSubmitTransactionResult, LiveDirectRawTransactionRequest,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EthTxExecutorClientConfig {
    pub base_url: String,
    pub bearer_token: String,
}

impl EthTxExecutorClientConfig {
    pub fn new(base_url: impl Into<String>, bearer_token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            bearer_token: bearer_token.into(),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

#[derive(Clone)]
pub struct EthTxExecutorClient {
    http: reqwest::Client,
    config: EthTxExecutorClientConfig,
}

impl EthTxExecutorClient {
    pub fn new(config: EthTxExecutorClientConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    pub fn with_http_client(http: reqwest::Client, config: EthTxExecutorClientConfig) -> Self {
        Self { http, config }
    }

    pub async fn eth_tx_status(&self) -> Result<EthTxExecutorStatus, EthTxExecutorClientError> {
        self.get_json("/eth/tx/status").await
    }

    pub async fn submit_direct_raw(
        &self,
        request: &LiveDirectRawTransactionRequest,
    ) -> Result<EthTxSubmitDirectRawResult, EthTxExecutorClientError> {
        self.require_token()?;
        let response = self
            .http
            .post(self.config.endpoint("/eth/tx/direct-raw"))
            .bearer_auth(self.config.bearer_token.trim())
            .json(request)
            .send()
            .await?;
        decode_response(response).await
    }

    pub async fn submit_transaction(
        &self,
        request: &EthTxSubmitTransactionRequest,
    ) -> Result<EthTxSubmitTransactionResult, EthTxExecutorClientError> {
        self.require_token()?;
        let response = self
            .http
            .post(self.config.endpoint("/eth/tx/submit"))
            .bearer_auth(self.config.bearer_token.trim())
            .json(request)
            .send()
            .await?;
        decode_response(response).await
    }

    pub async fn policy_decisions_for_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<EthTxPolicyDecisionList, EthTxExecutorClientError> {
        let path = format!("/eth/tx/policy/decisions/{attempt_id}");
        self.get_json(&path).await
    }

    pub async fn recent_policy_decisions(
        &self,
        limit: usize,
    ) -> Result<EthTxPolicyDecisionList, EthTxExecutorClientError> {
        let path = format!("/eth/tx/policy/decisions?limit={limit}");
        self.get_json(&path).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<T, EthTxExecutorClientError>
    where
        T: DeserializeOwned,
    {
        self.require_token()?;
        let response = self
            .http
            .get(self.config.endpoint(path))
            .bearer_auth(self.config.bearer_token.trim())
            .send()
            .await?;
        decode_response(response).await
    }

    fn require_token(&self) -> Result<(), EthTxExecutorClientError> {
        if self.config.bearer_token.trim().is_empty() {
            Err(EthTxExecutorClientError::Config(
                "ETH tx executor bearer token is empty".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

async fn decode_response<T>(response: reqwest::Response) -> Result<T, EthTxExecutorClientError>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(EthTxExecutorClientError::Server(EthTxExecutorServerError {
            status: status.as_u16(),
            body,
        }));
    }

    response
        .json::<T>()
        .await
        .map_err(EthTxExecutorClientError::Http)
}

#[derive(Debug, thiserror::Error)]
pub enum EthTxExecutorClientError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("ETH tx executor HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("ETH tx executor server error: {0}")]
    Server(EthTxExecutorServerError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EthTxExecutorServerError {
    pub status: u16,
    pub body: String,
}

impl std::fmt::Display for EthTxExecutorServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {}: {}", self.status, self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_join_removes_trailing_slash() {
        let config = EthTxExecutorClientConfig::new("http://127.0.0.1:5006/", "token");

        assert_eq!(
            config.endpoint("/eth/tx/status"),
            "http://127.0.0.1:5006/eth/tx/status"
        );
    }
}
