use std::time::Duration;

use alloy_primitives::B256;
use chrono::{DateTime, Utc};
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use serde::Deserialize;
use tokio::time::sleep;

#[derive(Debug)]
pub struct BlockReadiness {
    pub number: Option<u64>,
    pub timestamp: Option<u64>,
    pub transaction_count: usize,
    pub ready_at: DateTime<Utc>,
}

pub struct ExecutionClient {
    http: HttpClient,
    poll_interval: Duration,
    readiness_timeout: Duration,
}

impl ExecutionClient {
    pub fn new(
        endpoint: &str,
        poll_interval: Duration,
        readiness_timeout: Duration,
    ) -> eyre::Result<Self> {
        let http = HttpClientBuilder::default().build(endpoint)?;
        Ok(Self {
            http,
            poll_interval,
            readiness_timeout,
        })
    }

    pub async fn wait_for_block(&self, block_hash: B256) -> eyre::Result<Option<BlockReadiness>> {
        let hash_hex = format!("{:#x}", block_hash);
        let deadline = Utc::now() + chrono::Duration::from_std(self.readiness_timeout)?;
        loop {
            let params = rpc_params![hash_hex.clone(), false];
            match self
                .http
                .request::<Option<RpcBlock>, _>("eth_getBlockByHash", params)
                .await
            {
                Ok(Some(block)) => {
                    let ready_at = Utc::now();
                    return Ok(Some(BlockReadiness {
                        number: block.number_hex.and_then(|v| hex_to_u64(&v).ok()),
                        timestamp: block.timestamp_hex.and_then(|v| hex_to_u64(&v).ok()),
                        transaction_count: block.transactions.len(),
                        ready_at,
                    }));
                }
                Ok(None) => {}
                Err(err) => {
                    tracing::warn!("Error fetching block {hash_hex}: {err}");
                }
            }

            if Utc::now() > deadline {
                return Ok(None);
            }
            sleep(self.poll_interval).await;
        }
    }
}

#[derive(Debug, Deserialize)]
struct RpcBlock {
    #[serde(rename = "number")]
    number_hex: Option<String>,
    #[serde(rename = "timestamp")]
    timestamp_hex: Option<String>,
    transactions: Vec<serde_json::Value>,
}

fn hex_to_u64(value: &str) -> Result<u64, std::num::ParseIntError> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    if trimmed.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(trimmed, 16)
}
