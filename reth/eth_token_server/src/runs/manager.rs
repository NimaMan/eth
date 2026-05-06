use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use tokio::sync::RwLock;

use crate::config::TokenServerConfig;
use crate::processed_block_cache::TokenProcessedBlockCacheStore;
use crate::views::run::RunSummaryView;

use super::range_runner;
use super::{ResolvedRunRequest, StartRunRequest, TrackingRun};

#[derive(Clone)]
pub struct RunManager {
    inner: Arc<RunManagerInner>,
}

struct RunManagerInner {
    config: TokenServerConfig,
    provider: Arc<RethQueryProvider>,
    processed_block_cache: Option<Arc<TokenProcessedBlockCacheStore>>,
    runs: RwLock<HashMap<String, Arc<TrackingRun>>>,
    next_id: AtomicU64,
}

impl RunManager {
    pub fn new(
        config: TokenServerConfig,
        provider: Arc<RethQueryProvider>,
        processed_block_cache: Option<Arc<TokenProcessedBlockCacheStore>>,
    ) -> Self {
        Self {
            inner: Arc::new(RunManagerInner {
                config,
                provider,
                processed_block_cache,
                runs: RwLock::new(HashMap::new()),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    pub async fn start_run(&self, request: StartRunRequest) -> Result<Arc<TrackingRun>> {
        let request = self.resolve_request(request)?;
        let sequence = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        let id = format!("run-{sequence}");
        let run = Arc::new(TrackingRun::new(id.clone(), request));

        self.inner.runs.write().await.insert(id, run.clone());

        let task_run = run.clone();
        let task_provider = self.inner.provider.clone();
        let task_processed_block_cache = self.inner.processed_block_cache.clone();
        let processed_block_cache_blocks = self.inner.config.processed_block_cache_blocks;
        tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("failed to build token tracking runtime");
            runtime.block_on(range_runner::run_range(
                task_run,
                task_provider,
                task_processed_block_cache,
                processed_block_cache_blocks,
            ));
        });

        Ok(run)
    }

    pub async fn get_run(&self, id: &str) -> Option<Arc<TrackingRun>> {
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

    pub async fn stop_run(&self, id: &str) -> Option<Arc<TrackingRun>> {
        let run = self.get_run(id).await?;
        run.request_stop();
        run.mark_stopping().await;
        Some(run)
    }

    fn resolve_request(&self, request: StartRunRequest) -> Result<ResolvedRunRequest> {
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
            Some(self.inner.provider.get_latest_block()?)
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

        let resolved = ResolvedRunRequest {
            start_block,
            end_block,
            history_limit,
        };

        if resolved.block_count() > self.inner.config.max_blocks {
            bail!(
                "range has {} blocks, max allowed is {}",
                resolved.block_count(),
                self.inner.config.max_blocks
            );
        }

        Ok(resolved)
    }
}
