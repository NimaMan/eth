use std::sync::Arc;

use serde::Serialize;

use crate::token_analytics::network::job::{
    TokenNetworkAnalysisJob, TokenNetworkAnalysisProgress, TokenNetworkAnalysisState,
};
use crate::token_analytics::network::pipeline::summary::TokenNetworkAnalysisResult;
use crate::token_analytics::network::request::ResolvedTokenNetworkAnalysisRequest;

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkAnalysisJobResponse {
    pub id: String,
    pub request: ResolvedTokenNetworkAnalysisRequest,
    pub progress: TokenNetworkAnalysisProgress,
    pub result: Option<TokenNetworkAnalysisResult>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkAnalysisListResponse {
    pub count: usize,
    pub jobs: Vec<TokenNetworkAnalysisJobResponse>,
}

pub async fn job(job: &Arc<TokenNetworkAnalysisJob>) -> TokenNetworkAnalysisJobResponse {
    let TokenNetworkAnalysisState {
        progress,
        result,
        error,
    } = job.state().await;
    TokenNetworkAnalysisJobResponse {
        id: job.id.clone(),
        request: job.request.clone(),
        progress,
        result,
        error,
    }
}

pub async fn list(jobs: Vec<Arc<TokenNetworkAnalysisJob>>) -> TokenNetworkAnalysisListResponse {
    let mut responses = Vec::with_capacity(jobs.len());
    for job_ref in jobs {
        responses.push(job(&job_ref).await);
    }
    responses.sort_by(|left, right| right.progress.started_at.cmp(&left.progress.started_at));
    TokenNetworkAnalysisListResponse {
        count: responses.len(),
        jobs: responses,
    }
}
