use serde::{Deserialize, Serialize};
/// Centralized Configuration for Mempool Processor
///
/// This module contains all configuration parameters for the mempool processor
/// system, providing a single source of truth for all settings.
use std::path::Path;
use std::time::Duration;

/// Default location of the local Reth data directory used by the processor.
pub const DEFAULT_RETH_DATA_DIR: &str = "/home/nima/.local/share/reth/mainnet";
/// Default IPC socket path exposed by the local Reth node.
pub const DEFAULT_RETH_IPC_PATH: &str = "/home/nima/.local/share/reth/mainnet/reth.ipc";
/// Default number of simulation worker threads.
pub const DEFAULT_SIM_WORKERS: usize = 4;
/// Default log directory within the repository.
pub const DEFAULT_LOG_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/logs");
pub const DEFAULT_TOKEN_CACHE_PUB_ENDPOINT: &str = "tcp://127.0.0.1:5557";
pub const DEFAULT_TOKEN_CACHE_REP_ENDPOINT: &str = "tcp://127.0.0.1:5558";
pub const DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL: &str = "redis://localhost:6379/0";
pub const DEFAULT_REDIS_TOKEN_PREFIX: &str = "token:snapshot:";

fn default_simulation_workers() -> usize {
    DEFAULT_SIM_WORKERS
}

fn default_live_data_redis_url() -> String {
    std::env::var("LIVE_BLOCKCHAIN_DATA_REDIS_URL")
        .unwrap_or_else(|_| DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string())
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
    /// ZMQ REP endpoint for token cache queries
    pub zmq_rep_endpoint: String,
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
            zmq_rep_endpoint: DEFAULT_TOKEN_CACHE_REP_ENDPOINT.to_string(),
            redis_url: DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string(),
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
                socket_path: DEFAULT_RETH_IPC_PATH.to_string(),
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
                reth_datadir: DEFAULT_RETH_DATA_DIR.to_string(),
                worker_threads: DEFAULT_SIM_WORKERS,
                batch_size: 50,
                batch_timeout: Duration::from_millis(100),
                max_queue_size: 1000,
                skip_simple_transfers: true,
                min_value_eth: 0.01,
                live_data_redis_url: DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string(),
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
                log_dir: DEFAULT_LOG_DIR.to_string(),
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
        if let Ok(path) = std::env::var("MEMPOOL_IPC_PATH") {
            config.ipc.socket_path = path;
        }

        if let Ok(dir) = std::env::var("MEMPOOL_RETH_DATADIR") {
            config.simulation.reth_datadir = dir;
        }

        if let Ok(workers) = std::env::var("MEMPOOL_SIM_WORKERS") {
            if let Ok(val) = workers.parse() {
                config.simulation.worker_threads = val;
            }
        }

        if let Ok(url) = std::env::var("LIVE_BLOCKCHAIN_DATA_REDIS_URL") {
            config.simulation.live_data_redis_url = url;
        }

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

        if let Ok(rep_endpoint) = std::env::var("MEMPOOL_TOKEN_CACHE_REP_ENDPOINT") {
            config.token_cache_source.zmq_rep_endpoint = rep_endpoint;
        }

        if let Ok(threshold) = std::env::var("MEMPOOL_TOKEN_CACHE_ETH_THRESHOLD") {
            if let Ok(val) = threshold.parse::<f64>() {
                config.token_cache_source.eth_threshold = val;
            }
        }

        // Honeypot threshold removed - now determined by can't sell condition

        if config.ipc.socket_path == DEFAULT_RETH_IPC_PATH {
            let derived = Path::new(&config.simulation.reth_datadir).join("reth.ipc");
            config.ipc.socket_path = derived.to_string_lossy().into_owned();
        }

        config
    }
}

/// Helper function to create example configuration file
pub fn create_example_config(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = MempoolProcessorConfig::default();
    config.to_file(path)?;
    println!("Created example configuration at: {}", path);
    Ok(())
}
