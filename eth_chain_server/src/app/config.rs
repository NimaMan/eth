use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use eyre::{eyre, Result};

const ETH_CONFIG_PATH_ENV: &str = "ETH_CONFIG_PATH";
const ETH_NODE_ROOT_CONFIG: &str = "ETH_NODE_ROOT";
const RETH_DATADIR_CONFIG: &str = "RETH_DATADIR";
const RETH_INDEX_DIR_CONFIG: &str = "RETH_INDEX_DIR";
const RETH_HTTP_RPC_CONFIG: &str = "RETH_HTTP_RPC";
const RETH_WS_RPC_CONFIG: &str = "RETH_WS_RPC";
const CHAIN_SERVER_BIND_CONFIG: &str = "CHAIN_SERVER_BIND";
const CHAIN_SERVER_AUTO_START_LIVE_CONFIG: &str = "CHAIN_SERVER_AUTO_START_LIVE";
const CHAIN_SERVER_HISTORY_LIMIT_CONFIG: &str = "CHAIN_SERVER_HISTORY_LIMIT";
const CHAIN_SERVER_DEFAULT_BLOCKS_CONFIG: &str = "CHAIN_SERVER_DEFAULT_BLOCKS";
const LIVE_TOKEN_TRACKER_WARMUP_BLOCKS_CONFIG: &str = "LIVE_TOKEN_TRACKER_WARMUP_BLOCKS";
const PROCESSED_BLOCK_DISK_CACHE_DIR_CONFIG: &str = "PROCESSED_BLOCK_DISK_CACHE_DIR";
const PROCESSED_BLOCK_DISK_CACHE_BLOCKS_CONFIG: &str = "PROCESSED_BLOCK_DISK_CACHE_BLOCKS";
const PROCESSED_BLOCK_DISK_CACHE_DIR_NAME: &str = "processed-block-cache";
const LIVE_TOKEN_TRACKER_BLOCK_APPLY_TIMEOUT_MS_CONFIG: &str =
    "LIVE_TOKEN_TRACKER_BLOCK_APPLY_TIMEOUT_MS";
const MEMPOOL_DATABASE_URL_CONFIG: &str = "MEMPOOL_DATABASE_URL";
const ALPHA_DATABASE_URL_CONFIG: &str = "ALPHA_DATABASE_URL";
const MEMPOOL_SIGNAL_LIMIT_CONFIG: &str = "MEMPOOL_SIGNAL_LIMIT";
const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_ETH_NODE_ROOT: &str = "/home/nima/storage/samsung8tb/ethereum";
const DEFAULT_RETH_HTTP_RPC: &str = "http://127.0.0.1:8545";
const DEFAULT_RETH_WS_RPC: &str = "ws://127.0.0.1:8546";
const DEFAULT_BIND: &str = "127.0.0.1:8765";
const DEFAULT_AUTO_START_LIVE: bool = true;
const DEFAULT_HISTORY_LIMIT: usize = 1_000;
const DEFAULT_BLOCKS: u64 = 7_000;
const DEFAULT_LIVE_WARMUP_BLOCKS: u64 = 7_000;
const DEFAULT_PROCESSED_BLOCK_DISK_CACHE_BLOCKS: u64 = 1_000_000;
const DEFAULT_BLOCK_APPLY_TIMEOUT_MS: u64 = 3_000;
const DEFAULT_MEMPOOL_DATABASE_URL: &str = "postgresql://postgres:postgres@localhost:5432/eth_db";
const DEFAULT_MEMPOOL_SIGNAL_LIMIT: i64 = 200;

#[derive(Clone, Debug)]
pub struct ChainServerConfig {
    pub bind: SocketAddr,
    pub reth_datadir: PathBuf,
    pub reth_index_dir: Option<PathBuf>,
    pub execution_rpc: String,
    pub execution_ws: String,
    pub auto_start_live: bool,
    pub history_limit: usize,
    pub default_blocks: u64,
    pub live_warmup_blocks: u64,
    pub processed_block_disk_cache_dir: Option<PathBuf>,
    pub processed_block_disk_cache_blocks: u64,
    pub live_block_apply_timeout_ms: u64,
    pub mempool_database_url: String,
    pub alpha_database_url: String,
    pub mempool_signal_limit: i64,
}

impl ChainServerConfig {
    pub fn from_config_file() -> Result<Self> {
        let config = load_config_env()?;
        Self::from_config_values(&config)
    }

    fn from_config_values(config: &HashMap<String, String>) -> Result<Self> {
        let bind = config_string(config, CHAIN_SERVER_BIND_CONFIG, DEFAULT_BIND).parse()?;
        let reth_datadir = PathBuf::from(config_string(
            config,
            RETH_DATADIR_CONFIG,
            DEFAULT_RETH_DATADIR,
        ));
        let reth_index_dir = config_optional_path(config, RETH_INDEX_DIR_CONFIG)
            .or_else(|| default_reth_index_dir(&reth_datadir));
        let execution_rpc = config_string(config, RETH_HTTP_RPC_CONFIG, DEFAULT_RETH_HTTP_RPC);
        let execution_ws = config_string(config, RETH_WS_RPC_CONFIG, DEFAULT_RETH_WS_RPC);
        let auto_start_live = config_parse(
            config,
            CHAIN_SERVER_AUTO_START_LIVE_CONFIG,
            DEFAULT_AUTO_START_LIVE,
        )?;
        let history_limit = config_parse(
            config,
            CHAIN_SERVER_HISTORY_LIMIT_CONFIG,
            DEFAULT_HISTORY_LIMIT,
        )?;
        let default_blocks =
            config_parse(config, CHAIN_SERVER_DEFAULT_BLOCKS_CONFIG, DEFAULT_BLOCKS)?;
        let live_warmup_blocks = config_parse(
            config,
            LIVE_TOKEN_TRACKER_WARMUP_BLOCKS_CONFIG,
            DEFAULT_LIVE_WARMUP_BLOCKS,
        )?;
        let processed_block_disk_cache_dir =
            config_optional_path(config, PROCESSED_BLOCK_DISK_CACHE_DIR_CONFIG)
                .or_else(|| default_processed_block_disk_cache_dir(config));
        let processed_block_disk_cache_blocks = config_parse(
            config,
            PROCESSED_BLOCK_DISK_CACHE_BLOCKS_CONFIG,
            DEFAULT_PROCESSED_BLOCK_DISK_CACHE_BLOCKS,
        )?;
        let live_block_apply_timeout_ms = config_parse(
            config,
            LIVE_TOKEN_TRACKER_BLOCK_APPLY_TIMEOUT_MS_CONFIG,
            DEFAULT_BLOCK_APPLY_TIMEOUT_MS,
        )?;
        let mempool_database_url = config_string(
            config,
            MEMPOOL_DATABASE_URL_CONFIG,
            DEFAULT_MEMPOOL_DATABASE_URL,
        );
        let alpha_database_url = config_string(
            config,
            ALPHA_DATABASE_URL_CONFIG,
            mempool_database_url.as_str(),
        );
        let mempool_signal_limit = config_parse(
            config,
            MEMPOOL_SIGNAL_LIMIT_CONFIG,
            DEFAULT_MEMPOOL_SIGNAL_LIMIT,
        )?;

        if history_limit == 0 {
            return Err(eyre!(
                "{CHAIN_SERVER_HISTORY_LIMIT_CONFIG} must be greater than zero"
            ));
        }
        if default_blocks == 0 {
            return Err(eyre!(
                "{CHAIN_SERVER_DEFAULT_BLOCKS_CONFIG} must be greater than zero"
            ));
        }
        if live_warmup_blocks == 0 {
            return Err(eyre!(
                "{LIVE_TOKEN_TRACKER_WARMUP_BLOCKS_CONFIG} must be greater than zero"
            ));
        }
        if processed_block_disk_cache_dir.is_some() && processed_block_disk_cache_blocks == 0 {
            return Err(eyre!(
                "{PROCESSED_BLOCK_DISK_CACHE_BLOCKS_CONFIG} must be greater than zero"
            ));
        }
        if execution_rpc.trim().is_empty() {
            return Err(eyre!("{RETH_HTTP_RPC_CONFIG} must not be empty"));
        }
        if execution_ws.trim().is_empty() {
            return Err(eyre!("{RETH_WS_RPC_CONFIG} must not be empty"));
        }
        if live_block_apply_timeout_ms == 0 {
            return Err(eyre!(
                "{LIVE_TOKEN_TRACKER_BLOCK_APPLY_TIMEOUT_MS_CONFIG} must be greater than zero"
            ));
        }
        if mempool_database_url.trim().is_empty() {
            return Err(eyre!("{MEMPOOL_DATABASE_URL_CONFIG} must not be empty"));
        }
        if alpha_database_url.trim().is_empty() {
            return Err(eyre!("{ALPHA_DATABASE_URL_CONFIG} must not be empty"));
        }
        if mempool_signal_limit <= 0 {
            return Err(eyre!(
                "{MEMPOOL_SIGNAL_LIMIT_CONFIG} must be greater than zero"
            ));
        }

        Ok(Self {
            bind,
            reth_datadir,
            reth_index_dir,
            execution_rpc,
            execution_ws,
            auto_start_live,
            history_limit,
            default_blocks,
            live_warmup_blocks,
            processed_block_disk_cache_dir,
            processed_block_disk_cache_blocks,
            live_block_apply_timeout_ms,
            mempool_database_url,
            alpha_database_url,
            mempool_signal_limit,
        })
    }
}

pub fn shared_config_value(key: &str) -> Result<Option<String>> {
    if let Some(value) = env_config_value(key) {
        return Ok(Some(value));
    }
    let config = load_config_env()?;
    Ok(config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned))
}

fn default_processed_block_disk_cache_dir(config: &HashMap<String, String>) -> Option<PathBuf> {
    let root = config_string(config, ETH_NODE_ROOT_CONFIG, DEFAULT_ETH_NODE_ROOT);
    if root.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(root).join(PROCESSED_BLOCK_DISK_CACHE_DIR_NAME))
    }
}

fn default_reth_index_dir(reth_datadir: &std::path::Path) -> Option<PathBuf> {
    let path = reth_datadir.join("reth_index");
    path.exists().then_some(path)
}

fn eth_config_path() -> PathBuf {
    env::var_os(ETH_CONFIG_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("config.env")
        })
}

fn load_config_env() -> Result<HashMap<String, String>> {
    let path = eth_config_path();
    let contents = fs::read_to_string(&path).map_err(|err| {
        eyre!(
            "failed to read shared config file {}: {err}",
            path.display()
        )
    })?;
    Ok(parse_env_config(&contents))
}

fn parse_env_config(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        values.insert(key.to_string(), unquote(value.trim()).to_string());
    }

    values
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn config_string(config: &HashMap<String, String>, key: &str, default: &str) -> String {
    env_config_value(key).unwrap_or_else(|| {
        config
            .get(key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or(default)
            .to_string()
    })
}

fn config_value<'a>(config: &'a HashMap<String, String>, key: &str) -> Option<String> {
    env_config_value(key).or_else(|| {
        config
            .get(key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn env_config_value(key: &str) -> Option<String> {
    env::var_os(key)
        .map(|value| value.to_string_lossy().trim().to_string())
        .filter(|value| !value.is_empty())
}

fn config_optional_path(config: &HashMap<String, String>, key: &str) -> Option<PathBuf> {
    config_value(config, key).map(PathBuf::from)
}

fn config_parse<T>(config: &HashMap<String, String>, key: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(config_value(config, key)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(default))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_live_warmup_from_shared_config_key() {
        let values = parse_env_config(
            r#"
            CHAIN_SERVER_BIND=127.0.0.1:8765
            LIVE_TOKEN_TRACKER_WARMUP_BLOCKS=7000
            RETH_HTTP_RPC=http://127.0.0.1:8545
            RETH_WS_RPC=ws://127.0.0.1:8546
            MEMPOOL_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db
            "#,
        );

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert_eq!(config.live_warmup_blocks, 7_000);
        assert_eq!(config.execution_rpc, "http://127.0.0.1:8545");
        assert_eq!(config.execution_ws, "ws://127.0.0.1:8546");
    }

    #[test]
    fn ignores_non_canonical_warmup_key() {
        let values = parse_env_config("CHAIN_SERVER_LIVE_WARMUP_BLOCKS=2000");

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert_eq!(config.live_warmup_blocks, DEFAULT_LIVE_WARMUP_BLOCKS);
    }

    #[test]
    fn strips_quotes_from_config_values() {
        let values = parse_env_config(
            r#"
            CHAIN_SERVER_BIND="127.0.0.1:9999"
            LIVE_TOKEN_TRACKER_WARMUP_BLOCKS='123'
            "#,
        );

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert_eq!(config.bind.to_string(), "127.0.0.1:9999");
        assert_eq!(config.live_warmup_blocks, 123);
    }

    #[test]
    fn reads_auto_start_live_flag() {
        let values = parse_env_config("CHAIN_SERVER_AUTO_START_LIVE=false");

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert!(!config.auto_start_live);
    }
}
