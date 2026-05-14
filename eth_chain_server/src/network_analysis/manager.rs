use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use reth_chain_query::RethQueryProvider;
use tokio::sync::RwLock;
use tx_processor::ProcessedBlockReplayStoreWriter;

use super::cache::{prune_finished_jobs, DEFAULT_MAX_ANALYSIS_JOBS};
use super::error::StartNetworkAnalysisError;
use super::job::NetworkAnalysisJob;
use super::pipeline::NetworkAnalysisPipeline;
use super::request::NetworkAnalysisRequest;

#[derive(Clone)]
pub struct NetworkAnalysisManager {
    inner: Arc<NetworkAnalysisManagerInner>,
}

struct NetworkAnalysisManagerInner {
    provider: Arc<RethQueryProvider>,
    replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    jobs: RwLock<BTreeMap<String, Arc<NetworkAnalysisJob>>>,
    next_id: AtomicU64,
}

impl NetworkAnalysisManager {
    pub fn new(
        provider: Arc<RethQueryProvider>,
        replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        Self {
            inner: Arc::new(NetworkAnalysisManagerInner {
                provider,
                replay_store,
                jobs: RwLock::new(BTreeMap::new()),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    pub async fn start_analysis(
        &self,
        request: NetworkAnalysisRequest,
    ) -> Result<Arc<NetworkAnalysisJob>, StartNetworkAnalysisError> {
        let latest = self
            .inner
            .provider
            .get_latest_block()
            .map_err(StartNetworkAnalysisError::InvalidRequest)?;
        let request = request
            .resolve(latest)
            .map_err(StartNetworkAnalysisError::InvalidRequest)?;
        let id = format!(
            "network-analysis-{}",
            self.inner.next_id.fetch_add(1, Ordering::SeqCst)
        );
        let job = Arc::new(NetworkAnalysisJob::new(id.clone(), request.clone()));

        let pipeline = NetworkAnalysisPipeline::new(
            self.inner.provider.clone(),
            self.inner.replay_store.clone(),
        );
        let running_job = job.clone();
        std::thread::Builder::new()
            .name(format!("network-analysis-{id}"))
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build network analysis runtime");
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
                            tracing::info!(job_id = %running_job.id, error = %error, "network analysis canceled");
                            running_job.cancel().await;
                        }
                        Err(error) => {
                            tracing::warn!(job_id = %running_job.id, error = %error, "network analysis failed");
                            running_job.fail(error.to_string()).await;
                        }
                    }
                });
            })
            .map_err(StartNetworkAnalysisError::Spawn)?;

        {
            let mut jobs = self.inner.jobs.write().await;
            jobs.insert(id, job.clone());
            prune_finished_jobs(&mut jobs, DEFAULT_MAX_ANALYSIS_JOBS).await;
        }

        Ok(job)
    }

    pub async fn get_job(&self, id: &str) -> Option<Arc<NetworkAnalysisJob>> {
        self.inner.jobs.read().await.get(id).cloned()
    }

    pub async fn list_jobs(&self) -> Vec<Arc<NetworkAnalysisJob>> {
        self.inner.jobs.read().await.values().cloned().collect()
    }

    pub async fn cancel_job(&self, id: &str) -> Option<Arc<NetworkAnalysisJob>> {
        let job = self.get_job(id).await?;
        job.request_stop();
        job.cancel().await;
        Some(job)
    }
}
