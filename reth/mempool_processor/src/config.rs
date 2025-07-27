/// Centralized Configuration for Mempool Processor
/// 
/// This module contains all configuration parameters for the mempool processor
/// system, providing a single source of truth for all settings.

use std::time::Duration;
use serde::{Deserialize, Serialize};

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
    
    /// Honeypot sell tax threshold (percentage)
    pub honeypot_sell_threshold: u8,
    
    /// Alert on any tax change
    pub alert_on_any_change: bool,
    
    /// Alert only on increases
    pub alert_only_increases: bool,
    
    /// Minimum tax change to alert (percentage points)
    pub min_change_threshold: u8,
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
                socket_path: "/tmp/reth.ipc".to_string(),
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
                honeypot_sell_threshold: 50,
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
                reth_datadir: "/home/nima/.local/share/reth/mainnet".to_string(),
                batch_size: 50,
                batch_timeout: Duration::from_millis(100),
                max_queue_size: 1000,
                skip_simple_transfers: true,
                min_value_eth: 0.01,
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
                log_dir: "/home/nima/code/crypto/logs/mempool".to_string(),
                file_logging: true,
                level: "info".to_string(),
                metrics_interval: Duration::from_secs(60),
            },
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
        
        if let Ok(url) = std::env::var("MEMPOOL_DATABASE_URL") {
            config.database.url = Some(url);
            config.database.enabled = true;
        }
        
        if let Ok(threshold) = std::env::var("MEMPOOL_TAX_HONEYPOT_THRESHOLD") {
            if let Ok(val) = threshold.parse() {
                config.tax_detection.honeypot_sell_threshold = val;
            }
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