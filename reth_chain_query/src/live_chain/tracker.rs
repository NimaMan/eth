use std::time::Duration;

use chrono::Utc;
use serde_json::Value;

use crate::live_chain::{
    beacon::BeaconRestClient,
    engine::ExecutionClient,
    lighthouse::LighthouseHeadListener,
    models::{HeadLatencySample, HeadObservation},
    utils::transaction_hashes_from_block,
};
use crate::provider::RpcBlockDataFetcher;

pub struct HeadLatencyTracker {
    beacon: BeaconRestClient,
    execution_rpc_url: String,
    poll_interval: Duration,
    readiness_timeout: Duration,
    rpc_fetcher: RpcBlockDataFetcher,
}

impl HeadLatencyTracker {
    pub fn new(beacon_base_url: &str, execution_rpc_url: impl Into<String>) -> eyre::Result<Self> {
        let exec_url = execution_rpc_url.into();
        let rpc_fetcher = RpcBlockDataFetcher::new(&exec_url)?.with_debug_endpoint(&exec_url)?;
        Ok(Self {
            beacon: BeaconRestClient::new(beacon_base_url)?,
            execution_rpc_url: exec_url,
            poll_interval: Duration::from_millis(200),
            readiness_timeout: Duration::from_secs(10),
            rpc_fetcher,
        })
    }
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn readiness_timeout(mut self, timeout: Duration) -> Self {
        self.readiness_timeout = timeout;
        self
    }

    pub async fn measure_for_duration(
        &self,
        duration: Duration,
    ) -> eyre::Result<Vec<HeadLatencySample>> {
        let events_url = self.beacon.sse_endpoint();
        let mut listener = LighthouseHeadListener::connect(&events_url).await?;
        let execution = ExecutionClient::new(
            &self.execution_rpc_url,
            self.poll_interval,
            self.readiness_timeout,
        )?;

        let end_time = Utc::now() + chrono::Duration::from_std(duration)?;
        let mut samples = Vec::new();

        while Utc::now() < end_time {
            let observation = listener.next_observation().await?;
            let sample = self.record_latency(&execution, observation).await?;
            samples.push(sample);
        }

        Ok(samples)
    }

    async fn record_latency(
        &self,
        execution: &ExecutionClient,
        observation: HeadObservation,
    ) -> eyre::Result<HeadLatencySample> {
        let exec_info = self
            .beacon
            .fetch_execution_info(observation.block_root)
            .await?;
        let mut sample = HeadLatencySample::new(observation.clone());
        sample.execution_info = Some(exec_info.clone());
        let readiness = execution
            .wait_for_block(exec_info.block_hash)
            .await?
            .ok_or_else(|| {
                eyre::eyre!(
                    "execution node never served block {:?}",
                    exec_info.block_hash
                )
            })?;
        sample.block_number = readiness.number;
        sample.block_timestamp = readiness.timestamp;
        sample.transaction_count = Some(readiness.transaction_count);
        sample.rpc_ready_at = Some(readiness.ready_at);
        let delta = (readiness.ready_at - observation.arrival_time)
            .num_microseconds()
            .unwrap_or(0) as f64
            / 1_000_000f64;
        sample.rpc_fetch_delay_secs = Some(delta.max(0.0));
        let block_json = self
            .rpc_fetcher
            .fetch_block(exec_info.block_hash)
            .await?
            .ok_or_else(|| eyre::eyre!("block data missing for {:?}", exec_info.block_hash))?;
        let tx_hashes = transaction_hashes_from_block(&block_json)?;
        let receipts = self
            .rpc_fetcher
            .fetch_receipts(exec_info.block_hash)
            .await?
            .ok_or_else(|| eyre::eyre!("receipts missing for {:?}", exec_info.block_hash))?;
        let traces = self.rpc_fetcher.trace_transactions(&tx_hashes).await?;
        sample.block = block_json;
        sample.receipts = receipts;
        sample.traces = Value::Array(traces);
        Ok(sample)
    }
}
