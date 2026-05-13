use eth_live_state::keys;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenRuntimeConfig {
    pub history_limit: usize,
    pub default_warmup_blocks: u64,
    pub redis_url: String,
    pub live_block_stream: String,
    pub block_apply_timeout_ms: u64,
}

impl Default for LiveTokenRuntimeConfig {
    fn default() -> Self {
        Self {
            history_limit: 1_000,
            default_warmup_blocks: 7_000,
            redis_url: "redis://127.0.0.1:6379/0".to_string(),
            live_block_stream: keys::processed_block_stream_key().to_string(),
            block_apply_timeout_ms: 3_000,
        }
    }
}
