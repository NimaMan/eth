use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use eth_ops_events::{
    emit_bottleneck, emit_issue, PipelineBottleneckSample, PipelineImpact, PipelineIssue,
    PipelineSeverity,
};
use eth_token::chain_metadata::{
    LiveRethChainMetadataProvider, RethChainMetadataProvider, TokenDiscoveryProvider,
};
use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use serde_json::json;
use tokio::sync::{broadcast, watch, Mutex, RwLock, RwLockReadGuard};
use tx_processor::{
    load_processed_block, sealed_header_from_processed_block_header, BlockProcessor,
    BlockStateSession, LivePoolBuySellSimulator, LiveProcessedBlock, LiveStateDiffFrame,
    LoadedProcessedBlock as LiveBlockLoad, ProcessedBlockReplayStoreWriter, ProcessedBlockSource,
};

use super::apply_report::{apply_report, push_bottleneck, push_issue};
use super::config::LiveTokenRuntimeConfig;
use super::event::LiveTokenEvent;
use super::helpers::{
    normalize_address, panic_payload_message, should_log_block_apply, status_label,
};
use super::progress::{
    LiveTokenError, LiveTokenProgress, LiveTokenStatus, ResolvedLiveTokenRuntimeRequest,
    StartLiveTokenRuntimeRequest,
};
use super::snapshot::LiveTokenSnapshot;
use super::state::LiveTokenState;
use super::time::now_unix_secs;

const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";
const LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET: &str = "live_token_apply_profile";

fn live_error_from_report(
    block_number: Option<u64>,
    tx_index: Option<u64>,
    tx_hash: Option<String>,
    error: &eyre::Report,
    mut context: BTreeMap<String, String>,
) -> LiveTokenError {
    if let Some(root_error) = error.chain().last() {
        context.insert("root_error".to_string(), root_error.to_string());
    }

    LiveTokenError::new(block_number, tx_index, tx_hash, error.to_string())
        .with_detail(error_chain_detail(error))
        .with_context_map(context)
}

fn phase_context(phase: &str) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("phase".to_string(), phase.to_string());
    context
}

fn error_chain_detail(error: &eyre::Report) -> String {
    error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

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
    direct_live_block_sessions: Mutex<BTreeMap<u64, BlockStateSession>>,
    event_tx: broadcast::Sender<LiveTokenEvent>,
    shutdown_tx: watch::Sender<bool>,
}

#[async_trait]
pub trait LiveTokenReader: Send + Sync {
    async fn progress(&self) -> LiveTokenProgress;
    async fn token_snapshots(&self) -> Vec<LiveTokenSnapshot>;
    async fn token_snapshot(&self, token_address: &str) -> Option<LiveTokenSnapshot>;
    fn subscribe(&self) -> broadcast::Receiver<LiveTokenEvent>;
}

#[derive(Debug)]
pub struct LiveBlockUpdate {
    loaded: LiveBlockLoad,
    state_diffs: Option<Vec<LiveStateDiffFrame>>,
}

impl LiveBlockUpdate {
    pub fn from_live_processed_block(
        processed: LiveProcessedBlock,
        disk_cache_write_ms: u128,
    ) -> Self {
        let upstream_ms = processed
            .processed_at
            .signed_duration_since(processed.head_arrival)
            .num_milliseconds()
            .max(0) as u128;
        let state_diffs = processed.state_diffs;
        Self {
            loaded: LiveBlockLoad {
                block: processed.processed_block,
                upstream_ms,
                disk_cache_hit: false,
                disk_cache_read_ms: 0,
                disk_cache_write_ms,
                source: ProcessedBlockSource::LiveDirect.as_str(),
            },
            state_diffs,
        }
    }

    pub fn block_number(&self) -> u64 {
        self.loaded.block.header.number
    }
}

impl LiveTokenRuntime {
    pub fn new(
        config: LiveTokenRuntimeConfig,
        provider: Arc<RethQueryProvider>,
        processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        let history_limit = config.history_limit;
        let (event_tx, _) = broadcast::channel(1024);
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            inner: Arc::new(LiveTokenRuntimeInner {
                config,
                provider,
                processed_block_replay_store,
                state: RwLock::new(LiveTokenState::idle(history_limit)),
                stop_requested: AtomicBool::new(false),
                next_id: AtomicU64::new(1),
                task: Mutex::new(None),
                direct_live_block_sessions: Mutex::new(BTreeMap::new()),
                event_tx,
                shutdown_tx,
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
        let _ = self.inner.shutdown_tx.send(false);
        self.inner.direct_live_block_sessions.lock().await.clear();

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
                local_runtime.block_on(runtime_for_failure.mark_failed(
                    LiveTokenError::new(None, None, None, message).with_context("phase", "panic"),
                ));
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
        let _ = self.inner.shutdown_tx.send(true);
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

    pub async fn apply_live_block_update(&self, update: LiveBlockUpdate) -> Result<()> {
        let LiveBlockUpdate {
            loaded,
            state_diffs,
        } = update;
        let block_number = loaded.block.header.number;
        {
            let state = self.inner.state.read().await;
            if state.progress.status != LiveTokenStatus::Live {
                bail!(
                    "live token runtime is {}; cannot apply live block update {}",
                    status_label(&state.progress.status),
                    block_number
                );
            }
        }

        let discovery_provider = LiveRethChainMetadataProvider::new(self.inner.provider.as_ref());
        let pool_simulator =
            LivePoolBuySellSimulator::from_simulator(self.inner.provider.simulator().clone());
        self.apply_loaded_block(
            block_number,
            true,
            loaded,
            &discovery_provider,
            &pool_simulator,
            state_diffs.as_deref(),
        )
        .await
    }

    pub async fn fail_runtime(&self, message: impl Into<String>, phase: impl Into<String>) {
        self.fail_runtime_error(
            LiveTokenError::new(None, None, None, message.into())
                .with_context("phase", phase.into()),
        )
        .await;
    }

    pub async fn fail_runtime_error(&self, error: LiveTokenError) {
        self.mark_failed(error).await;
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
                self.mark_failed(live_error_from_report(
                    Some(block_number),
                    None,
                    None,
                    &error,
                    phase_context("warmup"),
                ))
                .await;
                return;
            }
        }

        self.mark_live().await;
        let mut shutdown_rx = self.inner.shutdown_tx.subscribe();
        loop {
            if self.inner.stop_requested.load(Ordering::SeqCst) {
                self.mark_stopped().await;
                return;
            }

            if shutdown_rx.changed().await.is_err() || *shutdown_rx.borrow() {
                self.mark_stopped().await;
                return;
            }
        }
    }

    async fn apply_block<P>(
        &self,
        block_number: u64,
        is_live_tail: bool,
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
        )
        .await?;

        self.apply_loaded_block(
            block_number,
            is_live_tail,
            loaded,
            discovery_provider,
            pool_simulator,
            None,
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
        live_state_diffs: Option<&[LiveStateDiffFrame]>,
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
        let live_block_sessions: StdMutex<BTreeMap<u64, BlockStateSession>> =
            StdMutex::new(BTreeMap::new());
        if is_live_tail {
            match live_state_diffs {
                Some(state_diffs) => {
                    match self
                        .build_direct_live_block_session(&loaded, pool_simulator, state_diffs)
                        .await
                    {
                        Ok(session) => {
                            live_block_sessions
                                .lock()
                                .map_err(|err| {
                                    eyre::eyre!("direct live block session lock poisoned: {err}")
                                })?
                                .insert(block_number, session.clone());
                            self.remember_direct_live_block_session(block_number, session)
                                .await;
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                                block_number,
                                error = %error,
                                "failed to build direct live block state session"
                            );
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        block_number,
                        "live block update did not include state diffs for direct state session"
                    );
                }
            }
        }

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
        let report = if is_live_tail {
            let live_discovery_provider =
                LiveRethChainMetadataProvider::with_direct_live_block_sessions(
                    self.inner.provider.as_ref(),
                    &live_block_sessions,
                );
            state
                .processor
                .process_block_live_with_discovery_provider_and_sessions(
                    &loaded.block,
                    &live_discovery_provider,
                    pool_simulator,
                    &live_block_sessions,
                    true,
                )
                .await
        } else {
            state
                .processor
                .process_block_live_with_discovery_provider(
                    &loaded.block,
                    discovery_provider,
                    pool_simulator,
                )
                .await
        };
        let process_block_us = process_block_started.elapsed().as_micros();
        if process_block_started.elapsed() > apply_timeout {
            let mut sample = PipelineBottleneckSample::new(
                "eth_chain_server",
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
                transaction_failures = progress.transaction_failures,
                pool_simulation_failures = progress.pool_simulation_failures,
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

    async fn build_direct_live_block_session(
        &self,
        loaded: &LiveBlockLoad,
        pool_simulator: &LivePoolBuySellSimulator,
        state_diffs: &[LiveStateDiffFrame],
    ) -> Result<BlockStateSession> {
        let block_number = loaded.block.header.number;
        let block_hash = loaded.block.header.hash;
        let parent_hash = loaded.block.header.parent_hash;
        let block_header = sealed_header_from_processed_block_header(&loaded.block.header);
        let simulator = pool_simulator.simulator();
        let parent_session = {
            let sessions = self.inner.direct_live_block_sessions.lock().await;
            block_number
                .checked_sub(1)
                .and_then(|parent| sessions.get(&parent).cloned())
        };

        if let Some(parent_session) = parent_session {
            match simulator
                .block_state_session_from_parent_prestate_diffs(
                    &parent_session,
                    block_number,
                    block_hash,
                    parent_hash,
                    block_header.clone(),
                    state_diffs,
                )
                .await
            {
                Ok(session) => return Ok(session),
                Err(error) => {
                    tracing::info!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        block_number,
                        error = %error,
                        "rebuilding direct live block state session from current prestate diffs"
                    );
                }
            }
        }

        simulator
            .block_state_session_from_prestate_diffs(
                block_number,
                block_hash,
                parent_hash,
                block_header,
                state_diffs,
            )
            .await
    }

    async fn remember_direct_live_block_session(
        &self,
        block_number: u64,
        session: BlockStateSession,
    ) {
        let mut sessions = self.inner.direct_live_block_sessions.lock().await;
        sessions.insert(block_number, session);
        while sessions.len() > 16 {
            let Some(oldest) = sessions.keys().next().copied() else {
                break;
            };
            sessions.remove(&oldest);
        }
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
            "eth_chain_server",
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
        issue.detail = error.detail.clone().or_else(|| Some(error.message.clone()));
        for (key, value) in &error.context {
            issue.context.insert(key.clone(), json!(value));
        }
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
            transaction_failures = state.progress.transaction_failures,
            pool_simulation_failures = state.progress.pool_simulation_failures,
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
            error_detail = ?error.detail,
            error_context = ?error.context,
            "live token tracker failed"
        );
        emit_issue(&issue);
        push_issue(&mut state, issue);
        state.errors.push(error);
        drop(state);
        let _ = self.inner.event_tx.send(event);
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
