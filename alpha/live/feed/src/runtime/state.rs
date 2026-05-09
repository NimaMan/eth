use std::collections::BTreeSet;

use eth_token::live::LiveBlockTokenProcessor;
use eth_token::manager::LiveTokenRetentionReport;

use super::progress::{LiveTokenError, LiveTokenProgress};
use super::time::now_unix_secs;

#[derive(Debug)]
pub struct LiveTokenState {
    pub processor: LiveBlockTokenProcessor,
    pub progress: LiveTokenProgress,
    pub errors: Vec<LiveTokenError>,
    pub created_tokens: BTreeSet<String>,
    pub updated_tokens: BTreeSet<String>,
    pub discovered_v2_pools: BTreeSet<String>,
    pub updated_v2_pools: BTreeSet<String>,
    pub last_retention_report: Option<LiveTokenRetentionReport>,
}

impl LiveTokenState {
    pub fn idle(history_limit: usize) -> Self {
        Self {
            processor: LiveBlockTokenProcessor::new(history_limit),
            progress: LiveTokenProgress::idle(history_limit),
            errors: Vec::new(),
            created_tokens: BTreeSet::new(),
            updated_tokens: BTreeSet::new(),
            discovered_v2_pools: BTreeSet::new(),
            updated_v2_pools: BTreeSet::new(),
            last_retention_report: None,
        }
    }

    pub fn warming(id: String, history_limit: usize, start_block: u64, end_block: u64) -> Self {
        let now = now_unix_secs();
        Self {
            processor: LiveBlockTokenProcessor::new(history_limit),
            progress: LiveTokenProgress::warming(id, history_limit, start_block, end_block, now),
            errors: Vec::new(),
            created_tokens: BTreeSet::new(),
            updated_tokens: BTreeSet::new(),
            discovered_v2_pools: BTreeSet::new(),
            updated_v2_pools: BTreeSet::new(),
            last_retention_report: None,
        }
    }
}
