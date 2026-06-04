use eyre::{Result, WrapErr};
use serde::Deserialize;

use eth_alpha_engine::wire::{
    GasRankSamplesResponse, LiveBlockFrameResponse, LiveStatusResponse, MempoolSignalsResponse,
};

#[derive(Clone)]
pub(super) struct TokenServerClient {
    base_url: String,
    http: reqwest::Client,
}

impl TokenServerClient {
    pub(super) fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub(super) async fn status(&self) -> Result<LiveStatusResponse> {
        self.get_json("/api/v1/eth/live-token-tracker/status").await
    }

    pub(super) async fn versioned_status(&self) -> Result<LiveStatusResponse> {
        self.status().await
    }

    pub(super) async fn gas_rank_samples(&self, limit: usize) -> Result<GasRankSamplesResponse> {
        let path = format!("/api/v1/eth/alpha/gas-rank/samples?limit={limit}");
        self.get_json(&path).await
    }

    pub(super) async fn next_block_frame(
        &self,
        after_block: u64,
        timeout_ms: u64,
    ) -> Result<LiveBlockFrameResponse> {
        let path = format!(
            "/api/v1/eth/live-trading/block-frames/next?after_block={after_block}&timeout_ms={timeout_ms}"
        );
        self.get_json(&path).await
    }

    pub(super) async fn mempool_signals(
        &self,
        limit: i64,
        since_days: i64,
    ) -> Result<MempoolSignalsResponse> {
        let path = format!(
            "/api/v1/eth/mempool/pending-transaction-signals?limit={limit}&since_days={since_days}"
        );
        self.get_json(&path).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        self.http
            .get(&url)
            .send()
            .await
            .wrap_err_with(|| format!("request failed: {url}"))?
            .error_for_status()
            .wrap_err_with(|| format!("token server returned an error: {url}"))?
            .json::<T>()
            .await
            .wrap_err_with(|| format!("failed to decode token server response: {url}"))
    }
}
