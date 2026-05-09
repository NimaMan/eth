use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use tokio::sync::RwLock;

use crate::config::TokenServerConfig;
use crate::views::run::RunSummaryView;
use tx_processor::{ProcessedBlockProviderRetry, ProcessedBlockReplayStoreWriter};

use super::pipeline;
use super::RangeIndexStatus;
use super::{RangeIndexJob, ResolvedRangeIndexRequest, StartRangeIndexRequest};

const DEFAULT_HISTORICAL_END_BLOCK_LAG: u64 = 256;

#[derive(Clone)]
pub struct RangeIndexManager {
    inner: Arc<RangeIndexManagerInner>,
}

struct RangeIndexManagerInner {
    config: TokenServerConfig,
    provider: Arc<RethQueryProvider>,
    processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    runs: RwLock<HashMap<String, Arc<RangeIndexJob>>>,
    active_run_id: RwLock<Option<String>>,
    next_id: AtomicU64,
}

#[derive(Debug)]
pub enum StartRangeIndexError {
    ActiveRunConflict { active_run_id: String },
    InvalidRequest(eyre::Report),
}

impl fmt::Display for StartRangeIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActiveRunConflict { active_run_id } => {
                write!(formatter, "active run already exists: {active_run_id}")
            }
            Self::InvalidRequest(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for StartRangeIndexError {}

impl From<eyre::Report> for StartRangeIndexError {
    fn from(error: eyre::Report) -> Self {
        Self::InvalidRequest(error)
    }
}

impl RangeIndexManager {
    pub fn new(
        config: TokenServerConfig,
        provider: Arc<RethQueryProvider>,
        processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        Self {
            inner: Arc::new(RangeIndexManagerInner {
                config,
                provider,
                processed_block_replay_store,
                runs: RwLock::new(HashMap::new()),
                active_run_id: RwLock::new(None),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    pub async fn start_run(
        &self,
        request: StartRangeIndexRequest,
    ) -> std::result::Result<Arc<RangeIndexJob>, StartRangeIndexError> {
        let replace_active = request.replace_active;
        let request = self.resolve_request(request)?;
        self.inner.provider.refresh_static_file_provider()?;

        let mut active_run_id = self.inner.active_run_id.write().await;
        if let Some(current_run_id) = active_run_id.as_ref() {
            let current_run = self.inner.runs.read().await.get(current_run_id).cloned();
            if let Some(current_run) = current_run {
                let progress = current_run.progress().await;
                match active_run_start_decision(Some(&progress.status), replace_active) {
                    ActiveRunStartDecision::Reject => {
                        return Err(StartRangeIndexError::ActiveRunConflict {
                            active_run_id: current_run_id.clone(),
                        });
                    }
                    ActiveRunStartDecision::StopAndStart => {
                        current_run.request_stop();
                        current_run.mark_stopping().await;
                        tracing::info!(
                            run_id = %current_run.id,
                            "requested stop for replaced active token tracking run"
                        );
                    }
                    ActiveRunStartDecision::Start => {}
                }
            }
        }

        let sequence = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        let id = format!("run-{sequence}");
        let run = Arc::new(RangeIndexJob::new(id.clone(), request));

        self.inner.runs.write().await.insert(id, run.clone());
        *active_run_id = Some(run.id.clone());

        let task_run = run.clone();
        let task_provider = self.inner.provider.clone();
        let task_processed_block_replay_store = self.inner.processed_block_replay_store.clone();
        let processed_block_disk_cache_blocks = self.inner.config.processed_block_disk_cache_blocks;
        let processed_block_retry = ProcessedBlockProviderRetry {
            attempts: self
                .inner
                .config
                .live_processed_block_disk_cache_retry_attempts,
            delay_ms: self
                .inner
                .config
                .live_processed_block_disk_cache_retry_delay_ms,
        };
        tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("failed to build token tracking runtime");
            runtime.block_on(pipeline::run_range_index(
                task_run,
                task_provider,
                task_processed_block_replay_store,
                processed_block_disk_cache_blocks,
                processed_block_retry,
            ));
        });

        Ok(run)
    }

    pub async fn active_run(&self) -> Option<Arc<RangeIndexJob>> {
        let active_run_id = self.inner.active_run_id.read().await.clone()?;
        self.get_run(&active_run_id).await
    }

    pub async fn get_run(&self, id: &str) -> Option<Arc<RangeIndexJob>> {
        self.inner.runs.read().await.get(id).cloned()
    }

    pub async fn list_runs(&self) -> Vec<RunSummaryView> {
        let runs: Vec<_> = self.inner.runs.read().await.values().cloned().collect();
        let mut summaries = Vec::with_capacity(runs.len());
        for run in runs {
            summaries.push(RunSummaryView::from_run(&run).await);
        }
        summaries.sort_by(|left, right| left.id.cmp(&right.id));
        summaries
    }

    pub async fn stop_run(&self, id: &str) -> Option<Arc<RangeIndexJob>> {
        let run = self.get_run(id).await?;
        run.request_stop();
        run.mark_stopping().await;
        Some(run)
    }

    pub async fn stop_active_run(&self) -> Option<Arc<RangeIndexJob>> {
        let run = self.active_run().await?;
        run.request_stop();
        run.mark_stopping().await;
        Some(run)
    }

    fn resolve_request(
        &self,
        request: StartRangeIndexRequest,
    ) -> Result<ResolvedRangeIndexRequest> {
        let retention_mode = request.retention_mode;
        let history_limit = request
            .history_limit
            .unwrap_or(self.inner.config.history_limit);
        if history_limit == 0 {
            bail!("history_limit must be greater than zero");
        }

        let block_count = request
            .block_count
            .unwrap_or(self.inner.config.default_blocks);
        if block_count == 0 {
            bail!("block_count must be greater than zero");
        }

        let latest_block = if request.start_block.is_none() || request.end_block.is_none() {
            self.inner.provider.refresh_static_file_provider()?;
            Some(
                self.inner
                    .provider
                    .get_latest_block()?
                    .saturating_sub(DEFAULT_HISTORICAL_END_BLOCK_LAG),
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
                .checked_sub(block_count - 1)
                .ok_or_else(|| eyre::eyre!("block range underflow"))?,
        };

        if end_block < start_block {
            bail!("end_block must be greater than or equal to start_block");
        }

        let resolved = ResolvedRangeIndexRequest {
            start_block,
            end_block,
            history_limit,
            retention_mode,
        };

        Ok(resolved)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActiveRunStartDecision {
    Start,
    Reject,
    StopAndStart,
}

fn active_run_start_decision(
    status: Option<&RangeIndexStatus>,
    replace_active: bool,
) -> ActiveRunStartDecision {
    let Some(status) = status else {
        return ActiveRunStartDecision::Start;
    };
    if status.is_terminal() {
        return ActiveRunStartDecision::Start;
    }
    if replace_active {
        ActiveRunStartDecision::StopAndStart
    } else {
        ActiveRunStartDecision::Reject
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_becomes_active_start_candidate() {
        assert_eq!(
            active_run_start_decision(None, false),
            ActiveRunStartDecision::Start
        );
    }

    #[test]
    fn second_run_without_replace_conflicts_while_active_is_running() {
        assert_eq!(
            active_run_start_decision(Some(&RangeIndexStatus::Running), false),
            ActiveRunStartDecision::Reject
        );
    }

    #[test]
    fn second_run_with_replace_stops_running_active_run() {
        assert_eq!(
            active_run_start_decision(Some(&RangeIndexStatus::Running), true),
            ActiveRunStartDecision::StopAndStart
        );
    }

    #[test]
    fn terminal_active_run_allows_new_run_without_replace() {
        assert_eq!(
            active_run_start_decision(Some(&RangeIndexStatus::Completed), false),
            ActiveRunStartDecision::Start
        );
        assert_eq!(
            active_run_start_decision(Some(&RangeIndexStatus::Failed), false),
            ActiveRunStartDecision::Start
        );
        assert_eq!(
            active_run_start_decision(Some(&RangeIndexStatus::Stopped), false),
            ActiveRunStartDecision::Start
        );
    }
}
