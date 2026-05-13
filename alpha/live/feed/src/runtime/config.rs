use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenRuntimeConfig {
    pub history_limit: usize,
    pub default_warmup_blocks: u64,
    pub block_apply_timeout_ms: u64,
}

impl Default for LiveTokenRuntimeConfig {
    fn default() -> Self {
        Self {
            history_limit: 1_000,
            default_warmup_blocks: 7_000,
            block_apply_timeout_ms: 3_000,
        }
    }
}
