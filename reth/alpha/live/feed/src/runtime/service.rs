use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use eth_live_state::keys;
use eth_token::chain_metadata::{
    LiveRethChainMetadataProvider, RethChainMetadataProvider, TokenDiscoveryProvider,
};
use eth_token::live::LiveBlockTokenProcessor;
use eth_token::manager::TokenBlockUpdateReport;
use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use tokio::sync::{broadcast, Mutex, RwLock, RwLockReadGuard};
use tx_processor::{BlockProcessor, LivePoolBuySellSimulator, ProcessedBlockDiskCacheStore};

use super::config::LiveTokenRuntimeConfig;
use super::event::LiveTokenEvent;
use super::loader::{load_processed_block, LiveBlockLoad, ProcessedBlockDiskCacheRetry};
use super::progress::{
    LiveTokenError, LiveTokenProgress, LiveTokenStatus, ResolvedLiveTokenRuntimeRequest,
    StartLiveTokenRuntimeRequest,
};
use super::redis_stream::{missing_blocks_after, RedisBlockStream};
use super::snapshot::LiveTokenSnapshot;
use super::state::LiveTokenState;
use super::time::now_unix_secs;

#[derive(Clone)]
pub struct LiveTokenRuntime {
    inner: Arc<LiveTokenRuntimeInner>,
}

struct LiveTokenRuntimeInner {
    config: LiveTokenRuntimeConfig,
    provider: Arc<RethQueryProvider>,
    processed_block_disk_cache: Option<Arc<ProcessedBlockDiskCacheStore>>,
    state: RwLock<LiveTokenState>,
    stop_requested: AtomicBool,
    next_id: AtomicU64,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    event_tx: broadcast::Sender<LiveTokenEvent>,
}

#[async_trait]
pub trait LiveTokenReader: Send + Sync {
    async fn progress(&self) -> LiveTokenProgress;
    async fn token_snapshots(&self) -> Vec<LiveTokenSnapshot>;
    async fn token_snapshot(&self, token_address: &str) -> Option<LiveTokenSnapshot>;
    fn subscribe(&self) -> broadcast::Receiver<LiveTokenEvent>;
}

impl LiveTokenRuntime {
    pub fn new(
        config: LiveTokenRuntimeConfig,
        provider: Arc<RethQueryProvider>,
        processed_block_disk_cache: Option<Arc<ProcessedBlockDiskCacheStore>>,
    ) -> Self {
        let history_limit = config.history_limit;
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(LiveTokenRuntimeInner {
                config,
                provider,
                processed_block_disk_cache,
                state: RwLock::new(LiveTokenState::idle(history_limit)),
                stop_requested: AtomicBool::new(false),
                next_id: AtomicU64::new(1),
                task: Mutex::new(None),
                event_tx,
            }),
        }
    }

    pub async fn start(
        &self,
        request: StartLiveTokenRuntimeRequest,
    ) -> Result<ResolvedLiveTokenRuntimeRequest> {
        {
            let state = self.inner.state.read().await;
            if state.progress.status.is_busy() {
                bail!(
                    "live token runtime is already {}",
                    status_label(&state.progress.status)
                );
            }
        }

        let request = self.resolve_request(request)?;
        self.inner.stop_requested.store(false, Ordering::SeqCst);

        {
            let mut state = self.inner.state.write().await;
            *state = LiveTokenState::warming(
                request.id.clone(),
                request.history_limit,
                request.start_block,
                request.end_block,
            );
        }

        let _ = self.inner.event_tx.send(LiveTokenEvent::WarmupStarted {
            id: request.id.clone(),
            start_block: request.start_block,
            end_block: request.end_block,
        });

        let runtime = self.clone();
        let task_request = request.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let local_runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("live-token-runtime")
                .enable_all()
                .build()
                .expect("failed to build live token runtime thread");
            let runtime_for_failure = runtime.clone();
            let run_result = panic::catch_unwind(AssertUnwindSafe(|| {
                local_runtime.block_on(runtime.run(task_request));
            }));
            if let Err(payload) = run_result {
                let message = format!(
                    "live token runtime task panicked: {}",
                    panic_payload_message(payload.as_ref())
                );
                tracing::error!(error = %message, "live token runtime task panicked");
                local_runtime.block_on(runtime_for_failure.mark_failed(LiveTokenError {
                    block_number: None,
                    tx_index: None,
                    tx_hash: None,
                    message,
                }));
            }
        });

        let mut task = self.inner.task.lock().await;
        if let Some(previous) = task.replace(handle) {
            if !previous.is_finished() {
                previous.abort();
            }
        }

        Ok(request)
    }

    pub async fn stop(&self) -> LiveTokenProgress {
        self.inner.stop_requested.store(true, Ordering::SeqCst);
        let mut state = self.inner.state.write().await;
        match state.progress.status {
            LiveTokenStatus::Warming | LiveTokenStatus::Live => {
                state.progress.status = LiveTokenStatus::Stopping;
                state.progress.updated_at_unix_secs = now_unix_secs();
            }
            LiveTokenStatus::Idle => {
                state.progress.status = LiveTokenStatus::Stopped;
                state.progress.completed_at_unix_secs = Some(now_unix_secs());
                state.progress.updated_at_unix_secs = now_unix_secs();
            }
            _ => {}
        }
        state.progress.clone()
    }

    pub async fn progress(&self) -> LiveTokenProgress {
        self.inner.state.read().await.progress.clone()
    }

    pub async fn state(&self) -> RwLockReadGuard<'_, LiveTokenState> {
        self.inner.state.read().await
    }

    fn resolve_request(
        &self,
        request: StartLiveTokenRuntimeRequest,
    ) -> Result<ResolvedLiveTokenRuntimeRequest> {
        let history_limit = request
            .history_limit
            .unwrap_or(self.inner.config.history_limit);
        if history_limit == 0 {
            bail!("history_limit must be greater than zero");
        }

        let warmup_blocks = request
            .warmup_blocks
            .unwrap_or(self.inner.config.default_warmup_blocks);
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

        let sequence = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        Ok(ResolvedLiveTokenRuntimeRequest {
            id: format!("live-{sequence}"),
            start_block,
            end_block,
            warmup_blocks,
            history_limit,
        })
    }

    fn latest_cached_block(&self) -> Result<Option<u64>> {
        let Some(cache_store) = self.inner.processed_block_disk_cache.as_deref() else {
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

    async fn run(&self, request: ResolvedLiveTokenRuntimeRequest) {
        tracing::info!(
            live_id = %request.id,
            start_block = request.start_block,
            end_block = request.end_block,
            "starting live token runtime warmup"
        );

        let tx_processor = BlockProcessor::new(self.inner.provider.clone());
        let warmup_discovery_provider =
            RethChainMetadataProvider::new(self.inner.provider.as_ref());
        let live_discovery_provider =
            LiveRethChainMetadataProvider::new(self.inner.provider.as_ref());
        let pool_simulator =
            LivePoolBuySellSimulator::from_simulator(self.inner.provider.simulator().clone());

        for block_number in request.start_block..=request.end_block {
            if self.inner.stop_requested.load(Ordering::SeqCst) {
                self.mark_stopped().await;
                return;
            }

            if let Err(error) = self
                .apply_block(
                    block_number,
                    false,
                    ProcessedBlockDiskCacheRetry::none(),
                    &tx_processor,
                    &warmup_discovery_provider,
                    &pool_simulator,
                )
                .await
            {
                self.mark_failed(LiveTokenError {
                    block_number: Some(block_number),
                    tx_index: None,
                    tx_hash: None,
                    message: error.to_string(),
                })
                .await;
                return;
            }
        }

        self.mark_live().await;

        let stream = match RedisBlockStream::new(
            &self.inner.config.redis_url,
            &self.inner.config.live_block_stream,
            keys::latest_block_number_key(),
            self.inner.config.stream_block_ms,
            self.inner.config.stream_count,
        ) {
            Ok(stream) => stream,
            Err(error) => {
                self.mark_failed(LiveTokenError {
                    block_number: None,
                    tx_index: None,
                    tx_hash: None,
                    message: error.to_string(),
                })
                .await;
                return;
            }
        };

        let mut last_stream_id = "$".to_string();
        loop {
            if self.inner.stop_requested.load(Ordering::SeqCst) {
                self.mark_stopped().await;
                return;
            }

            if let Err(error) = self
                .catch_up_to_latest(
                    &stream,
                    &tx_processor,
                    &live_discovery_provider,
                    &pool_simulator,
                )
                .await
            {
                self.mark_failed(LiveTokenError {
                    block_number: None,
                    tx_index: None,
                    tx_hash: None,
                    message: error.to_string(),
                })
                .await;
                return;
            }

            let events = match stream.read_after(&last_stream_id).await {
                Ok(events) => events,
                Err(error) => {
                    self.mark_failed(LiveTokenError {
                        block_number: None,
                        tx_index: None,
                        tx_hash: None,
                        message: error.to_string(),
                    })
                    .await;
                    return;
                }
            };

            if events.is_empty() {
                continue;
            }

            if let Some(last) = events.last() {
                last_stream_id = last.stream_id.clone();
            }
            self.record_stream_events(events.len() as u64, Some(last_stream_id.clone()))
                .await;

            if let Err(error) = self
                .catch_up_to_latest(
                    &stream,
                    &tx_processor,
                    &live_discovery_provider,
                    &pool_simulator,
                )
                .await
            {
                self.mark_failed(LiveTokenError {
                    block_number: None,
                    tx_index: None,
                    tx_hash: None,
                    message: error.to_string(),
                })
                .await;
                return;
            }
        }
    }

    async fn catch_up_to_latest(
        &self,
        stream: &RedisBlockStream,
        tx_processor: &BlockProcessor,
        discovery_provider: &LiveRethChainMetadataProvider<'_>,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> Result<()> {
        let Some(latest_block) = stream.latest_block_number().await? else {
            return Ok(());
        };
        let last_applied_block = self.progress().await.current_block;
        let missing = missing_blocks_after(last_applied_block, latest_block);
        if missing.is_empty() {
            return Ok(());
        }

        if missing.len() > 1 {
            self.record_gap_blocks((missing.len() - 1) as u64).await;
        }
        for block_number in missing {
            if self.inner.stop_requested.load(Ordering::SeqCst) {
                return Ok(());
            }
            self.apply_block(
                block_number,
                true,
                ProcessedBlockDiskCacheRetry {
                    attempts: self.inner.config.processed_block_disk_cache_retry_attempts,
                    delay_ms: self.inner.config.processed_block_disk_cache_retry_delay_ms,
                },
                tx_processor,
                discovery_provider,
                pool_simulator,
            )
            .await?;
        }
        Ok(())
    }

    async fn apply_block<P>(
        &self,
        block_number: u64,
        is_live_tail: bool,
        retry: ProcessedBlockDiskCacheRetry,
        tx_processor: &BlockProcessor,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> Result<()>
    where
        P: TokenDiscoveryProvider,
    {
        let loaded = load_processed_block(
            tx_processor,
            self.inner.provider.as_ref(),
            self.inner.processed_block_disk_cache.clone(),
            block_number,
            retry,
        )
        .await?;

        let mut processor = self.clone_processor_for_apply(block_number).await;
        let apply_started = Instant::now();
        let apply_timeout = Duration::from_millis(self.inner.config.block_apply_timeout_ms);
        let report = match tokio::time::timeout(
            apply_timeout,
            processor.process_block_live_with_discovery_provider(
                &loaded.block,
                discovery_provider,
                pool_simulator,
            ),
        )
        .await
        {
            Ok(report) => report,
            Err(_) => {
                bail!(
                    "live token block apply timed out after {} ms at block {}",
                    self.inner.config.block_apply_timeout_ms,
                    block_number
                );
            }
        };
        let retention_report = processor.apply_index_retention_policy(block_number);
        let token_apply_ms = apply_started.elapsed().as_millis();

        let event = self
            .restore_processor_after_apply(
                processor,
                report,
                retention_report,
                loaded,
                token_apply_ms,
                is_live_tail,
            )
            .await;
        let _ = self.inner.event_tx.send(event);
        Ok(())
    }

    async fn clone_processor_for_apply(&self, block_number: u64) -> LiveBlockTokenProcessor {
        let mut state = self.inner.state.write().await;
        state.progress.current_block = Some(block_number);
        state.progress.updated_at_unix_secs = now_unix_secs();
        state.processor.clone()
    }

    async fn restore_processor_after_apply(
        &self,
        processor: LiveBlockTokenProcessor,
        report: TokenBlockUpdateReport,
        retention_report: Option<eth_token::manager::LiveTokenRetentionReport>,
        loaded: LiveBlockLoad,
        token_apply_ms: u128,
        is_live_tail: bool,
    ) -> LiveTokenEvent {
        let mut state = self.inner.state.write().await;
        state.processor = processor;
        let event = apply_report(
            &mut state,
            report,
            retention_report,
            loaded,
            token_apply_ms,
            is_live_tail,
        );
        if should_log_block_apply(&state.progress, is_live_tail) {
            tracing::info!(
                status = ?state.progress.status,
                current_block = ?state.progress.current_block,
                blocks_processed = state.progress.blocks_processed,
                warmup_total_blocks = state.progress.warmup_total_blocks,
                live_blocks_processed = state.progress.live_blocks_processed,
                txs_processed = state.progress.txs_processed,
                tx_failures = state.progress.tx_failures,
                tracked_tokens = state.progress.tracked_tokens,
                tracked_v2_pools = state.progress.tracked_v2_pools,
                block_source = ?state.progress.last_block_source,
                disk_cache_hits = state.progress.processed_block_disk_cache_hits,
                disk_cache_misses = state.progress.processed_block_disk_cache_misses,
                token_apply_ms = ?state.progress.last_block_token_apply_ms,
                "live token runtime applied block"
            );
        }
        event
    }

    async fn mark_live(&self) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTokenStatus::Live;
        state.progress.live_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
        let event = LiveTokenEvent::RuntimeLive {
            id: state.progress.id.clone().unwrap_or_default(),
            current_block: state.progress.current_block,
        };
        tracing::info!(
            live_id = ?state.progress.id,
            tracked_tokens = state.progress.tracked_tokens,
            tracked_v2_pools = state.progress.tracked_v2_pools,
            "live token runtime warmup completed; entering live tail"
        );
        drop(state);
        let _ = self.inner.event_tx.send(event);
    }

    async fn mark_stopped(&self) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTokenStatus::Stopped;
        state.progress.completed_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
        let event = LiveTokenEvent::RuntimeStopped {
            id: state.progress.id.clone(),
            current_block: state.progress.current_block,
        };
        drop(state);
        let _ = self.inner.event_tx.send(event);
    }

    async fn mark_failed(&self, error: LiveTokenError) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTokenStatus::Failed;
        state.progress.last_error = Some(error.message.clone());
        state.progress.completed_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
        let event = LiveTokenEvent::RuntimeFailed {
            id: state.progress.id.clone(),
            block_number: error.block_number,
            message: error.message.clone(),
        };
        state.errors.push(error);
        drop(state);
        let _ = self.inner.event_tx.send(event);
    }

    async fn record_stream_events(&self, count: u64, last_stream_id: Option<String>) {
        let mut state = self.inner.state.write().await;
        state.progress.live_stream_events += count;
        state.progress.last_stream_id = last_stream_id;
        state.progress.updated_at_unix_secs = now_unix_secs();
    }

    async fn record_gap_blocks(&self, count: u64) {
        let mut state = self.inner.state.write().await;
        state.progress.live_gap_blocks_caught_up += count;
        state.progress.updated_at_unix_secs = now_unix_secs();
    }
}

#[async_trait]
impl LiveTokenReader for LiveTokenRuntime {
    async fn progress(&self) -> LiveTokenProgress {
        self.progress().await
    }

    async fn token_snapshots(&self) -> Vec<LiveTokenSnapshot> {
        let state = self.inner.state.read().await;
        let mut snapshots = state
            .processor
            .registry()
            .tokens
            .values()
            .map(LiveTokenSnapshot::from_token)
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| {
            left.latest_activity_block
                .cmp(&right.latest_activity_block)
                .reverse()
                .then(left.contract_address.cmp(&right.contract_address))
        });
        snapshots
    }

    async fn token_snapshot(&self, token_address: &str) -> Option<LiveTokenSnapshot> {
        let state = self.inner.state.read().await;
        state
            .processor
            .registry()
            .tokens
            .get(&normalize_address(token_address))
            .map(LiveTokenSnapshot::from_token)
    }

    fn subscribe(&self) -> broadcast::Receiver<LiveTokenEvent> {
        self.inner.event_tx.subscribe()
    }
}

fn apply_report(
    state: &mut LiveTokenState,
    report: TokenBlockUpdateReport,
    retention_report: Option<eth_token::manager::LiveTokenRetentionReport>,
    loaded: LiveBlockLoad,
    token_apply_ms: u128,
    is_live_tail: bool,
) -> LiveTokenEvent {
    let updated_tokens = report.updated_token_addresses.clone();
    let created_tokens = report.created_token_addresses.clone();
    let block_hash = report.block_hash.clone();
    let block_number = report.block_number;

    state.progress.current_block = Some(report.block_number);
    state.progress.blocks_processed += 1;
    if is_live_tail {
        state.progress.live_blocks_processed += 1;
    }
    state.progress.txs_scanned += report.transaction_count;
    state.progress.txs_processed += report.processed_transaction_count;
    state.progress.tx_failures += report.failed_transaction_count;
    state.progress.token_update_reports += report.token_updates.len();
    state.progress.last_block_upstream_ms = Some(loaded.upstream_ms);
    state.progress.last_block_token_apply_ms = Some(token_apply_ms);
    state.progress.last_block_disk_cache_read_ms = Some(loaded.disk_cache_read_ms);
    state.progress.last_block_disk_cache_write_ms = Some(loaded.disk_cache_write_ms);
    state.progress.last_block_source = Some(loaded.source.to_string());
    if loaded.disk_cache_hit {
        state.progress.processed_block_disk_cache_hits += 1;
    } else {
        state.progress.processed_block_disk_cache_misses += 1;
    }
    state.progress.updated_at_unix_secs = now_unix_secs();

    state.created_tokens.extend(created_tokens);
    state.updated_tokens.extend(updated_tokens.clone());

    let mut updated_v2_pools = Vec::new();
    for update in report.token_updates {
        state
            .discovered_v2_pools
            .extend(update.discovered_uniswap_v2_pools);
        updated_v2_pools.extend(update.updated_uniswap_v2_pools.clone());
        state
            .updated_v2_pools
            .extend(update.updated_uniswap_v2_pools);
    }
    updated_v2_pools.sort();
    updated_v2_pools.dedup();

    for error in report.transaction_errors {
        state.errors.push(LiveTokenError {
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

    LiveTokenEvent::BlockApplied {
        block_number,
        block_hash,
        updated_tokens,
        updated_v2_pools,
    }
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn status_label(status: &LiveTokenStatus) -> &'static str {
    match status {
        LiveTokenStatus::Idle => "idle",
        LiveTokenStatus::Warming => "warming",
        LiveTokenStatus::Live => "live",
        LiveTokenStatus::Stopping => "stopping",
        LiveTokenStatus::Stopped => "stopped",
        LiveTokenStatus::Failed => "failed",
    }
}

fn should_log_block_apply(progress: &LiveTokenProgress, is_live_tail: bool) -> bool {
    is_live_tail
        || progress.blocks_processed == 1
        || progress.blocks_processed % 100 == 0
        || (progress.warmup_total_blocks > 0
            && progress.blocks_processed == progress.warmup_total_blocks)
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}
