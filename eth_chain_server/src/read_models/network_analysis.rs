use std::sync::Arc;

use serde::Serialize;

use crate::network_analysis::job::{
    NetworkAnalysisJob, NetworkAnalysisProgress, NetworkAnalysisState,
};
use crate::network_analysis::pipeline::summary::NetworkAnalysisResult;
use crate::network_analysis::request::ResolvedNetworkAnalysisRequest;

#[derive(Clone, Debug, Serialize)]
pub struct NetworkAnalysisJobResponse {
    pub id: String,
    pub request: ResolvedNetworkAnalysisRequest,
    pub progress: NetworkAnalysisProgress,
    pub result: Option<NetworkAnalysisResult>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkAnalysisListResponse {
    pub count: usize,
    pub jobs: Vec<NetworkAnalysisJobResponse>,
}

pub async fn job(job: &Arc<NetworkAnalysisJob>) -> NetworkAnalysisJobResponse {
    let NetworkAnalysisState {
        progress,
        result,
        error,
    } = job.state().await;
    NetworkAnalysisJobResponse {
        id: job.id.clone(),
        request: job.request.clone(),
        progress,
        result,
        error,
    }
}

pub async fn list(jobs: Vec<Arc<NetworkAnalysisJob>>) -> NetworkAnalysisListResponse {
    let mut responses = Vec::with_capacity(jobs.len());
    for job_ref in jobs {
        responses.push(job(&job_ref).await);
    }
    responses.sort_by(|left, right| right.progress.started_at.cmp(&left.progress.started_at));
    NetworkAnalysisListResponse {
        count: responses.len(),
        jobs: responses,
    }
}
