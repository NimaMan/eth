use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use serde::Serialize;
use tokio::sync::RwLock;

use super::pipeline::summary::NetworkAnalysisResult;
use super::request::ResolvedNetworkAnalysisRequest;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAnalysisStatus {
    Queued,
    Running,
    Complete,
    Failed,
    Canceled,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkAnalysisProgress {
    pub status: NetworkAnalysisStatus,
    pub stage: String,
    pub token_blocks_selected: usize,
    pub context_blocks_selected: usize,
    pub blocks_to_load: usize,
    pub blocks_loaded: usize,
    pub started_at: i64,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkAnalysisState {
    pub progress: NetworkAnalysisProgress,
    pub result: Option<NetworkAnalysisResult>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct NetworkAnalysisJob {
    pub id: String,
    pub request: ResolvedNetworkAnalysisRequest,
    state: RwLock<NetworkAnalysisState>,
    stop_requested: AtomicBool,
}

impl NetworkAnalysisJob {
    pub fn new(id: impl Into<String>, request: ResolvedNetworkAnalysisRequest) -> Self {
        let now = now_unix_secs();
        Self {
            id: id.into(),
            request,
            state: RwLock::new(NetworkAnalysisState {
                progress: NetworkAnalysisProgress {
                    status: NetworkAnalysisStatus::Queued,
                    stage: "queued".to_string(),
                    token_blocks_selected: 0,
                    context_blocks_selected: 0,
                    blocks_to_load: 0,
                    blocks_loaded: 0,
                    started_at: now,
                    updated_at: now,
                    finished_at: None,
                },
                result: None,
                error: None,
            }),
            stop_requested: AtomicBool::new(false),
        }
    }

    pub async fn state(&self) -> NetworkAnalysisState {
        self.state.read().await.clone()
    }

    pub async fn mark_running(&self, stage: impl Into<String>) {
        self.update_progress(|progress| {
            progress.status = NetworkAnalysisStatus::Running;
            progress.stage = stage.into();
        })
        .await;
    }

    pub async fn update_progress(&self, update: impl FnOnce(&mut NetworkAnalysisProgress)) {
        let mut state = self.state.write().await;
        update(&mut state.progress);
        state.progress.updated_at = now_unix_secs();
    }

    pub async fn complete(&self, result: NetworkAnalysisResult) {
        let mut state = self.state.write().await;
        let now = now_unix_secs();
        state.progress.status = NetworkAnalysisStatus::Complete;
        state.progress.stage = "complete".to_string();
        state.progress.updated_at = now;
        state.progress.finished_at = Some(now);
        state.result = Some(result);
        state.error = None;
    }

    pub async fn fail(&self, error: impl Into<String>) {
        let mut state = self.state.write().await;
        let now = now_unix_secs();
        state.progress.status = NetworkAnalysisStatus::Failed;
        state.progress.stage = "failed".to_string();
        state.progress.updated_at = now;
        state.progress.finished_at = Some(now);
        state.error = Some(error.into());
    }

    pub async fn cancel(&self) {
        let mut state = self.state.write().await;
        let now = now_unix_secs();
        state.progress.status = NetworkAnalysisStatus::Canceled;
        state.progress.stage = "canceled".to_string();
        state.progress.updated_at = now;
        state.progress.finished_at = Some(now);
    }

    pub fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
    }

    pub fn stop_requested(&self) -> bool {
        self.stop_requested.load(Ordering::SeqCst)
    }
}

fn now_unix_secs() -> i64 {
    Utc::now().timestamp()
}
