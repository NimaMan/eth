use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use eth_token::live::LiveBlockTokenProcessor;
use eth_token::manager::{RethChainDiscoveryProvider, TokenBlockUpdateReport};
use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, RwLockReadGuard};
use tx_processor::{BlockProcessor, LivePoolBuySellSimulator};

use crate::config::TokenServerConfig;
use crate::processed_block_cache::TokenProcessedBlockCacheStore;
use crate::runs::progress::now_unix_secs;

use super::state::{LiveTrackerError, LiveTrackerProgress, LiveTrackerState, LiveTrackerStatus};
use super::warmup::{load_processed_block, LiveWarmupBlock};

#[derive(Clone)]
pub struct LiveTracker {
    inner: Arc<LiveTrackerInner>,
}

struct LiveTrackerInner {
    config: TokenServerConfig,
    provider: Arc<RethQueryProvider>,
    processed_block_cache: Option<Arc<TokenProcessedBlockCacheStore>>,
    state: RwLock<LiveTrackerState>,
    stop_requested: AtomicBool,
    next_id: AtomicU64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StartLiveTrackerRequest {
    #[serde(default)]
    pub start_block: Option<u64>,
    #[serde(default)]
    pub end_block: Option<u64>,
    #[serde(default)]
    pub warmup_blocks: Option<u64>,
    #[serde(default)]
    pub history_limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedLiveTrackerRequest {
    pub id: String,
    pub start_block: u64,
    pub end_block: u64,
    pub warmup_blocks: u64,
    pub history_limit: usize,
}

impl ResolvedLiveTrackerRequest {
    pub fn block_count(&self) -> u64 {
        self.end_block - self.start_block + 1
    }
}

impl LiveTracker {
    pub fn new(
        config: TokenServerConfig,
        provider: Arc<RethQueryProvider>,
        processed_block_cache: Option<Arc<TokenProcessedBlockCacheStore>>,
    ) -> Self {
        let history_limit = config.history_limit;
        Self {
            inner: Arc::new(LiveTrackerInner {
                config,
                provider,
                processed_block_cache,
                state: RwLock::new(LiveTrackerState::idle(history_limit)),
                stop_requested: AtomicBool::new(false),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    pub async fn start(
        &self,
        request: StartLiveTrackerRequest,
    ) -> Result<ResolvedLiveTrackerRequest> {
        {
            let state = self.inner.state.read().await;
            if state.progress.status.is_busy() {
                bail!(
                    "live token tracker is already {}",
                    status_label(&state.progress.status)
                );
            }
        }

        let request = self.resolve_request(request)?;
        self.inner.stop_requested.store(false, Ordering::SeqCst);

        {
            let mut state = self.inner.state.write().await;
            *state = LiveTrackerState::warming(
                request.id.clone(),
                request.history_limit,
                request.start_block,
                request.end_block,
            );
        }

        let tracker = self.clone();
        let task_request = request.clone();
        tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build live token tracker runtime");
            runtime.block_on(tracker.run_warmup(task_request));
        });

        Ok(request)
    }

    pub async fn stop(&self) -> LiveTrackerProgress {
        self.inner.stop_requested.store(true, Ordering::SeqCst);
        let mut state = self.inner.state.write().await;
        match state.progress.status {
            LiveTrackerStatus::Warming => {
                state.progress.status = LiveTrackerStatus::Stopping;
                state.progress.updated_at_unix_secs = now_unix_secs();
            }
            LiveTrackerStatus::Ready => {
                state.progress.status = LiveTrackerStatus::Stopped;
                state.progress.completed_at_unix_secs = Some(now_unix_secs());
                state.progress.updated_at_unix_secs = now_unix_secs();
            }
            _ => {}
        }
        state.progress.clone()
    }

    pub async fn progress(&self) -> LiveTrackerProgress {
        self.inner.state.read().await.progress.clone()
    }

    pub async fn state(&self) -> RwLockReadGuard<'_, LiveTrackerState> {
        self.inner.state.read().await
    }

    fn resolve_request(
        &self,
        request: StartLiveTrackerRequest,
    ) -> Result<ResolvedLiveTrackerRequest> {
        let history_limit = request
            .history_limit
            .unwrap_or(self.inner.config.history_limit);
        if history_limit == 0 {
            bail!("history_limit must be greater than zero");
        }

        let warmup_blocks = request
            .warmup_blocks
            .unwrap_or(self.inner.config.default_blocks);
        if warmup_blocks == 0 {
            bail!("warmup_blocks must be greater than zero");
        }

        let latest_cached_block = self.latest_cached_block()?;
        let latest_block = if request.start_block.is_none() || request.end_block.is_none() {
            Some(
                latest_cached_block
                    .or_else(|| self.inner.provider.get_latest_block().ok())
                    .ok_or_else(|| eyre::eyre!("could not resolve latest block"))?,
            )
        } else {
            None
        };

        let end_block = request
            .end_block
            .or(latest_block)
            .ok_or_else(|| eyre::eyre!("could not resolve end block"))?;
        let start_block = match request.start_block {
            Some(start_block) => start_block,
            None => end_block
                .checked_sub(warmup_blocks - 1)
                .ok_or_else(|| eyre::eyre!("live warmup range underflow"))?,
        };

        if end_block < start_block {
            bail!("end_block must be greater than or equal to start_block");
        }

        let block_count = end_block - start_block + 1;
        if block_count > self.inner.config.max_blocks {
            bail!(
                "live warmup has {} blocks, max allowed is {}",
                block_count,
                self.inner.config.max_blocks
            );
        }

        let sequence = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        Ok(ResolvedLiveTrackerRequest {
            id: format!("live-{sequence}"),
            start_block,
            end_block,
            warmup_blocks,
            history_limit,
        })
    }

    fn latest_cached_block(&self) -> Result<Option<u64>> {
        let Some(cache_store) = self.inner.processed_block_cache.as_deref() else {
            return Ok(None);
        };
        let coverage = cache_store.coverage()?;
        let chain_id = self.inner.provider.chain_id();
        Ok(coverage
            .chains
            .iter()
            .find(|chain| chain.chain_id == chain_id)
            .and_then(|chain| chain.ranges.iter().map(|range| range.end_block).max()))
    }

    async fn run_warmup(&self, request: ResolvedLiveTrackerRequest) {
        tracing::info!(
            live_id = %request.id,
            start_block = request.start_block,
            end_block = request.end_block,
            "starting live token tracker warmup"
        );

        let tx_processor = BlockProcessor::new(self.inner.provider.clone());
        let discovery_provider = RethChainDiscoveryProvider::new(self.inner.provider.as_ref());
        let pool_simulator =
            LivePoolBuySellSimulator::from_simulator(self.inner.provider.simulator().clone());

        for block_number in request.start_block..=request.end_block {
            if self.inner.stop_requested.load(Ordering::SeqCst) {
                self.mark_stopped().await;
                return;
            }

            let loaded = match load_processed_block(
                &tx_processor,
                self.inner.provider.as_ref(),
                self.inner.processed_block_cache.clone(),
                block_number,
            )
            .await
            {
                Ok(loaded) => loaded,
                Err(error) => {
                    self.mark_failed(LiveTrackerError {
                        block_number: Some(block_number),
                        tx_index: None,
                        tx_hash: None,
                        message: error.to_string(),
                    })
                    .await;
                    return;
                }
            };

            let mut processor = self.take_processor_for_apply(block_number).await;
            let apply_started = Instant::now();
            let report = processor
                .process_block_live_with_discovery_provider(
                    &loaded.block,
                    &discovery_provider,
                    &pool_simulator,
                )
                .await;
            let retention_report = processor.apply_index_retention_policy(block_number);
            let token_apply_ms = apply_started.elapsed().as_millis();

            self.restore_processor_after_apply(
                processor,
                report,
                retention_report,
                loaded,
                token_apply_ms,
            )
            .await;
        }

        self.mark_ready().await;
    }

    async fn take_processor_for_apply(&self, block_number: u64) -> LiveBlockTokenProcessor {
        let mut state = self.inner.state.write().await;
        state.progress.current_block = Some(block_number);
        state.progress.updated_at_unix_secs = now_unix_secs();
        let history_limit = state.progress.history_limit;
        std::mem::replace(
            &mut state.processor,
            LiveBlockTokenProcessor::new(history_limit),
        )
    }

    async fn restore_processor_after_apply(
        &self,
        processor: LiveBlockTokenProcessor,
        report: TokenBlockUpdateReport,
        retention_report: Option<eth_token::manager::LiveTokenRetentionReport>,
        loaded: LiveWarmupBlock,
        token_apply_ms: u128,
    ) {
        let mut state = self.inner.state.write().await;
        state.processor = processor;
        apply_report(&mut state, report, retention_report, loaded, token_apply_ms);
    }

    async fn mark_ready(&self) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTrackerStatus::Ready;
        state.progress.completed_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
        tracing::info!(
            live_id = ?state.progress.id,
            tracked_tokens = state.progress.tracked_tokens,
            tracked_v2_pools = state.progress.tracked_v2_pools,
            "live token tracker warmup completed"
        );
    }

    async fn mark_stopped(&self) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTrackerStatus::Stopped;
        state.progress.completed_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
    }

    async fn mark_failed(&self, error: LiveTrackerError) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTrackerStatus::Failed;
        state.progress.last_error = Some(error.message.clone());
        state.progress.completed_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
        state.errors.push(error);
    }
}

fn apply_report(
    state: &mut LiveTrackerState,
    report: TokenBlockUpdateReport,
    retention_report: Option<eth_token::manager::LiveTokenRetentionReport>,
    loaded: LiveWarmupBlock,
    token_apply_ms: u128,
) {
    state.progress.current_block = Some(report.block_number);
    state.progress.blocks_processed += 1;
    state.progress.txs_scanned += report.transaction_count;
    state.progress.txs_processed += report.processed_transaction_count;
    state.progress.tx_failures += report.failed_transaction_count;
    state.progress.token_update_reports += report.token_updates.len();
    state.progress.last_block_upstream_ms = Some(loaded.upstream_ms);
    state.progress.last_block_token_apply_ms = Some(token_apply_ms);
    state.progress.last_block_cache_read_ms = Some(loaded.cache_read_ms);
    state.progress.last_block_cache_write_ms = Some(loaded.cache_write_ms);
    state.progress.last_block_source = Some(loaded.source.to_string());
    if loaded.cache_hit {
        state.progress.processed_block_cache_hits += 1;
    } else {
        state.progress.processed_block_cache_misses += 1;
    }
    state.progress.updated_at_unix_secs = now_unix_secs();

    state.created_tokens.extend(report.created_token_addresses);
    state.updated_tokens.extend(report.updated_token_addresses);

    for update in report.token_updates {
        state
            .discovered_v2_pools
            .extend(update.discovered_uniswap_v2_pools);
        state
            .updated_v2_pools
            .extend(update.updated_uniswap_v2_pools);
    }

    for error in report.transaction_errors {
        state.errors.push(LiveTrackerError {
            block_number: Some(report.block_number),
            tx_index: Some(error.tx_index),
            tx_hash: Some(error.tx_hash),
            message: error.message,
        });
    }

    if let Some(retention_report) = retention_report {
        state.progress.retention_evaluated_tokens = retention_report.evaluated_tokens;
        state.progress.retention_dropped_tokens += retention_report.dropped_tokens;
        state.progress.retention_dropped_v2_pools += retention_report.dropped_v2_pool_count;
        state.last_retention_report = Some(retention_report);
    }

    state.progress.created_tokens_unique = state.created_tokens.len();
    state.progress.updated_tokens_unique = state.updated_tokens.len();
    state.progress.discovered_v2_pools_unique = state.discovered_v2_pools.len();
    state.progress.updated_v2_pools_unique = state.updated_v2_pools.len();
    state.progress.tracked_tokens = state.processor.registry().tokens.len();
    state.progress.indexed_tokens = state.processor.block_processor().token_index.entries.len();
    state.progress.indexed_v2_pools = state
        .processor
        .block_processor()
        .token_index
        .pool_to_token
        .len();
    state.progress.tracked_v2_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum();
}

fn status_label(status: &LiveTrackerStatus) -> &'static str {
    match status {
        LiveTrackerStatus::Idle => "idle",
        LiveTrackerStatus::Warming => "warming",
        LiveTrackerStatus::Ready => "ready",
        LiveTrackerStatus::Stopping => "stopping",
        LiveTrackerStatus::Stopped => "stopped",
        LiveTrackerStatus::Failed => "failed",
    }
}
