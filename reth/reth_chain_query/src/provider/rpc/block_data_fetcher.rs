use alloy_primitives::B256;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use serde_json::Value;
use tracing::warn;

pub struct RpcBlockDataFetcher {
    http: HttpClient,
    debug: HttpClient,
}

impl RpcBlockDataFetcher {
    pub fn new(http_endpoint: &str) -> eyre::Result<Self> {
        let http = HttpClientBuilder::default().build(http_endpoint)?;
        let debug = HttpClientBuilder::default().build(http_endpoint)?;
        Ok(Self { http, debug })
    }

    pub fn with_debug_endpoint(mut self, endpoint: &str) -> eyre::Result<Self> {
        self.debug = HttpClientBuilder::default().build(endpoint)?;
        Ok(self)
    }

    pub async fn fetch_block(&self, block_hash: B256) -> eyre::Result<Option<Value>> {
        let params = rpc_params![format!("{block_hash:#x}"), true];
        let block = self
            .http
            .request::<Option<Value>, _>("eth_getBlockByHash", params)
            .await?;
        Ok(block)
    }

    pub async fn fetch_receipts(&self, block_hash: B256) -> eyre::Result<Option<Vec<Value>>> {
        let params = rpc_params![format!("{block_hash:#x}")];
        let receipts = self
            .http
            .request::<Option<Vec<Value>>, _>("eth_getBlockReceipts", params)
            .await?;
        Ok(receipts)
    }

    pub async fn trace_transaction(&self, tx_hash: B256) -> eyre::Result<Value> {
        let params = rpc_params![
            format!("{tx_hash:#x}"),
            serde_json::json!({"tracer": "callTracer", "timeout": "60s"})
        ];
        let trace = self
            .debug
            .request::<Value, _>("debug_traceTransaction", params)
            .await?;
        Ok(trace)
    }

    pub async fn trace_transactions(&self, tx_hashes: &[B256]) -> eyre::Result<Vec<Value>> {
        let mut traces = Vec::with_capacity(tx_hashes.len());
        for hash in tx_hashes {
            match self.trace_transaction(*hash).await {
                Ok(value) => traces.push(value),
                Err(err) => {
                    warn!("trace for {hash:#x} failed: {}", err);
                    return Err(err);
                }
            }
        }
        Ok(traces)
    }
}
