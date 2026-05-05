use std::{future::Future, str::FromStr, sync::Arc};

use alloy_primitives::B256;
use alloy_rpc_types_trace::geth::PreStateFrame;
use chrono::{DateTime, Utc};
use eyre::Result;
use jsonrpsee::{
    core::client::{Subscription, SubscriptionClientT},
    rpc_params,
    ws_client::{WsClient, WsClientBuilder},
};
use reth_chain_query::{
    live_chain::models::ExecutionInfo,
    provider::{BlockDataFetcher, RpcBlockDataFetcher},
    RethQueryProvider,
};

use serde::Deserialize;

use crate::{block_processor::BlockProcessor, ProcessedBlock};

/// Configuration for [`LiveBlockProcessor`].
#[derive(Debug, Clone)]
pub struct LiveBlockProcessorConfig {
    pub execution_rpc: String,
    pub execution_ws: String,
    pub include_traces: bool,
}

impl Default for LiveBlockProcessorConfig {
    fn default() -> Self {
        Self {
            execution_rpc: "http://127.0.0.1:8545".to_string(),
            execution_ws: "ws://127.0.0.1:8546".to_string(),
            include_traces: true,
        }
    }
}

impl LiveBlockProcessorConfig {
    pub fn with_execution_rpc(mut self, url: impl Into<String>) -> Self {
        self.execution_rpc = url.into();
        self
    }

    pub fn with_execution_ws(mut self, url: impl Into<String>) -> Self {
        self.execution_ws = url.into();
        self
    }

    pub fn include_traces(mut self, include: bool) -> Self {
        self.include_traces = include;
        self
    }
}

/// Real-time block processor that mirrors the Python live block processor.
pub struct LiveBlockProcessor {
    block_processor: BlockProcessor,
    include_traces: bool,
    execution_ws: String,
    ws_client: WsClient,
    head_subscription: Subscription<WsHead>,
}

impl LiveBlockProcessor {
    /// Connect to Lighthouse SSE + execution RPC and prepare to stream processed blocks.
    pub async fn connect(
        _provider: Arc<RethQueryProvider>,
        config: LiveBlockProcessorConfig,
    ) -> Result<Self> {
        let rpc_fetcher = RpcBlockDataFetcher::new(&config.execution_rpc)?;
        let fetcher = BlockDataFetcher::rpc_only(rpc_fetcher);
        let block_processor = BlockProcessor::with_block_fetcher(Arc::new(fetcher));
        let (ws_client, head_subscription) =
            Self::connect_subscription(&config.execution_ws).await?;

        Ok(Self {
            block_processor,
            include_traces: config.include_traces,
            execution_ws: config.execution_ws,
            ws_client,
            head_subscription,
        })
    }

    async fn connect_subscription(execution_ws: &str) -> Result<(WsClient, Subscription<WsHead>)> {
        let client = WsClientBuilder::default().build(execution_ws).await?;
        let subscription = client
            .subscribe::<WsHead, _>("eth_subscribe", rpc_params!["newHeads"], "eth_unsubscribe")
            .await?;
        Ok((client, subscription))
    }

    /// Process the next head event and return the fully processed block.
    pub async fn next_processed_block(&mut self) -> Result<LiveProcessedBlock> {
        loop {
            match self.head_subscription.next().await {
                Some(Ok(head)) => match self.process_head_event(head).await {
                    Ok(processed) => return Ok(processed),
                    Err(err) => {
                        tracing::error!("failed to process websocket head: {}", err);
                        continue;
                    }
                },
                Some(Err(err)) => {
                    tracing::warn!("websocket subscription error: {}", err);
                    self.reconnect_ws().await?;
                }
                None => {
                    tracing::warn!("websocket subscription ended, reconnecting");
                    self.reconnect_ws().await?;
                }
            }
        }
    }

    pub async fn latest_block_number(&self) -> Result<u64> {
        self.block_processor.latest_rpc_block_number().await
    }

    pub async fn process_block_number(&self, block_number: u64) -> Result<LiveProcessedBlock> {
        let processed = self
            .block_processor
            .process_block_via_rpc_number(block_number, self.include_traces)
            .await?;
        let state_diffs = self.fetch_state_diffs_for_processed_block(&processed).await;
        let execution_info = ExecutionInfo {
            block_hash: processed.header.hash,
            block_number: processed.header.number,
            timestamp: processed.header.timestamp,
        };

        Ok(LiveProcessedBlock {
            execution_info,
            head_arrival: Utc::now(),
            processed_block: processed,
            state_diffs,
            processed_at: Utc::now(),
        })
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

    async fn reconnect_ws(&mut self) -> Result<()> {
        let (client, subscription) = Self::connect_subscription(&self.execution_ws).await?;
        self.ws_client = client;
        self.head_subscription = subscription;
        Ok(())
    }

    async fn process_head_event(&self, head: WsHead) -> Result<LiveProcessedBlock> {
        let arrival = Utc::now();
        let hash = B256::from_str(&head.hash)?;
        let mut block_number = parse_hex_u64_opt(head.number.as_deref()).unwrap_or(0);
        let mut timestamp = parse_hex_u64_opt(head.timestamp.as_deref()).unwrap_or(0);

        let processed = self
            .block_processor
            .process_block_via_rpc(hash, block_number, self.include_traces)
            .await?;
        let state_diffs = self.fetch_state_diffs_for_processed_block(&processed).await;

        if block_number == 0 {
            block_number = processed.header.number;
        }
        if timestamp == 0 {
            timestamp = processed.header.timestamp;
        }

        let execution_info = ExecutionInfo {
            block_hash: hash,
            block_number,
            timestamp,
        };

        Ok(LiveProcessedBlock {
            execution_info,
            head_arrival: arrival,
            processed_block: processed,
            state_diffs,
            processed_at: Utc::now(),
        })
    }

    async fn fetch_state_diffs_for_processed_block(
        &self,
        processed: &ProcessedBlock,
    ) -> Option<Vec<PreStateFrame>> {
        let block_number = processed.header.number;
        let tx_count = processed.transactions.len();
        if tx_count == 0 {
            return Some(Vec::new());
        }

        match self
            .block_processor
            .fetch_rpc_state_diffs_by_number(block_number)
            .await
        {
            Ok(state_diffs) if state_diffs.len() == tx_count => Some(state_diffs),
            Ok(state_diffs) => {
                tracing::warn!(
                    block_number,
                    tx_count,
                    state_diff_count = state_diffs.len(),
                    "prestate diff trace count did not match block transaction count"
                );
                None
            }
            Err(err) => {
                tracing::warn!(
                    block_number,
                    "failed to fetch exact prestate diff traces: {}",
                    err
                );
                None
            }
        }
    }
}

/// Result payload for a processed live block.
#[derive(Debug)]
pub struct LiveProcessedBlock {
    pub execution_info: ExecutionInfo,
    pub head_arrival: DateTime<Utc>,
    pub processed_block: ProcessedBlock,
    pub state_diffs: Option<Vec<PreStateFrame>>,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct WsHead {
    hash: String,
    number: Option<String>,
    timestamp: Option<String>,
}

fn parse_hex_u64_opt(value: Option<&str>) -> Option<u64> {
    value.and_then(|input| {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }
        let digits = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        u64::from_str_radix(if digits.is_empty() { "0" } else { digits }, 16).ok()
    })
}
