use std::time::Duration;

use alloy_primitives::B256;
use reqwest::{Client, StatusCode};
use tokio::time::{sleep, Duration as TokioDuration};

use crate::live_chain::models::ExecutionInfo;

pub struct BeaconRestClient {
    base_url: String,
    client: Client,
}

impl BeaconRestClient {
    pub fn new(base_url: &str) -> eyre::Result<Self> {
        let base = base_url.trim_end_matches('/');
        let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
        Ok(Self {
            base_url: base.to_owned(),
            client,
        })
    }

    pub async fn fetch_execution_info(&self, block_root: B256) -> eyre::Result<ExecutionInfo> {
        let url = format!("{}/eth/v2/beacon/blocks/{:#x}", self.base_url, block_root);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let response = self.client.get(&url).send().await?;
            if response.status() == StatusCode::NOT_FOUND {
                if std::time::Instant::now() > deadline {
                    return Err(eyre::eyre!(
                        "beacon node did not return block data for root {:#x}",
                        block_root
                    ));
                }
                sleep(TokioDuration::from_millis(100)).await;
                continue;
            }
            let response = response.error_for_status()?;
            let payload: BeaconBlockResponse = response.json().await?;
            let payload = payload.data.message.body.execution_payload;
            let block_hash = parse_hash(&payload.block_hash)?;
            let block_number = parse_u64_hex(&payload.block_number)?;
            let timestamp = parse_u64_hex(&payload.timestamp)?;
            return Ok(ExecutionInfo {
                block_hash,
                block_number,
                timestamp,
            });
        }
    }

    pub fn sse_endpoint(&self) -> String {
        format!("{}/eth/v1/events?topics=head", self.base_url)
    }
}

#[derive(Debug, serde::Deserialize)]
struct BeaconBlockResponse {
    data: SignedBeaconBlock,
}

#[derive(Debug, serde::Deserialize)]
struct SignedBeaconBlock {
    message: BeaconBlock,
}

#[derive(Debug, serde::Deserialize)]
struct BeaconBlock {
    body: BeaconBlockBody,
}

#[derive(Debug, serde::Deserialize)]
struct BeaconBlockBody {
    #[serde(rename = "execution_payload")]
    execution_payload: ExecutionPayload,
}

#[derive(Debug, serde::Deserialize)]
struct ExecutionPayload {
    #[serde(rename = "block_hash")]
    block_hash: String,
    #[serde(rename = "block_number")]
    block_number: String,
    timestamp: String,
}

fn parse_hash(value: &str) -> eyre::Result<B256> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(trimmed)?;
    if bytes.len() != 32 {
        return Err(eyre::eyre!("expected 32-byte hash, got {}", value));
    }
    Ok(B256::from_slice(&bytes))
}

fn parse_u64_hex(value: &str) -> eyre::Result<u64> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    u64::from_str_radix(trimmed, 16).map_err(|err| eyre::eyre!("invalid hex {}: {}", value, err))
}
