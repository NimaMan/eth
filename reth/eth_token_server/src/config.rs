use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use eyre::{eyre, Result};

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_ETH_NODE_ROOT: &str = "/home/nima/storage/samsung8tb/ethereum";
const DEFAULT_BIND: &str = "127.0.0.1:8765";
const DEFAULT_HISTORY_LIMIT: usize = 1_000;
const DEFAULT_BLOCKS: u64 = 7_000;
const DEFAULT_PROCESSED_BLOCK_CACHE_BLOCKS: u64 = 100_000;
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/0";
const DEFAULT_LIVE_BLOCK_STREAM: &str = "eth/live/blocks";
const DEFAULT_CACHE_RETRY_ATTEMPTS: usize = 20;
const DEFAULT_CACHE_RETRY_DELAY_MS: u64 = 100;
const DEFAULT_STREAM_BLOCK_MS: usize = 5_000;
const DEFAULT_STREAM_COUNT: usize = 100;

#[derive(Clone, Debug)]
pub struct TokenServerConfig {
    pub bind: SocketAddr,
    pub reth_datadir: PathBuf,
    pub history_limit: usize,
    pub default_blocks: u64,
    pub processed_block_cache_dir: Option<PathBuf>,
    pub processed_block_cache_blocks: u64,
    pub redis_url: String,
    pub live_block_stream: String,
    pub live_cache_retry_attempts: usize,
    pub live_cache_retry_delay_ms: u64,
    pub live_stream_block_ms: usize,
    pub live_stream_count: usize,
}

impl TokenServerConfig {
    pub fn from_env() -> Result<Self> {
        let bind = env_string("ETH_TOKEN_SERVER_BIND", DEFAULT_BIND).parse()?;
        let reth_datadir = PathBuf::from(env_string("RETH_DATADIR", DEFAULT_RETH_DATADIR));
        let history_limit = env_parse("ETH_TOKEN_SERVER_HISTORY_LIMIT", DEFAULT_HISTORY_LIMIT)?;
        let default_blocks = env_parse("ETH_TOKEN_SERVER_DEFAULT_BLOCKS", DEFAULT_BLOCKS)?;
        let processed_block_cache_dir =
            env_optional_path("ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_DIR")
                .or_else(default_processed_block_cache_dir);
        let processed_block_cache_blocks = env_parse(
            "ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_BLOCKS",
            DEFAULT_PROCESSED_BLOCK_CACHE_BLOCKS,
        )?;
        let redis_url = env_string("ETH_TOKEN_SERVER_REDIS_URL", DEFAULT_REDIS_URL);
        let live_block_stream = env_string(
            "ETH_TOKEN_SERVER_LIVE_BLOCK_STREAM",
            DEFAULT_LIVE_BLOCK_STREAM,
        );
        let live_cache_retry_attempts = env_parse(
            "ETH_TOKEN_SERVER_LIVE_CACHE_RETRY_ATTEMPTS",
            DEFAULT_CACHE_RETRY_ATTEMPTS,
        )?;
        let live_cache_retry_delay_ms = env_parse(
            "ETH_TOKEN_SERVER_LIVE_CACHE_RETRY_DELAY_MS",
            DEFAULT_CACHE_RETRY_DELAY_MS,
        )?;
        let live_stream_block_ms = env_parse(
            "ETH_TOKEN_SERVER_LIVE_STREAM_BLOCK_MS",
            DEFAULT_STREAM_BLOCK_MS,
        )?;
        let live_stream_count =
            env_parse("ETH_TOKEN_SERVER_LIVE_STREAM_COUNT", DEFAULT_STREAM_COUNT)?;

        if history_limit == 0 {
            return Err(eyre!(
                "ETH_TOKEN_SERVER_HISTORY_LIMIT must be greater than zero"
            ));
        }
        if default_blocks == 0 {
            return Err(eyre!(
                "ETH_TOKEN_SERVER_DEFAULT_BLOCKS must be greater than zero"
            ));
        }
        if processed_block_cache_dir.is_some() && processed_block_cache_blocks == 0 {
            return Err(eyre!(
                "ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_BLOCKS must be greater than zero"
            ));
        }
        if redis_url.trim().is_empty() {
            return Err(eyre!("ETH_TOKEN_SERVER_REDIS_URL must not be empty"));
        }
        if live_block_stream.trim().is_empty() {
            return Err(eyre!(
                "ETH_TOKEN_SERVER_LIVE_BLOCK_STREAM must not be empty"
            ));
        }
        if live_stream_block_ms == 0 {
            return Err(eyre!(
                "ETH_TOKEN_SERVER_LIVE_STREAM_BLOCK_MS must be greater than zero"
            ));
        }
        if live_stream_count == 0 {
            return Err(eyre!(
                "ETH_TOKEN_SERVER_LIVE_STREAM_COUNT must be greater than zero"
            ));
        }

        Ok(Self {
            bind,
            reth_datadir,
            history_limit,
            default_blocks,
            processed_block_cache_dir,
            processed_block_cache_blocks,
            redis_url,
            live_block_stream,
            live_cache_retry_attempts,
            live_cache_retry_delay_ms,
            live_stream_block_ms,
            live_stream_count,
        })
    }
}

fn env_string(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_optional_path(key: &str) -> Option<PathBuf> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn default_processed_block_cache_dir() -> Option<PathBuf> {
    let root = env_string("ETH_NODE_ROOT", DEFAULT_ETH_NODE_ROOT);
    if root.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(root).join("processed_block_cache"))
    }
}

fn env_parse<T>(key: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match env::var(key) {
        Ok(value) => Ok(value.parse()?),
        Err(_) => Ok(default),
    }
}
