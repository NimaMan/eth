use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use eth_live_state::keys;
use eth_pipeline_telemetry::{
    emit_bottleneck, emit_issue, PipelineBottleneckSample, PipelineImpact, PipelineIssue,
    PipelineSeverity,
};
use eth_token::chain_metadata::{
    LiveRethChainMetadataProvider, RethChainMetadataProvider, TokenDiscoveryProvider,
};
use eth_token::tracking::TokenBlockUpdateReport;
use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use serde_json::json;
use tokio::sync::{broadcast, Mutex, RwLock, RwLockReadGuard};
use tx_processor::{
    load_processed_block, BlockProcessor, LivePoolBuySellSimulator, LiveProcessedBlockProvider,
    LoadedProcessedBlock as LiveBlockLoad, ProcessedBlockProviderRetry,
    ProcessedBlockReplayStoreWriter,
};

use super::config::LiveTokenRuntimeConfig;
use super::event::LiveTokenEvent;
use super::helpers::{
    normalize_address, panic_payload_message, should_log_block_apply, status_label,
};
use super::progress::{
    LiveTokenError, LiveTokenProgress, LiveTokenStatus, ResolvedLiveTokenRuntimeRequest,
    StartLiveTokenRuntimeRequest,
};
use super::redis_stream::{missing_blocks_after, RedisBlockStream};
use super::snapshot::LiveTokenSnapshot;
use super::state::LiveTokenState;
use super::time::now_unix_secs;

const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";
const LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET: &str = "live_token_apply_profile";
const MAX_LIVE_ISSUES: usize = 1_000;
const MAX_LIVE_BOTTLENECKS: usize = 1_000;

#[derive(Clone)]
pub struct LiveTokenRuntime {
    inner: Arc<LiveTokenRuntimeInner>,
}

struct LiveTokenRuntimeInner {
    config: LiveTokenRuntimeConfig,
    provider: Arc<RethQueryProvider>,
    processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
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
        processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        let history_limit = config.history_limit;
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(LiveTokenRuntimeInner {
                config,
                provider,
                processed_block_replay_store,
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
        let provider_latest_block = self.inner.provider.get_latest_block().ok();
        let latest_provider_readable_block = match (latest_cached_block, provider_latest_block) {
            (Some(cached), Some(provider_latest)) => {
                let readable = cached.min(provider_latest);
                if cached > provider_latest {
                    tracing::warn!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        latest_cached_block = cached,
                        provider_latest_block = provider_latest,
                        resolved_latest_block = readable,
                        "clamped live token warmup to provider-readable block"
                    );
                }
                Some(readable)
            }
            (Some(cached), None) => Some(cached),
            (None, Some(provider_latest)) => Some(provider_latest),
            (None, None) => None,
        };
        let latest_block = if request.start_block.is_none() || request.end_block.is_none() {
            Some(
                latest_provider_readable_block
                    .ok_or_else(|| eyre::eyre!("could not resolve latest block"))?,
            )
        } else {
            None
        };

        let requested_end_block = request
            .end_block
            .or(latest_block)
            .ok_or_else(|| eyre::eyre!("could not resolve end block"))?;
        let end_block = if let Some(provider_latest_block) = provider_latest_block {
            let clamped = requested_end_block.min(provider_latest_block);
            if requested_end_block > provider_latest_block {
                tracing::warn!(
                    target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                    requested_end_block,
                    provider_latest_block,
                    resolved_end_block = clamped,
                    "clamped live token warmup end block to provider-readable state"
                );
            }
            clamped
        } else {
            requested_end_block
        };
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
        let Some(replay_store_writer) = self.inner.processed_block_replay_store.as_deref() else {
            return Ok(None);
        };
        let coverage = replay_store_writer.disk_cache_store().coverage()?;
        let chain_id = self.inner.provider.chain_id();
        Ok(coverage
            .chains
            .iter()
            .find(|chain| chain.chain_id == chain_id)
            .and_then(|chain| chain.ranges.iter().map(|range| range.end_block).max()))
    }

    async fn run(&self, request: ResolvedLiveTokenRuntimeRequest) {
        tracing::info!(
            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
            live_id = %request.id,
            start_block = request.start_block,
            end_block = request.end_block,
            warmup_blocks = request.warmup_blocks,
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
                    self.processed_block_retry(),
                    &tx_processor,
                    &warmup_discovery_provider,
                    &pool_simulator,
                )
                .await
            {
                tracing::error!(
                    target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                    live_id = %request.id,
                    block_number,
                    error = %error,
                    "live token runtime warmup block failed"
                );
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
                tracing::error!(
                    target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                    live_id = %request.id,
                    error = %error,
                    "live token runtime failed to initialize redis stream"
                );
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
        let live_processed_block_provider = match LiveProcessedBlockProvider::new(
            &self.inner.config.redis_url,
            tx_processor.clone(),
            self.inner.provider.clone(),
            self.inner.processed_block_replay_store.clone(),
            self.processed_block_retry(),
        ) {
            Ok(provider) => provider,
            Err(error) => {
                tracing::error!(
                    target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                    live_id = %request.id,
                    error = %error,
                    "live token runtime failed to initialize live processed block provider"
                );
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
                    &live_processed_block_provider,
                    &live_discovery_provider,
                    &pool_simulator,
                )
                .await
            {
                tracing::error!(
                    target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                    live_id = %request.id,
                    error = %error,
                    "live token runtime live-tail catch-up failed"
                );
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
                    tracing::error!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        live_id = %request.id,
                        last_stream_id = %last_stream_id,
                        error = %error,
                        "live token runtime failed to read redis stream"
                    );
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
                    &live_processed_block_provider,
                    &live_discovery_provider,
                    &pool_simulator,
                )
                .await
            {
                tracing::error!(
                    target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                    live_id = %request.id,
                    error = %error,
                    "live token runtime live-tail catch-up failed after stream event"
                );
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

    fn processed_block_retry(&self) -> ProcessedBlockProviderRetry {
        ProcessedBlockProviderRetry {
            attempts: self.inner.config.processed_block_disk_cache_retry_attempts,
            delay_ms: self.inner.config.processed_block_disk_cache_retry_delay_ms,
        }
    }

    async fn catch_up_to_latest(
        &self,
        stream: &RedisBlockStream,
        live_processed_block_provider: &LiveProcessedBlockProvider,
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

        let mut applied_blocks = 0_u64;
        for block_number in missing {
            if self.inner.stop_requested.load(Ordering::SeqCst) {
                return Ok(());
            }
            let applied = self
                .apply_live_tail_block(
                    block_number,
                    live_processed_block_provider,
                    discovery_provider,
                    pool_simulator,
                )
                .await?;
            if !applied {
                break;
            }
            applied_blocks += 1;
        }
        if applied_blocks > 1 {
            self.record_gap_blocks(applied_blocks - 1).await;
        }
        Ok(())
    }

    async fn apply_live_tail_block<P>(
        &self,
        block_number: u64,
        live_processed_block_provider: &LiveProcessedBlockProvider,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> Result<bool>
    where
        P: TokenDiscoveryProvider,
    {
        let loaded = live_processed_block_provider
            .load_block(block_number)
            .await?;
        self.apply_loaded_block(
            block_number,
            true,
            loaded,
            discovery_provider,
            pool_simulator,
        )
        .await?;
        Ok(true)
    }

    async fn apply_block<P>(
        &self,
        block_number: u64,
        is_live_tail: bool,
        retry: ProcessedBlockProviderRetry,
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
            self.inner.processed_block_replay_store.clone(),
            block_number,
            retry,
        )
        .await?;

        self.apply_loaded_block(
            block_number,
            is_live_tail,
            loaded,
            discovery_provider,
            pool_simulator,
        )
        .await
    }

    async fn apply_loaded_block<P>(
        &self,
        block_number: u64,
        is_live_tail: bool,
        loaded: LiveBlockLoad,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> Result<()>
    where
        P: TokenDiscoveryProvider,
    {
        let block_apply_started = Instant::now();
        let block_transaction_count = loaded.block.transactions.len();
        let block_source = loaded.source;
        let upstream_ms = loaded.upstream_ms;
        let disk_cache_hit = loaded.disk_cache_hit;
        let disk_cache_read_ms = loaded.disk_cache_read_ms;
        let disk_cache_write_ms = loaded.disk_cache_write_ms;

        let state_lock_started = Instant::now();
        let mut state = self.inner.state.write().await;
        let state_lock_wait_us = state_lock_started.elapsed().as_micros();
        state.progress.current_block = Some(block_number);
        state.progress.updated_at_unix_secs = now_unix_secs();

        let apply_started = Instant::now();
        let apply_timeout = Duration::from_millis(self.inner.config.block_apply_timeout_ms);
        let tracked_tokens_before = state.processor.registry().tokens.len();
        let tracked_pools_before: usize = state
            .processor
            .registry()
            .tokens
            .values()
            .map(|token| token.pool_count())
            .sum();
        let process_block_started = Instant::now();
        let report = state
            .processor
            .process_block_live_with_discovery_provider(
                &loaded.block,
                discovery_provider,
                pool_simulator,
            )
            .await;
        let process_block_us = process_block_started.elapsed().as_micros();
        if process_block_started.elapsed() > apply_timeout {
            let mut sample = PipelineBottleneckSample::new(
                "eth_token_server",
                "live_tracker",
                "block_apply",
                process_block_us / 1_000,
                "slow live token block apply",
            );
            sample.run_id = state.progress.id.clone();
            sample.block_number = Some(block_number);
            sample.threshold_ms = Some(u128::from(self.inner.config.block_apply_timeout_ms));
            sample
                .work_units
                .insert("txs".to_string(), json!(block_transaction_count));
            sample.work_units.insert(
                "tracked_tokens_before".to_string(),
                json!(tracked_tokens_before),
            );
            sample.work_units.insert(
                "tracked_pools_before".to_string(),
                json!(tracked_pools_before),
            );
            sample
                .breakdown_ms
                .insert("process_block".to_string(), json!(process_block_us / 1_000));
            sample
                .breakdown_ms
                .insert("upstream".to_string(), json!(upstream_ms));
            sample
                .breakdown_ms
                .insert("disk_cache_read".to_string(), json!(disk_cache_read_ms));
            sample
                .breakdown_ms
                .insert("disk_cache_write".to_string(), json!(disk_cache_write_ms));
            emit_bottleneck(&sample);
            push_bottleneck(&mut state, sample);
            tracing::warn!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                block_number,
                is_live_tail,
                source = block_source,
                txs = block_transaction_count,
                tracked_tokens_before,
                tracked_pools_before,
                slow_threshold_ms = self.inner.config.block_apply_timeout_ms,
                process_block_ms = process_block_us / 1_000,
                upstream_ms,
                disk_cache_hit,
                disk_cache_read_ms,
                disk_cache_write_ms,
                "slow live token block apply"
            );
        }
        let retention_started = Instant::now();
        let retention_report = state.processor.apply_index_retention_policy(block_number);
        let retention_us = retention_started.elapsed().as_micros();
        let token_apply_us = apply_started.elapsed().as_micros();
        let token_apply_ms = token_apply_us / 1_000;

        let state_update_started = Instant::now();
        let event = apply_report(
            &mut state,
            report,
            retention_report,
            loaded,
            token_apply_ms,
            is_live_tail,
        );
        let should_log_progress = should_log_block_apply(&state.progress, is_live_tail);
        let progress = should_log_progress.then(|| state.progress.clone());
        let state_update_us = state_update_started.elapsed().as_micros();
        let block_apply_wall_us = block_apply_started.elapsed().as_micros();
        let measured_us = state_lock_wait_us
            .saturating_add(token_apply_us)
            .saturating_add(state_update_us);
        let unaccounted_us = block_apply_wall_us.saturating_sub(measured_us);
        tracing::info!(
            target: LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET,
            block_number,
            is_live_tail,
            source = block_source,
            txs = block_transaction_count,
            tracked_tokens_before,
            tracked_pools_before,
            state_lock_wait_us,
            process_block_us,
            retention_us,
            token_apply_us,
            state_update_us,
            block_apply_wall_us,
            unaccounted_us,
            upstream_us = upstream_ms.saturating_mul(1_000),
            disk_cache_read_us = disk_cache_read_ms.saturating_mul(1_000),
            disk_cache_write_us = disk_cache_write_ms.saturating_mul(1_000),
            state_lock_wait_ms = state_lock_wait_us / 1_000,
            process_block_ms = process_block_us / 1_000,
            retention_ms = retention_us / 1_000,
            token_apply_ms,
            state_update_ms = state_update_us / 1_000,
            block_apply_wall_ms = block_apply_wall_us / 1_000,
            unaccounted_ms = unaccounted_us / 1_000,
            upstream_ms,
            disk_cache_hit,
            disk_cache_read_ms,
            disk_cache_write_ms,
            "live token apply profile"
        );
        if let Some(progress) = progress {
            tracing::info!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                status = ?progress.status,
                current_block = ?progress.current_block,
                blocks_processed = progress.blocks_processed,
                warmup_total_blocks = progress.warmup_total_blocks,
                live_blocks_processed = progress.live_blocks_processed,
                txs_processed = progress.txs_processed,
                tx_failures = progress.tx_failures,
                tracked_tokens = progress.tracked_tokens,
                tracked_pools = progress.tracked_pools,
                tracked_v2_pools = progress.tracked_v2_pools,
                tracked_v3_pools = progress.tracked_v3_pools,
                tracked_v4_pools = progress.tracked_v4_pools,
                block_source = ?progress.last_block_source,
                disk_cache_hits = progress.processed_block_disk_cache_hits,
                disk_cache_misses = progress.processed_block_disk_cache_misses,
                token_apply_ms = ?progress.last_block_token_apply_ms,
                "live token runtime applied block"
            );
        }
        drop(state);
        let _ = self.inner.event_tx.send(event);
        Ok(())
    }

    async fn mark_live(&self) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTokenStatus::Live;
        state.progress.live_at_unix_secs = Some(now_unix_secs());
        state.progress.last_error = None;
        state.progress.updated_at_unix_secs = now_unix_secs();
        let event = LiveTokenEvent::RuntimeLive {
            id: state.progress.id.clone().unwrap_or_default(),
            current_block: state.progress.current_block,
        };
        tracing::info!(
            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
            live_id = ?state.progress.id,
            tracked_tokens = state.progress.tracked_tokens,
            tracked_pools = state.progress.tracked_pools,
            tracked_v2_pools = state.progress.tracked_v2_pools,
            tracked_v3_pools = state.progress.tracked_v3_pools,
            tracked_v4_pools = state.progress.tracked_v4_pools,
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
        let mut issue = PipelineIssue::new(
            "eth_token_server",
            "live_tracker",
            "runtime",
            PipelineSeverity::Error,
            PipelineImpact::ServiceDown,
            "live_tracker_failed",
            "Live token tracker failed",
        );
        issue.run_id = state.progress.id.clone();
        issue.fatal = true;
        issue.retryable = true;
        issue.block_number = error.block_number;
        issue.tx_index = error.tx_index;
        issue.tx_hash = error.tx_hash.clone();
        issue.detail = Some(error.message.clone());
        issue.refresh_ids();
        let event = LiveTokenEvent::RuntimeFailed {
            id: state.progress.id.clone(),
            block_number: error.block_number,
            message: error.message.clone(),
        };
        tracing::error!(
            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
            live_id = ?state.progress.id,
            status = ?state.progress.status,
            current_block = ?state.progress.current_block,
            warmup_start_block = ?state.progress.warmup_start_block,
            warmup_end_block = ?state.progress.warmup_end_block,
            blocks_processed = state.progress.blocks_processed,
            warmup_total_blocks = state.progress.warmup_total_blocks,
            live_blocks_processed = state.progress.live_blocks_processed,
            txs_processed = state.progress.txs_processed,
            tx_failures = state.progress.tx_failures,
            tracked_tokens = state.progress.tracked_tokens,
            tracked_pools = state.progress.tracked_pools,
            tracked_v2_pools = state.progress.tracked_v2_pools,
            tracked_v3_pools = state.progress.tracked_v3_pools,
            tracked_v4_pools = state.progress.tracked_v4_pools,
            block_source = ?state.progress.last_block_source,
            last_block_upstream_ms = ?state.progress.last_block_upstream_ms,
            last_block_token_apply_ms = ?state.progress.last_block_token_apply_ms,
            last_block_disk_cache_read_ms = ?state.progress.last_block_disk_cache_read_ms,
            last_block_disk_cache_write_ms = ?state.progress.last_block_disk_cache_write_ms,
            error_block_number = ?error.block_number,
            error_tx_index = ?error.tx_index,
            error_tx_hash = ?error.tx_hash,
            error = %error.message,
            "live token tracker failed"
        );
        emit_issue(&issue);
        push_issue(&mut state, issue);
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
    retention_report: Option<eth_token::tracking::LiveTokenRetentionReport>,
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
    state.progress.last_error = None;
    if loaded.disk_cache_hit {
        state.progress.processed_block_disk_cache_hits += 1;
    } else {
        state.progress.processed_block_disk_cache_misses += 1;
    }
    state.progress.updated_at_unix_secs = now_unix_secs();

    state.created_tokens.extend(created_tokens);
    state.updated_tokens.extend(updated_tokens.clone());

    let mut updated_v2_pools = Vec::new();
    let mut updated_v3_pools = Vec::new();
    let mut updated_v4_pools = Vec::new();
    for update in report.token_updates {
        state
            .discovered_v2_pools
            .extend(update.discovered_known_v2_pools);
        updated_v2_pools.extend(update.updated_known_v2_pools.clone());
        state.updated_v2_pools.extend(update.updated_known_v2_pools);
        state
            .discovered_v3_pools
            .extend(update.discovered_uniswap_v3_pools);
        updated_v3_pools.extend(update.updated_uniswap_v3_pools.clone());
        state
            .updated_v3_pools
            .extend(update.updated_uniswap_v3_pools);
        state
            .discovered_v4_pools
            .extend(update.discovered_uniswap_v4_pools);
        updated_v4_pools.extend(update.updated_uniswap_v4_pools.clone());
        state
            .updated_v4_pools
            .extend(update.updated_uniswap_v4_pools);
    }
    updated_v2_pools.sort();
    updated_v2_pools.dedup();
    updated_v3_pools.sort();
    updated_v3_pools.dedup();
    updated_v4_pools.sort();
    updated_v4_pools.dedup();

    let run_id = state.progress.id.clone();
    for error in report.transaction_errors {
        let issue = PipelineIssue::live_transaction_error(
            run_id.clone(),
            report.block_number,
            error.tx_index,
            error.tx_hash.clone(),
            error.message.clone(),
        );
        emit_issue(&issue);
        push_issue(state, issue);
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
    state.progress.discovered_v3_pools_unique = state.discovered_v3_pools.len();
    state.progress.updated_v3_pools_unique = state.updated_v3_pools.len();
    state.progress.discovered_v4_pools_unique = state.discovered_v4_pools.len();
    state.progress.updated_v4_pools_unique = state.updated_v4_pools.len();
    state.progress.tracked_tokens = state.processor.registry().tokens.len();
    state.progress.indexed_tokens = state.processor.block_processor().token_index.entries.len();
    state.progress.indexed_pools = state
        .processor
        .block_processor()
        .token_index
        .pool_to_token
        .len();
    state.progress.indexed_v2_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum();
    state.progress.tracked_v2_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v2_pools.len())
        .sum();
    state.progress.indexed_v3_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v3_pools.len())
        .sum();
    state.progress.tracked_v3_pools = state.progress.indexed_v3_pools;
    state.progress.indexed_v4_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.v4_pools.len())
        .sum();
    state.progress.tracked_v4_pools = state.progress.indexed_v4_pools;
    state.progress.tracked_pools = state
        .processor
        .registry()
        .tokens
        .values()
        .map(|token| token.pool_count())
        .sum();

    LiveTokenEvent::BlockApplied {
        block_number,
        block_hash,
        updated_tokens,
        updated_v2_pools,
        updated_v3_pools,
        updated_v4_pools,
    }
}

fn push_issue(state: &mut LiveTokenState, issue: PipelineIssue) {
    state.issues.push(issue);
    if state.issues.len() > MAX_LIVE_ISSUES {
        let excess = state.issues.len() - MAX_LIVE_ISSUES;
        state.issues.drain(0..excess);
    }
}

fn push_bottleneck(state: &mut LiveTokenState, sample: PipelineBottleneckSample) {
    state.bottlenecks.push(sample);
    if state.bottlenecks.len() > MAX_LIVE_BOTTLENECKS {
        let excess = state.bottlenecks.len() - MAX_LIVE_BOTTLENECKS;
        state.bottlenecks.drain(0..excess);
    }
}
