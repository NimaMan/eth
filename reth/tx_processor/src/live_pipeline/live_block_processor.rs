use std::{future::Future, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use eyre::Result;
use reth_chain_query::{
    live_chain::{
        beacon::BeaconRestClient,
        engine::{BlockReadiness, ExecutionClient},
        lighthouse::LighthouseHeadListener,
        models::{ExecutionInfo, HeadObservation},
    },
    provider::{BlockDataFetcher, RpcBlockDataFetcher},
    RethQueryProvider,
};

use crate::{block_processor::BlockProcessor, ProcessedBlock};

/// Configuration for [`LiveBlockProcessor`].
#[derive(Debug, Clone)]
pub struct LiveBlockProcessorConfig {
    pub beacon_api: String,
    pub execution_rpc: String,
    pub include_traces: bool,
    pub poll_interval: Duration,
    pub readiness_timeout: Duration,
}

impl Default for LiveBlockProcessorConfig {
    fn default() -> Self {
        Self {
            beacon_api: "http://127.0.0.1:5052".to_string(),
            execution_rpc: "http://127.0.0.1:8545".to_string(),
            include_traces: true,
            poll_interval: Duration::from_millis(200),
            readiness_timeout: Duration::from_secs(10),
        }
    }
}

impl LiveBlockProcessorConfig {
    pub fn with_beacon_api(mut self, url: impl Into<String>) -> Self {
        self.beacon_api = url.into();
        self
    }

    pub fn with_execution_rpc(mut self, url: impl Into<String>) -> Self {
        self.execution_rpc = url.into();
        self
    }

    pub fn include_traces(mut self, include: bool) -> Self {
        self.include_traces = include;
        self
    }

    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn readiness_timeout(mut self, timeout: Duration) -> Self {
        self.readiness_timeout = timeout;
        self
    }
}

/// Real-time block processor that mirrors the Python live block processor.
pub struct LiveBlockProcessor {
    block_processor: BlockProcessor,
    include_traces: bool,
    beacon: BeaconRestClient,
    execution: ExecutionClient,
    listener: LighthouseHeadListener,
    events_endpoint: String,
}

impl LiveBlockProcessor {
    /// Connect to Lighthouse SSE + execution RPC and prepare to stream processed blocks.
    pub async fn connect(
        provider: Arc<RethQueryProvider>,
        config: LiveBlockProcessorConfig,
    ) -> Result<Self> {
        let rpc_fetcher = RpcBlockDataFetcher::new(&config.execution_rpc)?;
        let fetcher = BlockDataFetcher::new(provider).with_rpc_fetcher(rpc_fetcher);
        let block_processor = BlockProcessor::with_block_fetcher(Arc::new(fetcher));
        let beacon = BeaconRestClient::new(&config.beacon_api)?;
        let events_endpoint = beacon.sse_endpoint();
        let listener = LighthouseHeadListener::connect(&events_endpoint).await?;
        let execution = ExecutionClient::new(
            &config.execution_rpc,
            config.poll_interval,
            config.readiness_timeout,
        )?;

        Ok(Self {
            block_processor,
            include_traces: config.include_traces,
            beacon,
            execution,
            listener,
            events_endpoint,
        })
    }

    /// Return the Lighthouse SSE endpoint that is being consumed.
    pub fn events_endpoint(&self) -> &str {
        &self.events_endpoint
    }

    /// Process the next head event and return the fully processed block.
    pub async fn next_processed_block(&mut self) -> Result<LiveProcessedBlock> {
        loop {
            let observation = match self.listener.next_observation().await {
                Ok(obs) => obs,
                Err(err) => {
                    tracing::warn!(
                        "head listener failed ({}); attempting to reconnect to {}",
                        err,
                        self.events_endpoint
                    );
                    self.reconnect_listener().await?;
                    continue;
                }
            };

            match self.process_observation(observation.clone()).await {
                Ok(Some(processed)) => return Ok(processed),
                Ok(None) => {
                    tracing::warn!(
                        "execution node never served block for observation slot {}",
                        observation.slot
                    );
                    continue;
                }
                Err(err) => {
                    tracing::error!(
                        "failed to process block for slot {}: {}",
                        observation.slot,
                        err
                    );
                }
            }
        }
    }

    /// Continuously run and invoke the callback for each processed block.
    pub async fn run<F, Fut>(&mut self, mut handler: F) -> Result<()>
    where
        F: FnMut(LiveProcessedBlock) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        loop {
            let processed = self.next_processed_block().await?;
            handler(processed).await?;
        }
    }

    async fn reconnect_listener(&mut self) -> Result<()> {
        self.listener = LighthouseHeadListener::connect(&self.events_endpoint).await?;
        Ok(())
    }

    async fn process_observation(
        &self,
        observation: HeadObservation,
    ) -> Result<Option<LiveProcessedBlock>> {
        let exec_info = self
            .beacon
            .fetch_execution_info(observation.block_root)
            .await?;
        let readiness = match self.execution.wait_for_block(exec_info.block_hash).await? {
            Some(ready) => ready,
            None => return Ok(None),
        };

        let processed = self
            .block_processor
            .process_block_via_rpc(
                exec_info.block_hash,
                exec_info.block_number,
                self.include_traces,
            )
            .await?;

        Ok(Some(LiveProcessedBlock {
            observation,
            execution_info: exec_info,
            ready: readiness,
            processed_block: processed,
            processed_at: Utc::now(),
        }))
    }
}

/// Result payload for a processed live block.
#[derive(Debug)]
pub struct LiveProcessedBlock {
    pub observation: HeadObservation,
    pub execution_info: ExecutionInfo,
    pub ready: BlockReadiness,
    pub processed_block: ProcessedBlock,
    pub processed_at: DateTime<Utc>,
}
