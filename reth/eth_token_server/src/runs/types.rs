use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use eth_token::manager::BlockTokenProcessor;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::progress::{now_unix_secs, RunProgress, RunStatus};

#[derive(Clone, Debug, Deserialize)]
pub struct StartRunRequest {
    #[serde(default)]
    pub start_block: Option<u64>,
    #[serde(default)]
    pub end_block: Option<u64>,
    #[serde(default)]
    pub block_count: Option<u64>,
    #[serde(default)]
    pub history_limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedRunRequest {
    pub start_block: u64,
    pub end_block: u64,
    pub history_limit: usize,
}

impl ResolvedRunRequest {
    pub fn block_count(&self) -> u64 {
        self.end_block - self.start_block + 1
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RunError {
    pub block_number: Option<u64>,
    pub tx_index: Option<u64>,
    pub tx_hash: Option<String>,
    pub message: String,
}

#[derive(Debug)]
pub struct TrackingRunState {
    pub processor: BlockTokenProcessor,
    pub progress: RunProgress,
    pub errors: Vec<RunError>,
    pub created_tokens: BTreeSet<String>,
    pub updated_tokens: BTreeSet<String>,
    pub discovered_v2_pools: BTreeSet<String>,
    pub updated_v2_pools: BTreeSet<String>,
}

#[derive(Debug)]
pub struct TrackingRun {
    pub id: String,
    pub request: ResolvedRunRequest,
    pub state: RwLock<TrackingRunState>,
    stop_requested: AtomicBool,
}

impl TrackingRun {
    pub fn new(id: impl Into<String>, request: ResolvedRunRequest) -> Self {
        let id = id.into();
        let now = now_unix_secs();
        let processor = BlockTokenProcessor::new(request.history_limit);
        let progress = RunProgress::new(id.clone(), request.start_block, request.end_block, now);

        Self {
            id,
            request,
            state: RwLock::new(TrackingRunState {
                processor,
                progress,
                errors: Vec::new(),
                created_tokens: BTreeSet::new(),
                updated_tokens: BTreeSet::new(),
                discovered_v2_pools: BTreeSet::new(),
                updated_v2_pools: BTreeSet::new(),
            }),
            stop_requested: AtomicBool::new(false),
        }
    }

    pub fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
    }

    pub fn stop_requested(&self) -> bool {
        self.stop_requested.load(Ordering::SeqCst)
    }

    pub async fn progress(&self) -> RunProgress {
        self.state.read().await.progress.clone()
    }

    pub async fn mark_stopping(&self) {
        let mut state = self.state.write().await;
        if !state.progress.status.is_terminal() {
            state.progress.status = RunStatus::Stopping;
            state.progress.updated_at_unix_secs = now_unix_secs();
        }
    }
}
