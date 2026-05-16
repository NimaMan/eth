use std::sync::Arc;
use std::time::Instant;
use std::{
    any::Any,
    panic::{self, AssertUnwindSafe},
};

use eth_live_feed::{
    LiveBlockUpdate, LiveTokenError, LiveTokenEvent, LiveTokenReader, LiveTokenRuntime,
    ResolvedLiveTokenRuntimeRequest, StartLiveTokenRuntimeRequest,
};
use eyre::{bail, Result, WrapErr};
use reth_chain_query::RethQueryProvider;
use tokio::sync::{watch, Mutex};
use tx_processor::{
    LiveBlockProcessor, LiveBlockProcessorConfig, LiveProcessedBlock, ProcessedBlock,
    ProcessedBlockReplayStoreWriter,
};

const LIVE_CHAIN_RUNTIME_PHASE: &str = "live_chain_runtime";
const LIVE_BLOCK_APPLY_ERROR_PREFIX: &str = "failed to apply live block update ";

#[derive(Clone, Debug)]
pub struct LiveChainRuntimeConfig {
    pub execution_rpc: String,
    pub execution_ws: String,
}

#[derive(Clone)]
pub struct LiveChainRuntime {
    inner: Arc<LiveChainRuntimeInner>,
}

struct LiveChainRuntimeInner {
    config: LiveChainRuntimeConfig,
    provider: Arc<RethQueryProvider>,
    live_tracker: LiveTokenRuntime,
    processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    shutdown_tx: watch::Sender<bool>,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl LiveChainRuntime {
    pub fn new(
        config: LiveChainRuntimeConfig,
        provider: Arc<RethQueryProvider>,
        live_tracker: LiveTokenRuntime,
        processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            inner: Arc::new(LiveChainRuntimeInner {
                config,
                provider,
                live_tracker,
                processed_block_replay_store,
                shutdown_tx,
                task: Mutex::new(None),
            }),
        }
    }

    pub async fn start(
        &self,
        request: StartLiveTokenRuntimeRequest,
    ) -> Result<ResolvedLiveTokenRuntimeRequest> {
        {
            let task = self.inner.task.lock().await;
            if task.as_ref().is_some_and(|handle| !handle.is_finished()) {
                bail!("live chain runtime is already running");
            }
        }

        let _ = self.inner.shutdown_tx.send(false);
        let resolved = self.inner.live_tracker.start(request).await?;
        let runtime = self.clone();
        let live_id = resolved.id.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let local_runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("live-chain-runtime")
                .enable_all()
                .build()
                .expect("failed to build live chain runtime thread");
            let run_result = panic::catch_unwind(AssertUnwindSafe(|| {
                local_runtime.block_on(runtime.run(live_id))
            }));
            let live_error = match run_result {
                Ok(Ok(())) => return,
                Ok(Err(error)) => {
                    let error_detail = error_chain_detail(&error);
                    tracing::error!(
                        target: "live_chain_runtime",
                        error = %error,
                        error_detail = %error_detail,
                        error_debug = ?error,
                        "live chain runtime failed"
                    );
                    live_runtime_error_from_report(&error, LIVE_CHAIN_RUNTIME_PHASE)
                }
                Err(payload) => {
                    let message = format!(
                        "live chain runtime task panicked: {}",
                        panic_payload_message(payload.as_ref())
                    );
                    tracing::error!(
                        target: "live_chain_runtime",
                        error = %message,
                        "live chain runtime failed"
                    );
                    LiveTokenError::new(None, None, None, message)
                        .with_context("phase", LIVE_CHAIN_RUNTIME_PHASE)
                        .with_context("runtime_failure_kind", "panic")
                }
            };
            local_runtime.block_on(runtime.inner.live_tracker.fail_runtime_error(live_error));
        });

        let mut task = self.inner.task.lock().await;
        *task = Some(handle);
        Ok(resolved)
    }

    pub async fn stop(&self) -> eth_live_feed::LiveTokenProgress {
        let _ = self.inner.shutdown_tx.send(true);
        self.inner.live_tracker.stop().await
    }

    async fn run(&self, live_id: String) -> Result<()> {
        if !self.wait_until_live(&live_id).await? {
            return Ok(());
        }
        let processor_config = LiveBlockProcessorConfig::default()
            .with_execution_rpc(self.inner.config.execution_rpc.clone())
            .with_execution_ws(self.inner.config.execution_ws.clone());
        let mut processor =
            LiveBlockProcessor::connect(self.inner.provider.clone(), processor_config)
                .await
                .wrap_err("failed to connect live block processor")?;
        let mut shutdown_rx = self.inner.shutdown_tx.subscribe();

        tracing::info!(
            target: "live_chain_runtime",
            live_id = %live_id,
            execution_rpc = %self.inner.config.execution_rpc,
            execution_ws = %self.inner.config.execution_ws,
            "live chain runtime started direct block loop"
        );

        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return Ok(());
                    }
                }
                processed = processor.next_processed_block() => {
                    let processed = processed.wrap_err("failed to process next live block")?;
                    self.apply_processed_with_gap_fill(&processor, processed).await?;
                }
            }
        }
    }

    async fn wait_until_live(&self, live_id: &str) -> Result<bool> {
        let mut events = self.inner.live_tracker.subscribe();
        let mut shutdown_rx = self.inner.shutdown_tx.subscribe();
        loop {
            let progress = self.inner.live_tracker.progress().await;
            if matches!(progress.status, eth_live_feed::LiveTokenStatus::Live) {
                return Ok(true);
            }
            if matches!(progress.status, eth_live_feed::LiveTokenStatus::Failed) {
                bail!("live token runtime failed before live block loop started");
            }

            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return Ok(false);
                    }
                }
                event = events.recv() => {
                    match event {
                        Ok(LiveTokenEvent::RuntimeLive { id, .. }) if id == live_id => return Ok(true),
                        Ok(LiveTokenEvent::RuntimeFailed { message, .. }) => bail!(message),
                        Ok(_) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            bail!("live token event channel closed before runtime became live");
                        }
                    }
                }
            }
        }
    }

    async fn apply_processed_with_gap_fill(
        &self,
        processor: &LiveBlockProcessor,
        processed: LiveProcessedBlock,
    ) -> Result<()> {
        let block_number = processed.execution_info.block_number;
        let current_block = self.inner.live_tracker.progress().await.current_block;
        if current_block.is_some_and(|current| block_number <= current) {
            return Ok(());
        }

        if let Some(current) = current_block {
            if block_number > current.saturating_add(1) {
                for missing_block in current.saturating_add(1)..block_number {
                    let missing = processor
                        .process_block_number(missing_block)
                        .await
                        .wrap_err_with(|| {
                            format!("failed to fill missing live block {missing_block}")
                        })?;
                    self.apply_processed(missing).await?;
                }
            }
        }

        self.apply_processed(processed).await
    }

    async fn apply_processed(&self, processed: LiveProcessedBlock) -> Result<()> {
        let block_number = processed.execution_info.block_number;
        let disk_cache_write_ms = self
            .write_processed_block_if_missing(processed.processed_block.clone())
            .await?;
        let update = LiveBlockUpdate::from_live_processed_block(processed, disk_cache_write_ms);
        self.inner
            .live_tracker
            .apply_live_block_update(update)
            .await
            .wrap_err_with(|| format!("failed to apply live block update {block_number}"))
    }

    async fn write_processed_block_if_missing(&self, block: ProcessedBlock) -> Result<u128> {
        let Some(writer) = self.inner.processed_block_replay_store.clone() else {
            return Ok(0);
        };
        let block_number = block.header.number;
        let started = Instant::now();
        let write =
            tokio::task::spawn_blocking(move || writer.write_processed_block_if_missing(&block))
                .await
                .wrap_err("processed block replay store write task failed")?;
        match write {
            Ok(Some(write)) => Ok(write.disk_cache.write_ms),
            Ok(None) => Ok(0),
            Err(error) => {
                tracing::warn!(
                    target: "live_chain_runtime",
                    block_number,
                    error = %error,
                    elapsed_ms = started.elapsed().as_millis(),
                    "failed to write direct live processed block to replay store"
                );
                Ok(0)
            }
        }
    }
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_string()
}

fn live_runtime_error_from_report(error: &eyre::Report, phase: &str) -> LiveTokenError {
    let mut live_error = LiveTokenError::new(
        live_apply_error_block_number(error),
        None,
        None,
        error.to_string(),
    )
    .with_detail(error_chain_detail(error))
    .with_context("phase", phase)
    .with_context("runtime_failure_kind", "error");

    if let Some(root_error) = error.chain().last() {
        live_error = live_error.with_context("root_error", root_error.to_string());
    }

    live_error
}

fn error_chain_detail(error: &eyre::Report) -> String {
    error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

fn live_apply_error_block_number(error: &eyre::Report) -> Option<u64> {
    error
        .chain()
        .find_map(|source| parse_live_apply_block_number(&source.to_string()))
}

fn parse_live_apply_block_number(message: &str) -> Option<u64> {
    message
        .strip_prefix(LIVE_BLOCK_APPLY_ERROR_PREFIX)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_live_apply_block_number() {
        assert_eq!(
            parse_live_apply_block_number("failed to apply live block update 25106480"),
            Some(25106480)
        );
    }

    #[test]
    fn live_runtime_error_keeps_block_and_error_chain() {
        let error = eyre::eyre!("inner apply failure")
            .wrap_err("failed to apply live block update 25106480");

        let live_error = live_runtime_error_from_report(&error, LIVE_CHAIN_RUNTIME_PHASE);

        assert_eq!(live_error.block_number, Some(25106480));
        assert_eq!(
            live_error.message,
            "failed to apply live block update 25106480"
        );
        assert_eq!(
            live_error.context.get("phase").map(String::as_str),
            Some(LIVE_CHAIN_RUNTIME_PHASE)
        );
        assert_eq!(
            live_error.context.get("root_error").map(String::as_str),
            Some("inner apply failure")
        );
        assert_eq!(
            live_error.detail.as_deref(),
            Some("failed to apply live block update 25106480: inner apply failure")
        );
    }
}
