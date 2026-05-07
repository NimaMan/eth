use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub recent_blocks: usize,
    pub chain_state_snapshots: usize,
    pub block_snapshot_ttl_secs: Option<u64>,
    pub token_snapshot_ttl_secs: Option<u64>,
    pub position_ttl_secs: Option<u64>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            recent_blocks: 128,
            chain_state_snapshots: 10,
            block_snapshot_ttl_secs: None,
            token_snapshot_ttl_secs: None,
            position_ttl_secs: None,
        }
    }
}
