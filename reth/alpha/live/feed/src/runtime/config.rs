use eth_live_state::keys;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenRuntimeConfig {
    pub history_limit: usize,
    pub default_warmup_blocks: u64,
    pub redis_url: String,
    pub live_block_stream: String,
    pub processed_block_disk_cache_retry_attempts: usize,
    pub processed_block_disk_cache_retry_delay_ms: u64,
    pub stream_block_ms: usize,
    pub stream_count: usize,
    pub block_apply_timeout_ms: u64,
}

impl Default for LiveTokenRuntimeConfig {
    fn default() -> Self {
        Self {
            history_limit: 1_000,
            default_warmup_blocks: 7_000,
            redis_url: "redis://127.0.0.1:6379/0".to_string(),
            live_block_stream: keys::processed_block_stream_key().to_string(),
            processed_block_disk_cache_retry_attempts: 20,
            processed_block_disk_cache_retry_delay_ms: 100,
            stream_block_ms: 5_000,
            stream_count: 100,
            block_apply_timeout_ms: 180_000,
        }
    }
}
