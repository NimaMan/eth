use serde::de::DeserializeOwned;

use super::{
    policy_journal::KartalPolicyDecisionList,
    wire::{
        KartalEthTxExecutorStatus, KartalSignDirectRawResult, KartalSubmitDirectRawResult,
        LiveDirectRawTransactionRequest,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KartalClientConfig {
    pub base_url: String,
    pub bearer_token: String,
}

impl KartalClientConfig {
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
pub struct KartalClient {
    http: reqwest::Client,
    config: KartalClientConfig,
}

impl KartalClient {
    pub fn new(config: KartalClientConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    pub fn with_http_client(http: reqwest::Client, config: KartalClientConfig) -> Self {
        Self { http, config }
    }

    pub async fn eth_tx_status(&self) -> Result<KartalEthTxExecutorStatus, KartalClientError> {
        self.get_json("/eth/tx/status").await
    }

    pub async fn submit_direct_raw(
        &self,
        request: &LiveDirectRawTransactionRequest,
    ) -> Result<KartalSubmitDirectRawResult, KartalClientError> {
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

    pub async fn sign_direct_raw(
        &self,
        request: &LiveDirectRawTransactionRequest,
    ) -> Result<KartalSignDirectRawResult, KartalClientError> {
        self.require_token()?;
        let response = self
            .http
            .post(self.config.endpoint("/eth/tx/sign-direct-raw"))
            .bearer_auth(self.config.bearer_token.trim())
            .json(request)
            .send()
            .await?;
        decode_response(response).await
    }

    pub async fn policy_decisions_for_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<KartalPolicyDecisionList, KartalClientError> {
        let path = format!("/eth/tx/policy/decisions/{attempt_id}");
        self.get_json(&path).await
    }

    pub async fn recent_policy_decisions(
        &self,
        limit: usize,
    ) -> Result<KartalPolicyDecisionList, KartalClientError> {
        let path = format!("/eth/tx/policy/decisions?limit={limit}");
        self.get_json(&path).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<T, KartalClientError>
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

    fn require_token(&self) -> Result<(), KartalClientError> {
        if self.config.bearer_token.trim().is_empty() {
            Err(KartalClientError::Config(
                "Kartal bearer token is empty".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

async fn decode_response<T>(response: reqwest::Response) -> Result<T, KartalClientError>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(KartalClientError::Server(KartalServerError {
            status: status.as_u16(),
            body,
        }));
    }

    response.json::<T>().await.map_err(KartalClientError::Http)
}

#[derive(Debug, thiserror::Error)]
pub enum KartalClientError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("Kartal HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Kartal server error: {0}")]
    Server(KartalServerError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KartalServerError {
    pub status: u16,
    pub body: String,
}

impl std::fmt::Display for KartalServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {}: {}", self.status, self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_join_removes_trailing_slash() {
        let config = KartalClientConfig::new("http://127.0.0.1:5004/", "token");

        assert_eq!(
            config.endpoint("/eth/tx/status"),
            "http://127.0.0.1:5004/eth/tx/status"
        );
    }
}
