use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use eth_token::token_analytics::TokenPoolCurrentObservation;
use eth_token::tracking::BlockTokenProcessor;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::progress::{now_unix_secs, RangeIndexProgress, RangeIndexStatus};

#[derive(Clone, Debug, Deserialize)]
pub struct StartRangeIndexRequest {
    #[serde(default)]
    pub start_block: Option<u64>,
    #[serde(default)]
    pub end_block: Option<u64>,
    #[serde(default)]
    pub block_count: Option<u64>,
    #[serde(default)]
    pub history_limit: Option<usize>,
    #[serde(default)]
    pub retention_mode: RangeIndexRetentionMode,
    #[serde(default)]
    pub replace_active: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedRangeIndexRequest {
    pub start_block: u64,
    pub end_block: u64,
    pub history_limit: usize,
    pub retention_mode: RangeIndexRetentionMode,
}

impl ResolvedRangeIndexRequest {
    pub fn block_count(&self) -> u64 {
        self.end_block - self.start_block + 1
    }

    pub fn block_token_processor(&self) -> BlockTokenProcessor {
        let mut processor = match self.retention_mode {
            RangeIndexRetentionMode::KeepAll => {
                BlockTokenProcessor::new_unbounded_token_index(self.history_limit)
            }
            RangeIndexRetentionMode::BoundedIndex => BlockTokenProcessor::new(self.history_limit),
            RangeIndexRetentionMode::EphemeralTerminalScam => {
                BlockTokenProcessor::new_with_ephemeral_terminal_scam_retention(self.history_limit)
            }
        };
        processor.disable_network_graphs();
        processor
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeIndexRetentionMode {
    #[default]
    KeepAll,
    BoundedIndex,
    EphemeralTerminalScam,
}

impl RangeIndexRetentionMode {
    pub fn applies_after_observations(self) -> bool {
        matches!(self, Self::EphemeralTerminalScam)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeIndexErrorKind {
    Transaction,
    PoolSimulation,
    Run,
}

#[derive(Clone, Debug, Serialize)]
pub struct RangeIndexError {
    pub kind: RangeIndexErrorKind,
    pub block_number: Option<u64>,
    pub tx_index: Option<u64>,
    pub tx_hash: Option<String>,
    pub message: String,
}

#[derive(Debug)]
pub struct RangeIndexState {
    pub processor: BlockTokenProcessor,
    pub progress: RangeIndexProgress,
    pub errors: Vec<RangeIndexError>,
    pub created_tokens: BTreeSet<String>,
    pub updated_tokens: BTreeSet<String>,
    pub discovered_v2_pools: BTreeSet<String>,
    pub updated_v2_pools: BTreeSet<String>,
    pub discovered_v3_pools: BTreeSet<String>,
    pub updated_v3_pools: BTreeSet<String>,
    pub discovered_v4_pools: BTreeSet<String>,
    pub updated_v4_pools: BTreeSet<String>,
    pub observations: Vec<TokenPoolCurrentObservation>,
    pub active_observation_counts_by_pool: BTreeMap<String, u64>,
}

#[derive(Debug)]
pub struct RangeIndexJob {
    pub id: String,
    pub request: ResolvedRangeIndexRequest,
    pub state: RwLock<RangeIndexState>,
    stop_requested: AtomicBool,
}

impl RangeIndexJob {
    pub fn new(id: impl Into<String>, request: ResolvedRangeIndexRequest) -> Self {
        let id = id.into();
        let now = now_unix_secs();
        let processor = request.block_token_processor();
        let progress =
            RangeIndexProgress::new(id.clone(), request.start_block, request.end_block, now);

        Self {
            id,
            request,
            state: RwLock::new(RangeIndexState {
                processor,
                progress,
                errors: Vec::new(),
                created_tokens: BTreeSet::new(),
                updated_tokens: BTreeSet::new(),
                discovered_v2_pools: BTreeSet::new(),
                updated_v2_pools: BTreeSet::new(),
                discovered_v3_pools: BTreeSet::new(),
                updated_v3_pools: BTreeSet::new(),
                discovered_v4_pools: BTreeSet::new(),
                updated_v4_pools: BTreeSet::new(),
                observations: Vec::new(),
                active_observation_counts_by_pool: BTreeMap::new(),
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

    pub async fn progress(&self) -> RangeIndexProgress {
        self.state.read().await.progress.clone()
    }

    pub async fn mark_stopping(&self) {
        let mut state = self.state.write().await;
        if !state.progress.status.is_terminal() {
            state.progress.status = RangeIndexStatus::Stopping;
            state.progress.updated_at_unix_secs = now_unix_secs();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_range_request_defaults_to_keep_all_retention() {
        let request: StartRangeIndexRequest = serde_json::from_str("{}").unwrap();

        assert_eq!(request.retention_mode, RangeIndexRetentionMode::KeepAll);
    }

    #[test]
    fn start_range_request_defaults_to_not_replacing_active_run() {
        let request: StartRangeIndexRequest = serde_json::from_str("{}").unwrap();

        assert!(!request.replace_active);
    }

    #[test]
    fn start_range_request_accepts_replace_active() {
        let request: StartRangeIndexRequest =
            serde_json::from_str(r#"{"replace_active":true}"#).unwrap();

        assert!(request.replace_active);
    }

    #[test]
    fn keep_all_retention_uses_unbounded_token_index() {
        let request = ResolvedRangeIndexRequest {
            start_block: 100,
            end_block: 101,
            history_limit: 10,
            retention_mode: RangeIndexRetentionMode::KeepAll,
        };

        let processor = request.block_token_processor();

        assert_eq!(processor.token_index.max_size, None);
    }

    #[test]
    fn bounded_index_retention_uses_capped_token_index() {
        let request = ResolvedRangeIndexRequest {
            start_block: 100,
            end_block: 101,
            history_limit: 10,
            retention_mode: RangeIndexRetentionMode::BoundedIndex,
        };

        let processor = request.block_token_processor();

        assert!(processor.token_index.max_size.is_some());
    }

    #[test]
    fn ephemeral_terminal_scam_retention_uses_unbounded_index_with_policy() {
        let request = ResolvedRangeIndexRequest {
            start_block: 100,
            end_block: 101,
            history_limit: 10,
            retention_mode: RangeIndexRetentionMode::EphemeralTerminalScam,
        };

        let processor = request.block_token_processor();

        assert_eq!(processor.token_index.max_size, None);
        assert!(processor.token_index.live_retention_policy().is_some());
        assert!(request.retention_mode.applies_after_observations());
    }

    #[test]
    fn range_processors_disable_network_graphs() {
        let request = ResolvedRangeIndexRequest {
            start_block: 100,
            end_block: 101,
            history_limit: 10,
            retention_mode: RangeIndexRetentionMode::KeepAll,
        };

        let processor = request.block_token_processor();

        assert!(!processor.network_graphs_enabled);
    }
}
