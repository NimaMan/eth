use eth_alpha_core::ids::{BlockNumber, PoolAddress, TokenAddress, TradeId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KartalExecutorClientConfig {
    pub base_url: String,
    pub bearer_token: String,
}

impl KartalExecutorClientConfig {
    pub fn new(base_url: impl Into<String>, bearer_token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            bearer_token: bearer_token.into(),
        }
    }

    fn direct_raw_url(&self) -> String {
        join_endpoint(&self.base_url, "/eth/tx/direct-raw")
    }
}

#[derive(Clone)]
pub struct KartalExecutorClient {
    http: reqwest::Client,
    config: KartalExecutorClientConfig,
}

impl KartalExecutorClient {
    pub fn new(config: KartalExecutorClientConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    pub fn with_http_client(http: reqwest::Client, config: KartalExecutorClientConfig) -> Self {
        Self { http, config }
    }

    pub async fn submit_signal(
        &self,
        signal: &LiveTraderTxSignal,
    ) -> Result<KartalSubmitDirectRawResult, KartalExecutorClientError> {
        self.submit_direct_raw(&signal.request_with_strategy_metadata())
            .await
    }

    pub async fn submit_direct_raw(
        &self,
        request: &LiveDirectRawTransactionRequest,
    ) -> Result<KartalSubmitDirectRawResult, KartalExecutorClientError> {
        if self.config.bearer_token.trim().is_empty() {
            return Err(KartalExecutorClientError::Config(
                "Kartal bearer token is empty".to_string(),
            ));
        }

        let response = self
            .http
            .post(self.config.direct_raw_url())
            .bearer_auth(self.config.bearer_token.trim())
            .json(request)
            .send()
            .await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(KartalExecutorClientError::Server {
                status: status.as_u16(),
                body,
            });
        }

        response
            .json::<KartalSubmitDirectRawResult>()
            .await
            .map_err(KartalExecutorClientError::Http)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KartalExecutorClientError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("Kartal HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Kartal rejected transaction request with HTTP {status}: {body}")]
    Server { status: u16, body: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveTraderTxSignal {
    pub strategy_name: String,
    pub strategy_run_id: Option<String>,
    pub trade_id: Option<TradeId>,
    pub token_address: Option<TokenAddress>,
    pub pool_address: Option<PoolAddress>,
    pub observed_block: Option<BlockNumber>,
    pub request: LiveDirectRawTransactionRequest,
}

impl LiveTraderTxSignal {
    pub fn request_with_strategy_metadata(&self) -> LiveDirectRawTransactionRequest {
        let mut request = self.request.clone();
        request.metadata = self.metadata_with_strategy_context();
        request
    }

    fn metadata_with_strategy_context(&self) -> Value {
        let mut metadata = match self.request.metadata.clone() {
            Value::Object(map) => map,
            other if other.is_null() => Map::new(),
            other => {
                let mut map = Map::new();
                map.insert("source_metadata".to_string(), other);
                map
            }
        };

        metadata.insert("strategy_name".to_string(), json!(self.strategy_name));
        if let Some(strategy_run_id) = &self.strategy_run_id {
            metadata.insert("strategy_run_id".to_string(), json!(strategy_run_id));
        }
        if let Some(trade_id) = &self.trade_id {
            metadata.insert("trade_id".to_string(), json!(trade_id.0));
        }
        if let Some(token_address) = self.token_address {
            metadata.insert(
                "token_address".to_string(),
                json!(token_address.to_string()),
            );
        }
        if let Some(pool_address) = &self.pool_address {
            metadata.insert("pool_address".to_string(), json!(pool_address.as_str()));
        }
        if let Some(observed_block) = self.observed_block {
            metadata.insert("observed_block".to_string(), json!(observed_block));
        }

        Value::Object(metadata)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveDirectRawTransactionRequest {
    pub attempt_id: Option<String>,
    pub chain_id: u64,
    pub from: String,
    pub to: String,
    #[serde(default = "zero_string")]
    pub value: String,
    #[serde(default = "empty_data")]
    pub data: String,
    pub gas_limit: String,
    pub max_fee_per_gas: String,
    pub max_priority_fee_per_gas: String,
    pub nonce: Option<String>,
    pub bribe: Option<KartalBribeRequest>,
    pub simulation: Option<KartalSimulationReference>,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KartalBribeRequest {
    pub priority_fee_per_gas: String,
    pub max_fee_per_gas: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KartalSimulationReference {
    pub block_number: u64,
    pub block_hash: Option<String>,
    pub state_root: Option<String>,
    pub expected_output_token: Option<String>,
    pub expected_output_amount: Option<String>,
    pub min_output_amount: Option<String>,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KartalSubmitDirectRawResult {
    pub attempt_id: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub from: String,
    pub to: String,
    pub nonce: Value,
    pub gas_limit: Value,
    pub max_fee_per_gas: Value,
    pub max_priority_fee_per_gas: Value,
    pub error: Option<String>,
    pub elapsed_ms: Value,
}

fn join_endpoint(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

fn zero_string() -> String {
    "0".to_string()
}

fn empty_data() -> String {
    "0x".to_string()
}

fn default_metadata() -> Value {
    json!({})
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use serde_json::json;

    use super::*;

    fn request() -> LiveDirectRawTransactionRequest {
        LiveDirectRawTransactionRequest {
            attempt_id: Some("attempt-1".to_string()),
            chain_id: 1,
            from: "0x0000000000000000000000000000000000000001".to_string(),
            to: "0x0000000000000000000000000000000000000002".to_string(),
            value: "0".to_string(),
            data: "0x".to_string(),
            gas_limit: "21000".to_string(),
            max_fee_per_gas: "1000000000".to_string(),
            max_priority_fee_per_gas: "1000000000".to_string(),
            nonce: None,
            bribe: None,
            simulation: None,
            metadata: json!({"decision": "test"}),
        }
    }

    #[test]
    fn direct_raw_endpoint_is_joined_without_double_slash() {
        let config = KartalExecutorClientConfig::new("http://127.0.0.1:5004/", "token");

        assert_eq!(
            config.direct_raw_url(),
            "http://127.0.0.1:5004/eth/tx/direct-raw"
        );
    }

    #[test]
    fn strategy_signal_adds_audit_metadata() {
        let signal = LiveTraderTxSignal {
            strategy_name: "snipe-all-risk-atlas-lp-gate-hold15".to_string(),
            strategy_run_id: Some("run-1".to_string()),
            trade_id: Some(TradeId("trade-1".to_string())),
            token_address: Some(Address::with_last_byte(0x11)),
            pool_address: Some(PoolAddress::from("0xtoken:0xpool")),
            observed_block: Some(25_110_001),
            request: request(),
        };

        let request = signal.request_with_strategy_metadata();

        assert_eq!(request.metadata["decision"], json!("test"));
        assert_eq!(
            request.metadata["strategy_name"],
            json!("snipe-all-risk-atlas-lp-gate-hold15")
        );
        assert_eq!(request.metadata["strategy_run_id"], json!("run-1"));
        assert_eq!(request.metadata["trade_id"], json!("trade-1"));
        assert_eq!(request.metadata["observed_block"], json!(25_110_001));
    }

    #[test]
    fn serde_defaults_match_kartal_direct_raw_contract() {
        let request: LiveDirectRawTransactionRequest = serde_json::from_value(json!({
            "attempt_id": null,
            "chain_id": 1,
            "from": "0x0000000000000000000000000000000000000001",
            "to": "0x0000000000000000000000000000000000000002",
            "gas_limit": "21000",
            "max_fee_per_gas": "1000000000",
            "max_priority_fee_per_gas": "1000000000",
            "nonce": null,
            "bribe": null,
            "simulation": null
        }))
        .unwrap();

        assert_eq!(request.value, "0");
        assert_eq!(request.data, "0x");
        assert_eq!(request.metadata, json!({}));
    }
}
