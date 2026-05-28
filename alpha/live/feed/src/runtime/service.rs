use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use eth_token::chain_metadata::RethChainMetadataProvider;
use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use tokio::sync::{broadcast, watch, Mutex, RwLock, RwLockReadGuard};
use tx_processor::{
    BlockProcessor, BlockStateSession, LivePoolBuySellSimulator, ProcessedBlockReplayStoreWriter,
};
use tx_simulator::{
    InMemoryLiveBlockStateProvider, LiveStateStatus, LiveTxSimulator, UnsignedTxChainSimulation,
};

use super::config::LiveTokenRuntimeConfig;
use super::errors::{live_error_from_report, phase_context};
use super::event::LiveTokenEvent;
use super::helpers::{panic_payload_message, status_label};
use super::progress::{
    LiveTokenError, LiveTokenProgress, LiveTokenStatus, ResolvedLiveTokenRuntimeRequest,
    StartLiveTokenRuntimeRequest,
};
use super::state::LiveTokenState;
use super::time::now_unix_secs;

pub(super) const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";
pub(super) const LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET: &str = "live_token_apply_profile";

#[derive(Clone)]
pub struct LiveTokenRuntime {
    pub(super) inner: Arc<LiveTokenRuntimeInner>,
}

pub(super) struct LiveTokenRuntimeInner {
    pub(super) config: LiveTokenRuntimeConfig,
    pub(super) provider: Arc<RethQueryProvider>,
    pub(super) processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    pub(super) state: RwLock<LiveTokenState>,
    pub(super) stop_requested: AtomicBool,
    pub(super) next_id: AtomicU64,
    pub(super) task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub(super) live_tx_simulator: LiveTxSimulator,
    pub(super) direct_live_block_sessions: Mutex<BTreeMap<u64, BlockStateSession>>,
    pub(super) event_tx: broadcast::Sender<LiveTokenEvent>,
    pub(super) shutdown_tx: watch::Sender<bool>,
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
        let live_tx_simulator = LiveTxSimulator::new(
            provider.simulator().clone(),
            InMemoryLiveBlockStateProvider::new(),
        );
        Self {
            inner: Arc::new(LiveTokenRuntimeInner {
                config,
                provider,
                processed_block_replay_store,
                state: RwLock::new(LiveTokenState::idle(history_limit)),
                stop_requested: AtomicBool::new(false),
                next_id: AtomicU64::new(1),
                task: Mutex::new(None),
                live_tx_simulator,
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
        self.inner.live_tx_simulator.provider().clear()?;

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

    pub async fn live_simulation_chain_at(
        &self,
        block_number: u64,
    ) -> Result<(LiveStateStatus, UnsignedTxChainSimulation)> {
        let status = self
            .inner
            .live_tx_simulator
            .state_status_at(block_number)
            .await?;
        let chain = self
            .inner
            .live_tx_simulator
            .start_chain_at(block_number)
            .await?;
        Ok((status, chain))
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
}
