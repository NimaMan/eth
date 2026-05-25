use ethers_core::utils::keccak256;
use ethers_signers::{LocalWallet, Signer};
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_FLASHBOTS_RELAY_URL: &str = "https://relay.flashbots.net";

pub struct FlashbotsMevShareClientConfig {
    pub relay_url: String,
    pub auth_private_key: String,
    pub builders: Vec<String>,
    pub privacy_hints: Vec<String>,
    pub origin_id: Option<String>,
}

impl FlashbotsMevShareClientConfig {
    pub fn new(relay_url: impl Into<String>, auth_private_key: impl Into<String>) -> Self {
        Self {
            relay_url: relay_url.into(),
            auth_private_key: auth_private_key.into(),
            builders: Vec::new(),
            privacy_hints: Vec::new(),
            origin_id: None,
        }
    }
}

#[derive(Clone)]
pub struct FlashbotsMevShareClient {
    http: reqwest::Client,
    relay_url: String,
    auth_wallet: LocalWallet,
    builders: Vec<String>,
    privacy_hints: Vec<String>,
    origin_id: Option<String>,
}

impl FlashbotsMevShareClient {
    pub fn new(config: FlashbotsMevShareClientConfig) -> Result<Self, FlashbotsClientError> {
        let auth_wallet = config
            .auth_private_key
            .trim()
            .parse::<LocalWallet>()
            .map_err(|error| FlashbotsClientError::Config(format!("invalid auth key: {error}")))?;
        Ok(Self {
            http: reqwest::Client::new(),
            relay_url: config.relay_url,
            auth_wallet,
            builders: config.builders,
            privacy_hints: config.privacy_hints,
            origin_id: config.origin_id,
        })
    }

    pub fn with_http_client(
        http: reqwest::Client,
        config: FlashbotsMevShareClientConfig,
    ) -> Result<Self, FlashbotsClientError> {
        let mut client = Self::new(config)?;
        client.http = http;
        Ok(client)
    }

    pub async fn send_tail_bundle(
        &self,
        request: FlashbotsTailBundleRequest,
    ) -> Result<FlashbotsBundleSubmission, FlashbotsClientError> {
        validate_hex_prefixed(&request.tail_after_tx_hash, "tail_after_tx_hash")?;
        validate_hex_prefixed(&request.signed_tx, "signed_tx")?;
        if request.max_block < request.target_block {
            return Err(FlashbotsClientError::Config(format!(
                "max_block {} is below target_block {}",
                request.max_block, request.target_block
            )));
        }

        let params = MevSendBundleParams {
            version: "v0.1".to_string(),
            inclusion: MevBundleInclusion {
                block: block_hex(request.target_block),
                max_block: Some(block_hex(request.max_block)),
            },
            body: vec![
                MevBundleBody::Hash {
                    hash: request.tail_after_tx_hash.clone(),
                },
                MevBundleBody::Tx {
                    tx: request.signed_tx,
                    can_revert: request.can_revert,
                },
            ],
            validity: Some(MevBundleValidity::default()),
            privacy: self.privacy(),
            metadata: self.metadata(),
        };

        let bundle_hash = self.send_bundle(params).await?.bundle_hash;
        Ok(FlashbotsBundleSubmission {
            bundle_hash,
            tail_after_tx_hash: request.tail_after_tx_hash,
            target_block: request.target_block,
            max_block: request.max_block,
        })
    }

    pub async fn send_bundle(
        &self,
        params: MevSendBundleParams,
    ) -> Result<FlashbotsBundleResult, FlashbotsClientError> {
        let body = serde_json::to_string(&JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "mev_sendBundle",
            params: vec![params],
        })?;
        let signature = self.flashbots_signature(&body).await?;
        let response = self
            .http
            .post(&self.relay_url)
            .header(CONTENT_TYPE, "application/json")
            .header("X-Flashbots-Signature", signature)
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(FlashbotsClientError::Server {
                status: status.as_u16(),
                body: text,
            });
        }

        let value: Value = serde_json::from_str(&text)?;
        if let Some(error) = value.get("error") {
            return Err(FlashbotsClientError::Rpc(error.to_string()));
        }
        let result = value.get("result").cloned().unwrap_or(value);
        serde_json::from_value::<FlashbotsBundleResult>(result)
            .map_err(FlashbotsClientError::Serialization)
    }

    async fn flashbots_signature(&self, body: &str) -> Result<String, FlashbotsClientError> {
        let body_hash = format!("0x{}", hex::encode(keccak256(body.as_bytes())));
        let signature = self.auth_wallet.sign_message(body_hash).await?;
        Ok(format!("{}:{signature}", self.auth_wallet.address()))
    }

    fn privacy(&self) -> Option<MevBundlePrivacy> {
        (!self.builders.is_empty() || !self.privacy_hints.is_empty()).then(|| MevBundlePrivacy {
            hints: self.privacy_hints.clone(),
            builders: self.builders.clone(),
        })
    }

    fn metadata(&self) -> Option<MevBundleMetadata> {
        self.origin_id
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .map(|origin_id| MevBundleMetadata {
                origin_id: origin_id.clone(),
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlashbotsTailBundleRequest {
    pub tail_after_tx_hash: String,
    pub signed_tx: String,
    pub target_block: u64,
    pub max_block: u64,
    pub can_revert: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlashbotsBundleSubmission {
    pub bundle_hash: String,
    pub tail_after_tx_hash: String,
    pub target_block: u64,
    pub max_block: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MevSendBundleParams {
    pub version: String,
    pub inclusion: MevBundleInclusion,
    pub body: Vec<MevBundleBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validity: Option<MevBundleValidity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy: Option<MevBundlePrivacy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MevBundleMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MevBundleInclusion {
    pub block: String,
    #[serde(rename = "maxBlock", default, skip_serializing_if = "Option::is_none")]
    pub max_block: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MevBundleBody {
    Hash {
        hash: String,
    },
    Tx {
        tx: String,
        #[serde(rename = "canRevert")]
        can_revert: bool,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MevBundleValidity {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refund: Vec<Value>,
    #[serde(
        rename = "refundConfig",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub refund_config: Vec<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MevBundlePrivacy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub builders: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MevBundleMetadata {
    #[serde(rename = "originId")]
    pub origin_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FlashbotsBundleResult {
    #[serde(rename = "bundleHash")]
    pub bundle_hash: String,
    #[serde(default)]
    pub smart: Option<String>,
}

#[derive(Serialize)]
struct JsonRpcRequest<T> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: Vec<T>,
}

#[derive(Debug, thiserror::Error)]
pub enum FlashbotsClientError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("Flashbots HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Flashbots relay returned HTTP {status}: {body}")]
    Server { status: u16, body: String },
    #[error("Flashbots RPC error: {0}")]
    Rpc(String),
    #[error("Flashbots auth signing error: {0}")]
    Signer(#[from] ethers_signers::WalletError),
    #[error("Flashbots JSON error: {0}")]
    Serialization(#[from] serde_json::Error),
}

fn block_hex(block: u64) -> String {
    format!("0x{block:x}")
}

fn validate_hex_prefixed(value: &str, label: &str) -> Result<(), FlashbotsClientError> {
    let trimmed = value.trim();
    if trimmed.starts_with("0x") && trimmed.len() > 2 {
        Ok(())
    } else {
        Err(FlashbotsClientError::Config(format!(
            "{label} must be a non-empty 0x-prefixed hex string"
        )))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn tail_bundle_serializes_hash_then_backrun_tx() {
        let params = MevSendBundleParams {
            version: "v0.1".to_string(),
            inclusion: MevBundleInclusion {
                block: block_hex(25_128_247),
                max_block: Some(block_hex(25_128_249)),
            },
            body: vec![
                MevBundleBody::Hash {
                    hash: format!("0x{}", "11".repeat(32)),
                },
                MevBundleBody::Tx {
                    tx: "0x02abcd".to_string(),
                    can_revert: false,
                },
            ],
            validity: Some(MevBundleValidity::default()),
            privacy: None,
            metadata: None,
        };

        let value = serde_json::to_value(params).unwrap();

        assert_eq!(value["version"], json!("v0.1"));
        assert_eq!(value["inclusion"]["block"], json!("0x17f6d37"));
        assert_eq!(value["inclusion"]["maxBlock"], json!("0x17f6d39"));
        assert_eq!(
            value["body"][0]["hash"],
            json!(format!("0x{}", "11".repeat(32)))
        );
        assert_eq!(value["body"][1]["tx"], json!("0x02abcd"));
        assert_eq!(value["body"][1]["canRevert"], json!(false));
    }
}
