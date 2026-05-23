use std::collections::{BTreeMap, BTreeSet};

use eth_ops_events::{PipelineBottleneckSample, PipelineIssue};
use eth_token::live::LiveBlockTokenProcessor;
use eth_token::token_analytics::TokenPoolCurrentObservation;
use eth_token::tracking::LiveTokenRetentionReport;

use super::progress::{LiveTokenError, LiveTokenProgress};
use super::time::now_unix_secs;

#[derive(Debug)]
pub struct LiveTokenState {
    pub processor: LiveBlockTokenProcessor,
    pub progress: LiveTokenProgress,
    pub errors: Vec<LiveTokenError>,
    pub issues: Vec<PipelineIssue>,
    pub bottlenecks: Vec<PipelineBottleneckSample>,
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
    pub last_retention_report: Option<LiveTokenRetentionReport>,
}

impl LiveTokenState {
    pub fn idle(history_limit: usize) -> Self {
        Self {
            processor: LiveBlockTokenProcessor::new(history_limit),
            progress: LiveTokenProgress::idle(history_limit),
            errors: Vec::new(),
            issues: Vec::new(),
            bottlenecks: Vec::new(),
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
            last_retention_report: None,
        }
    }

    pub fn warming(id: String, history_limit: usize, start_block: u64, end_block: u64) -> Self {
        let now = now_unix_secs();
        Self {
            processor: LiveBlockTokenProcessor::new(history_limit),
            progress: LiveTokenProgress::warming(id, history_limit, start_block, end_block, now),
            errors: Vec::new(),
            issues: Vec::new(),
            bottlenecks: Vec::new(),
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
            last_retention_report: None,
        }
    }
}
