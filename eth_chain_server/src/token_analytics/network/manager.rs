use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use reth_chain_query::RethQueryProvider;
use tokio::sync::RwLock;
use tx_processor::ProcessedBlockReplayStoreWriter;

use super::cache::{prune_finished_jobs, DEFAULT_MAX_ANALYSIS_JOBS};
use super::error::StartTokenNetworkAnalysisError;
use super::job::TokenNetworkAnalysisJob;
use super::pipeline::TokenNetworkAnalysisPipeline;
use super::request::TokenNetworkAnalysisRequest;

#[derive(Clone)]
pub struct TokenNetworkAnalysisManager {
    inner: Arc<TokenNetworkAnalysisManagerInner>,
}

struct TokenNetworkAnalysisManagerInner {
    provider: Arc<RethQueryProvider>,
    replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    jobs: RwLock<BTreeMap<String, Arc<TokenNetworkAnalysisJob>>>,
    next_id: AtomicU64,
}

impl TokenNetworkAnalysisManager {
    pub fn new(
        provider: Arc<RethQueryProvider>,
        replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        Self {
            inner: Arc::new(TokenNetworkAnalysisManagerInner {
                provider,
                replay_store,
                jobs: RwLock::new(BTreeMap::new()),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    pub async fn start_analysis(
        &self,
        request: TokenNetworkAnalysisRequest,
    ) -> Result<Arc<TokenNetworkAnalysisJob>, StartTokenNetworkAnalysisError> {
        let latest = self
            .inner
            .provider
            .get_latest_block()
            .map_err(StartTokenNetworkAnalysisError::InvalidRequest)?;
        let request = request
            .resolve(latest)
            .map_err(StartTokenNetworkAnalysisError::InvalidRequest)?;
        let id = format!(
            "token-network-analysis-{}",
            self.inner.next_id.fetch_add(1, Ordering::SeqCst)
        );
        let job = Arc::new(TokenNetworkAnalysisJob::new(id.clone(), request.clone()));

        let pipeline = TokenNetworkAnalysisPipeline::new(
            self.inner.provider.clone(),
            self.inner.replay_store.clone(),
        );
        let running_job = job.clone();
        let runtime_thread_name = format!("token-network-analysis-runtime-{id}");
        std::thread::Builder::new()
            .name(format!("token-network-analysis-{id}"))
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .thread_name(runtime_thread_name)
                    .enable_all()
                    .build()
                    .expect("failed to build token network analysis runtime");
                runtime.block_on(async move {
                    match pipeline.run(request, running_job.clone()).await {
                        Ok(result) => {
                            if running_job.stop_requested() {
                                running_job.cancel().await;
                            } else {
                                running_job.complete(result).await;
                            }
                        }
                        Err(error) if running_job.stop_requested() => {
                            tracing::info!(job_id = %running_job.id, error = %error, "token network analysis canceled");
                            running_job.cancel().await;
                        }
                        Err(error) => {
                            tracing::warn!(job_id = %running_job.id, error = %error, "token network analysis failed");
                            running_job.fail(error.to_string()).await;
                        }
                    }
                });
            })
            .map_err(StartTokenNetworkAnalysisError::Spawn)?;

        {
            let mut jobs = self.inner.jobs.write().await;
            jobs.insert(id, job.clone());
            prune_finished_jobs(&mut jobs, DEFAULT_MAX_ANALYSIS_JOBS).await;
        }

        Ok(job)
    }

    pub async fn get_job(&self, id: &str) -> Option<Arc<TokenNetworkAnalysisJob>> {
        self.inner.jobs.read().await.get(id).cloned()
    }

    pub async fn list_jobs(&self) -> Vec<Arc<TokenNetworkAnalysisJob>> {
        self.inner.jobs.read().await.values().cloned().collect()
    }

    pub async fn cancel_job(&self, id: &str) -> Option<Arc<TokenNetworkAnalysisJob>> {
        let job = self.get_job(id).await?;
        job.request_stop();
        job.cancel().await;
        Some(job)
    }
}
