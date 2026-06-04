use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use eyre::{eyre, Result};

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
const MEMPOOL_DATABASE_CONFIG_KEY: &str = "databases.mempool.url";
const ALPHA_DATABASE_CONFIG_KEY: &str = "databases.alpha.url";
const RISK_ATLAS_DATABASE_CONFIG_KEY: &str = "databases.risk_atlas.url";
const TOKEN_PNL_DATABASE_CONFIG_KEY: &str = "databases.token_pnl.url";
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
const DEFAULT_MEMPOOL_SIGNAL_LIMIT: i64 = 200;

static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

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
    pub risk_atlas_database_url: String,
    pub token_pnl_database_url: String,
    pub mempool_signal_limit: i64,
}

impl ChainServerConfig {
    pub fn from_config_file() -> Result<Self> {
        let config = load_config_env()?;
        Self::from_config_values(&config)
    }

    pub fn from_config_path(path: impl AsRef<Path>) -> Result<Self> {
        let config = load_config_env_from_path(path.as_ref())?;
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
        let mempool_database_url = required_config_string(config, MEMPOOL_DATABASE_CONFIG_KEY)?;
        let alpha_database_url = required_config_string(config, ALPHA_DATABASE_CONFIG_KEY)?;
        let risk_atlas_database_url =
            required_config_string(config, RISK_ATLAS_DATABASE_CONFIG_KEY)?;
        let token_pnl_database_url = required_config_string(config, TOKEN_PNL_DATABASE_CONFIG_KEY)?;
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
            return Err(eyre!("{MEMPOOL_DATABASE_CONFIG_KEY} must not be empty"));
        }
        if alpha_database_url.trim().is_empty() {
            return Err(eyre!("{ALPHA_DATABASE_CONFIG_KEY} must not be empty"));
        }
        if risk_atlas_database_url.trim().is_empty() {
            return Err(eyre!("{RISK_ATLAS_DATABASE_CONFIG_KEY} must not be empty"));
        }
        if token_pnl_database_url.trim().is_empty() {
            return Err(eyre!("{TOKEN_PNL_DATABASE_CONFIG_KEY} must not be empty"));
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
            risk_atlas_database_url,
            token_pnl_database_url,
            mempool_signal_limit,
        })
    }
}

pub fn set_config_path(path: impl Into<PathBuf>) -> Result<()> {
    CONFIG_PATH
        .set(path.into())
        .map_err(|path| eyre!("config path already set to {}", path.display()))
}

pub fn active_config_path() -> PathBuf {
    CONFIG_PATH
        .get()
        .cloned()
        .unwrap_or_else(default_config_path)
}

pub fn shared_config_value(key: &str) -> Result<Option<String>> {
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

fn default_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("config.env")
}

fn load_config_env() -> Result<HashMap<String, String>> {
    load_config_env_from_path(&active_config_path())
}

fn load_config_env_from_path(path: &Path) -> Result<HashMap<String, String>> {
    let contents = fs::read_to_string(&path).map_err(|err| {
        eyre!(
            "failed to read shared config file {}: {err}",
            path.display()
        )
    })?;
    let mut values = parse_env_config(&contents);
    merge_toml_database_config(&mut values, &toml_config_path_for(path))?;
    Ok(values)
}

fn toml_config_path_for(path: &Path) -> PathBuf {
    if path.extension().and_then(|value| value.to_str()) == Some("toml") {
        path.to_path_buf()
    } else {
        path.with_file_name("config.toml")
    }
}

fn merge_toml_database_config(values: &mut HashMap<String, String>, path: &Path) -> Result<()> {
    let contents = fs::read_to_string(path).map_err(|err| {
        eyre!(
            "failed to read shared TOML config file {}: {err}",
            path.display()
        )
    })?;
    let root = contents.parse::<toml::Value>().map_err(|err| {
        eyre!(
            "failed to parse shared TOML config file {}: {err}",
            path.display()
        )
    })?;
    insert_toml_database_url(values, &root, "mempool", MEMPOOL_DATABASE_CONFIG_KEY);
    insert_toml_database_url(values, &root, "alpha", ALPHA_DATABASE_CONFIG_KEY);
    insert_toml_database_url(values, &root, "risk_atlas", RISK_ATLAS_DATABASE_CONFIG_KEY);
    insert_toml_database_url(values, &root, "token_pnl", TOKEN_PNL_DATABASE_CONFIG_KEY);
    Ok(())
}

fn insert_toml_database_url(
    values: &mut HashMap<String, String>,
    root: &toml::Value,
    database_name: &str,
    key: &str,
) {
    if let Some(url) = root
        .get("databases")
        .and_then(|value| value.get(database_name))
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        values.insert(key.to_string(), url.to_string());
    }
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
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn required_config_string(config: &HashMap<String, String>, key: &str) -> Result<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| eyre!("{key} must be set in {}", active_config_path().display()))
}

fn config_value<'a>(config: &'a HashMap<String, String>, key: &str) -> Option<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
            databases.mempool.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.alpha.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.risk_atlas.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.token_pnl.url=postgresql://postgres:postgres@localhost:5432/eth_db
            "#,
        );

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert_eq!(config.live_warmup_blocks, 7_000);
        assert_eq!(config.execution_rpc, "http://127.0.0.1:8545");
        assert_eq!(config.execution_ws, "ws://127.0.0.1:8546");
    }

    #[test]
    fn ignores_non_canonical_warmup_key() {
        let values = parse_env_config(
            r#"
            CHAIN_SERVER_LIVE_WARMUP_BLOCKS=2000
            databases.mempool.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.alpha.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.risk_atlas.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.token_pnl.url=postgresql://postgres:postgres@localhost:5432/eth_db
            "#,
        );

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert_eq!(config.live_warmup_blocks, DEFAULT_LIVE_WARMUP_BLOCKS);
    }

    #[test]
    fn strips_quotes_from_config_values() {
        let values = parse_env_config(
            r#"
            CHAIN_SERVER_BIND="127.0.0.1:9999"
            LIVE_TOKEN_TRACKER_WARMUP_BLOCKS='123'
            databases.mempool.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.alpha.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.risk_atlas.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.token_pnl.url=postgresql://postgres:postgres@localhost:5432/eth_db
            "#,
        );

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert_eq!(config.bind.to_string(), "127.0.0.1:9999");
        assert_eq!(config.live_warmup_blocks, 123);
    }

    #[test]
    fn reads_auto_start_live_flag() {
        let values = parse_env_config(
            r#"
            CHAIN_SERVER_AUTO_START_LIVE=false
            databases.mempool.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.alpha.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.risk_atlas.url=postgresql://postgres:postgres@localhost:5432/eth_db
            databases.token_pnl.url=postgresql://postgres:postgres@localhost:5432/eth_db
            "#,
        );

        let config = ChainServerConfig::from_config_values(&values).unwrap();

        assert!(!config.auto_start_live);
    }
}
