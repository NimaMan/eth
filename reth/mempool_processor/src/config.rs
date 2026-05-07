use serde::{Deserialize, Serialize};
/// Centralized Configuration for Mempool Processor
///
/// This module contains all configuration parameters for the mempool processor
/// system, providing a single source of truth for all settings.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const ETH_CONFIG_PATH_ENV: &str = "ETH_CONFIG_PATH";
pub const DEFAULT_ETH_CONFIG_PATH: &str = "/home/nima/code/crypto/blockchains/eth/config.env";
pub const RETH_DATADIR_ENV: &str = "RETH_DATADIR";
pub const RETH_DB_PATH_ENV: &str = "RETH_DB_PATH";
pub const RETH_IPC_PATH_ENV: &str = "RETH_IPC_PATH";
pub const IPC_PATH_ENV: &str = "IPC_PATH";
pub const MEMPOOL_IPC_PATH_ENV: &str = "MEMPOOL_IPC_PATH";
pub const MEMPOOL_RETH_DATADIR_ENV: &str = "MEMPOOL_RETH_DATADIR";
pub const MEMPOOL_LOG_DIR_ENV: &str = "MEMPOOL_LOG_DIR";
pub const ETH_LOG_DIR_ENV: &str = "ETH_LOG_DIR";
pub const ETH_RPC_URL_ENV: &str = "ETH_RPC_URL";
pub const RETH_HTTP_RPC_ENV: &str = "RETH_HTTP_RPC";
pub const LIVE_BLOCKCHAIN_DATA_REDIS_URL_ENV: &str = "LIVE_BLOCKCHAIN_DATA_REDIS_URL";

/// Default location of the local Reth data directory used by the processor.
pub const DEFAULT_RETH_DATA_DIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
/// Default IPC socket path exposed by the local Reth node.
pub const DEFAULT_RETH_IPC_PATH: &str = "/home/nima/storage/samsung8tb/ethereum/reth/reth.ipc";
/// Default HTTP RPC endpoint exposed by the local Reth node.
pub const DEFAULT_ETH_RPC_URL: &str = "http://127.0.0.1:8545";
/// Default number of simulation worker threads.
pub const DEFAULT_SIM_WORKERS: usize = 4;
/// Default log directory within the shared Ethereum workspace.
pub const DEFAULT_LOG_DIR: &str = "/home/nima/code/crypto/blockchains/eth/logs/mempool_processor";
pub const DEFAULT_TOKEN_CACHE_PUB_ENDPOINT: &str = "tcp://127.0.0.1:5557";
pub const DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL: &str = "redis://localhost:6379/0";
pub const DEFAULT_REDIS_TOKEN_PREFIX: &str = "eth/live/token/snapshot/";

/// Path to the shared Ethereum workspace config.
pub fn eth_config_path() -> PathBuf {
    std::env::var_os(ETH_CONFIG_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("config.env")
        })
}

/// Reth data directory for examples and runtime defaults.
pub fn reth_datadir_from_env() -> String {
    explicit_or_shared_config_value(
        &[MEMPOOL_RETH_DATADIR_ENV, RETH_DATADIR_ENV],
        &[RETH_DB_PATH_ENV],
    )
    .unwrap_or_else(|| DEFAULT_RETH_DATA_DIR.to_string())
}

/// IPC socket path for examples and lightweight tools.
///
/// Resolution order is explicit environment variables, then the shared
/// repository `config.env`, then `<RETH_DATADIR>/reth.ipc`.
pub fn reth_ipc_path_from_env() -> String {
    explicit_or_shared_config_value(&[MEMPOOL_IPC_PATH_ENV, RETH_IPC_PATH_ENV], &[IPC_PATH_ENV])
        .unwrap_or_else(|| {
            Path::new(&reth_datadir_from_env())
                .join("reth.ipc")
                .to_string_lossy()
                .into_owned()
        })
}

/// HTTP RPC endpoint for examples that need `txpool_*` or trace RPC calls.
pub fn eth_rpc_url_from_env() -> String {
    config_value(&[ETH_RPC_URL_ENV, RETH_HTTP_RPC_ENV])
        .unwrap_or_else(|| DEFAULT_ETH_RPC_URL.to_string())
}

/// Redis URL for live blockchain/token snapshot data.
pub fn live_data_redis_url_from_env() -> String {
    config_value(&[LIVE_BLOCKCHAIN_DATA_REDIS_URL_ENV])
        .unwrap_or_else(|| DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string())
}

/// Log directory for the mempool processor.
pub fn mempool_log_dir_from_env() -> String {
    if let Some(log_dir) = config_value(&[MEMPOOL_LOG_DIR_ENV]) {
        return log_dir;
    }

    if let Some(eth_log_dir) = config_value(&[ETH_LOG_DIR_ENV]) {
        return Path::new(&eth_log_dir)
            .join("mempool_processor")
            .to_string_lossy()
            .into_owned();
    }

    DEFAULT_LOG_DIR.to_string()
}

fn config_value(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    let config = load_config_env();
    for key in keys {
        if let Some(value) = config.get(*key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn explicit_or_shared_config_value(
    explicit_keys: &[&str],
    legacy_env_keys: &[&str],
) -> Option<String> {
    for key in explicit_keys {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    let config = load_config_env();
    for key in explicit_keys.iter().chain(legacy_env_keys.iter()) {
        if let Some(value) = config.get(*key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    for key in legacy_env_keys {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn load_config_env() -> HashMap<String, String> {
    let contents = match std::fs::read_to_string(eth_config_path()) {
        Ok(contents) => contents,
        Err(_) => return HashMap::new(),
    };
    parse_env_config(&contents)
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
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
}

fn default_simulation_workers() -> usize {
    DEFAULT_SIM_WORKERS
}

fn default_live_data_redis_url() -> String {
    live_data_redis_url_from_env()
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolProcessorConfig {
    /// IPC connection settings
    pub ipc: IpcConfig,

    /// Function detection settings
    pub function_detection: FunctionDetectionConfig,

    /// Tax detection and alerting settings
    pub tax_detection: TaxDetectionConfig,

    /// Signal detection thresholds
    pub signal_detection: SignalDetectionConfig,

    /// Transaction simulation settings
    pub simulation: SimulationConfig,

    /// Database settings
    pub database: DatabaseConfig,

    /// ZMQ publisher settings
    pub zmq: ZmqConfig,

    /// Logging settings
    pub logging: LoggingConfig,

    /// Token cache source settings
    #[serde(default)]
    pub token_cache_source: TokenCacheSourceConfig,
}

/// IPC connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcConfig {
    /// Path to Reth IPC socket
    pub socket_path: String,

    /// Buffer size for IPC reads
    pub buffer_size: usize,

    /// Reconnection delay after disconnect
    pub reconnect_delay: Duration,

    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
}

/// Function detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDetectionConfig {
    /// Enable function detection
    pub enabled: bool,

    /// Batch size for processing transactions
    pub batch_size: usize,

    /// Channel buffer size
    pub channel_buffer_size: usize,

    /// Processing timeout
    pub processing_timeout: Duration,
}

/// Tax detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxDetectionConfig {
    /// Enable tax detection
    pub enabled: bool,

    /// Maximum acceptable buy tax (percentage)
    pub max_acceptable_buy_tax: u8,

    /// Maximum acceptable sell tax (percentage)
    pub max_acceptable_sell_tax: u8,

    /// Alert on any tax change
    pub alert_on_any_change: bool,

    /// Alert only on increases
    pub alert_only_increases: bool,

    /// Minimum tax change to alert (percentage points)
    pub min_change_threshold: u8,
}

impl Default for TaxDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_acceptable_buy_tax: 30,
            max_acceptable_sell_tax: 30,
            alert_on_any_change: false,
            alert_only_increases: true,
            min_change_threshold: 5,
        }
    }
}

/// Signal detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalDetectionConfig {
    /// Minimum ETH in pool to track
    pub min_pool_eth: f64,

    /// Scam detection ETH threshold
    pub scam_eth_threshold: f64,

    /// Scam detection percentage threshold
    pub scam_percentage_threshold: f64,

    /// Liquidity warning percentage
    pub liquidity_warning_percentage: f64,

    /// Supply increase alert percentage
    pub supply_increase_percentage: f64,

    /// Minimum confidence score
    pub min_confidence: f64,
}

/// Simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Enable transaction simulation
    pub enabled: bool,

    /// Reth data directory path
    pub reth_datadir: String,

    /// Number of simulation worker threads
    #[serde(default = "default_simulation_workers")]
    pub worker_threads: usize,

    /// Batch size for simulation
    pub batch_size: usize,

    /// Batch timeout
    pub batch_timeout: Duration,

    /// Maximum queue size
    pub max_queue_size: usize,

    /// Skip simple transfers
    pub skip_simple_transfers: bool,

    /// Minimum value for simulation (in ETH)
    pub min_value_eth: f64,

    /// Redis URL used to hydrate live chain data for ahead-of-MDBX simulations
    #[serde(default = "default_live_data_redis_url")]
    pub live_data_redis_url: String,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Enable database writes
    pub enabled: bool,

    /// Database connection URL
    pub url: Option<String>,

    /// Connection pool size
    pub pool_size: u32,

    /// Write batch size
    pub batch_size: usize,

    /// Write interval
    pub write_interval: Duration,
}

/// ZMQ configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZmqConfig {
    /// Enable ZMQ publishing
    pub enabled: bool,

    /// Signal publisher endpoint
    pub signal_endpoint: String,

    /// Alert publisher endpoint
    pub alert_endpoint: String,

    /// Send high water mark
    pub send_hwm: i32,

    /// Linger period (ms)
    pub linger: i32,
}

/// Live token cache source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCacheSourceConfig {
    /// Minimum ETH threshold used by cache heuristics
    pub eth_threshold: f64,
    /// ZMQ PUB endpoint for token update notifications
    pub zmq_pub_endpoint: String,
    /// Redis URL hosting live token snapshots
    pub redis_url: String,
    /// Key prefix for token snapshots
    pub redis_token_prefix: String,
}

impl Default for TokenCacheSourceConfig {
    fn default() -> Self {
        Self {
            eth_threshold: 0.1,
            zmq_pub_endpoint: DEFAULT_TOKEN_CACHE_PUB_ENDPOINT.to_string(),
            redis_url: live_data_redis_url_from_env(),
            redis_token_prefix: DEFAULT_REDIS_TOKEN_PREFIX.to_string(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log directory path
    pub log_dir: String,

    /// Enable file logging
    pub file_logging: bool,

    /// Log level
    pub level: String,

    /// Performance metrics interval
    pub metrics_interval: Duration,
}

impl Default for MempoolProcessorConfig {
    fn default() -> Self {
        Self {
            ipc: IpcConfig {
                socket_path: reth_ipc_path_from_env(),
                buffer_size: 65536,
                reconnect_delay: Duration::from_secs(5),
                max_reconnect_attempts: 10,
            },

            function_detection: FunctionDetectionConfig {
                enabled: true,
                batch_size: 100,
                channel_buffer_size: 50000,
                processing_timeout: Duration::from_millis(100),
            },

            tax_detection: TaxDetectionConfig {
                enabled: true,
                max_acceptable_buy_tax: 30,
                max_acceptable_sell_tax: 30,
                alert_on_any_change: false,
                alert_only_increases: true,
                min_change_threshold: 5,
            },

            signal_detection: SignalDetectionConfig {
                min_pool_eth: 0.7,
                scam_eth_threshold: 0.3,
                scam_percentage_threshold: 60.0,
                liquidity_warning_percentage: 20.0,
                supply_increase_percentage: 50.0,
                min_confidence: 0.7,
            },

            simulation: SimulationConfig {
                enabled: true,
                reth_datadir: reth_datadir_from_env(),
                worker_threads: DEFAULT_SIM_WORKERS,
                batch_size: 50,
                batch_timeout: Duration::from_millis(100),
                max_queue_size: 1000,
                skip_simple_transfers: true,
                min_value_eth: 0.01,
                live_data_redis_url: live_data_redis_url_from_env(),
            },

            database: DatabaseConfig {
                enabled: false,
                url: None,
                pool_size: 10,
                batch_size: 100,
                write_interval: Duration::from_secs(10),
            },

            zmq: ZmqConfig {
                enabled: true,
                signal_endpoint: "tcp://127.0.0.1:5556".to_string(),
                alert_endpoint: "tcp://127.0.0.1:5557".to_string(),
                send_hwm: 10000,
                linger: 0,
            },

            logging: LoggingConfig {
                log_dir: mempool_log_dir_from_env(),
                file_logging: true,
                level: "info".to_string(),
                metrics_interval: Duration::from_secs(60),
            },
            token_cache_source: TokenCacheSourceConfig::default(),
        }
    }
}

impl MempoolProcessorConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Load from environment variables with prefix MEMPOOL_
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Override with environment variables
        config.ipc.socket_path = reth_ipc_path_from_env();
        config.simulation.reth_datadir = reth_datadir_from_env();

        if let Ok(workers) = std::env::var("MEMPOOL_SIM_WORKERS") {
            if let Ok(val) = workers.parse() {
                config.simulation.worker_threads = val;
            }
        }

        config.simulation.live_data_redis_url = live_data_redis_url_from_env();
        config.logging.log_dir = mempool_log_dir_from_env();

        if let Ok(endpoint) = std::env::var("MEMPOOL_ZMQ_SIGNAL_ENDPOINT") {
            config.zmq.signal_endpoint = endpoint;
        }

        if let Ok(endpoint) = std::env::var("MEMPOOL_ZMQ_ALERT_ENDPOINT") {
            config.zmq.alert_endpoint = endpoint;
        }

        if let Ok(url) = std::env::var("MEMPOOL_DATABASE_URL") {
            config.database.url = Some(url);
            config.database.enabled = true;
        }

        if let Ok(redis_url) = std::env::var("MEMPOOL_TOKEN_CACHE_REDIS_URL") {
            config.token_cache_source.redis_url = redis_url;
        }

        if let Ok(prefix) = std::env::var("MEMPOOL_TOKEN_CACHE_REDIS_PREFIX") {
            config.token_cache_source.redis_token_prefix = prefix;
        }

        if let Ok(pub_endpoint) = std::env::var("MEMPOOL_TOKEN_CACHE_PUB_ENDPOINT") {
            config.token_cache_source.zmq_pub_endpoint = pub_endpoint;
        }

        if let Ok(threshold) = std::env::var("MEMPOOL_TOKEN_CACHE_ETH_THRESHOLD") {
            if let Ok(val) = threshold.parse::<f64>() {
                config.token_cache_source.eth_threshold = val;
            }
        }

        // Honeypot threshold removed - now determined by can't sell condition

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shared_config_env_values() {
        let values = parse_env_config(
            r#"
            # shared config
            RETH_DATADIR=/mnt/eth/reth
            RETH_IPC_PATH="/mnt/eth/reth/reth.ipc"
            RETH_HTTP_RPC='http://127.0.0.1:8545'
            "#,
        );

        assert_eq!(values["RETH_DATADIR"], "/mnt/eth/reth");
        assert_eq!(values["RETH_IPC_PATH"], "/mnt/eth/reth/reth.ipc");
        assert_eq!(values["RETH_HTTP_RPC"], "http://127.0.0.1:8545");
    }
}

/// Helper function to create example configuration file
pub fn create_example_config(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = MempoolProcessorConfig::default();
    config.to_file(path)?;
    println!("Created example configuration at: {}", path);
    Ok(())
}
