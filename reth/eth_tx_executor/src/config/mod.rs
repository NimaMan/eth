//! Centralized configuration management
//! 
//! Handles loading configuration from environment variables, TOML files,
//! and provides validation and type-safe access to all settings.

use crate::common::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ethers::types::Address;

mod loader;
mod validator;

pub use loader::ConfigLoader;
pub use validator::ConfigValidator;

/// Main configuration structure containing all settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Trading configuration
    pub trading: TradingConfig,
    
    /// Risk management configuration
    pub risk: RiskConfig,
    
    /// Performance tuning
    pub performance: PerformanceConfig,
    
    /// Monitoring and alerting
    pub monitoring: MonitoringConfig,
    
    /// Security settings
    pub security: SecurityConfig,
}

/// Network and RPC configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkConfig {
    /// Chain ID (1 for mainnet)
    pub chain_id: u64,
    
    /// Primary RPC endpoint
    pub rpc_url: String,
    
    /// Backup RPC endpoints
    #[serde(default)]
    pub backup_rpc_urls: Vec<String>,
    
    /// WebSocket URL for real-time data
    pub ws_url: String,
    
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    
    /// Maximum retry attempts
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

/// Trading execution configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingConfig {
    /// Wallet keystore path
    pub keystore_path: PathBuf,
    
    /// Maximum gas price in gwei
    pub max_gas_price_gwei: f64,
    
    /// Default slippage tolerance (e.g., 0.01 = 1%)
    pub default_slippage: f64,
    
    /// Maximum slippage allowed
    pub max_slippage: f64,
    
    /// Minimum profit threshold (e.g., 0.001 = 0.1%)
    pub min_profit_threshold: f64,
    
    /// Enable Flashbots for high priority
    #[serde(default = "default_true")]
    pub flashbots_enabled: bool,
    
    /// Flashbots RPC endpoint
    pub flashbots_rpc: Option<String>,
    
    /// ZMQ alert receiver bind address
    #[serde(default = "default_zmq_bind")]
    pub zmq_bind_address: String,
}

/// Risk management configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskConfig {
    /// Maximum position size per token in USD
    pub max_position_usd: f64,
    
    /// Maximum daily loss in USD
    pub max_daily_loss_usd: f64,
    
    /// Minimum ETH balance to maintain
    pub min_eth_balance: f64,
    
    /// Maximum consecutive failures before circuit break
    pub max_consecutive_failures: u32,
    
    /// Circuit breaker cooldown in seconds
    pub circuit_breaker_cooldown_seconds: u64,
    
    /// Token blacklist
    #[serde(default)]
    pub blacklisted_tokens: Vec<Address>,
    
    /// Token whitelist (if specified, only these are allowed)
    #[serde(default)]
    pub whitelisted_tokens: Option<Vec<Address>>,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    
    /// Alert expiry time in seconds
    pub alert_expiry_seconds: u64,
    
    /// Position cache TTL in seconds
    pub position_cache_ttl_seconds: u64,
    
    /// Gas price cache TTL in milliseconds
    pub gas_price_cache_ttl_ms: u64,
    
    /// Mempool scan interval in milliseconds
    pub mempool_scan_interval_ms: u64,
    
    /// Maximum concurrent operations
    pub max_concurrent_operations: usize,
}

/// Monitoring and alerting configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
    
    /// Metrics export port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    
    /// Enable health check endpoint
    #[serde(default = "default_true")]
    pub health_check_enabled: bool,
    
    /// Health check port
    #[serde(default = "default_health_port")]
    pub health_check_port: u16,
    
    /// Alert webhook URL (Discord, Slack, etc.)
    pub alert_webhook_url: Option<String>,
    
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    /// Enable transaction simulation before execution
    #[serde(default = "default_true")]
    pub simulate_before_execute: bool,
    
    /// Require manual approval for high-value transactions
    #[serde(default)]
    pub require_manual_approval: bool,
    
    /// High-value threshold in USD
    pub high_value_threshold_usd: Option<f64>,
    
    /// Allowed operator addresses (for remote control)
    #[serde(default)]
    pub allowed_operators: Vec<Address>,
}

// Default value functions for serde
fn default_timeout() -> u64 { 30 }
fn default_retries() -> u32 { 3 }
fn default_true() -> bool { true }
fn default_zmq_bind() -> String { "tcp://127.0.0.1:5559".to_string() }
fn default_metrics_port() -> u16 { 9090 }
fn default_health_port() -> u16 { 8080 }
fn default_log_level() -> String { "info".to_string() }

impl Config {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        ConfigLoader::from_file(path)
    }
    
    /// Load configuration from environment
    pub fn from_env() -> Result<Self> {
        ConfigLoader::from_env()
    }
    
    /// Load with environment overrides
    pub fn from_file_with_env<P: AsRef<Path>>(path: P) -> Result<Self> {
        ConfigLoader::from_file_with_env(path)
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        ConfigValidator::validate(self)
    }
    
    /// Get environment name
    pub fn environment(&self) -> &str {
        if self.network.chain_id == 1 {
            "mainnet"
        } else {
            "testnet"
        }
    }
    
    /// Check if running in production
    pub fn is_production(&self) -> bool {
        self.network.chain_id == 1
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                chain_id: 1,
                rpc_url: "http://127.0.0.1:8545".to_string(),
                backup_rpc_urls: vec![],
                ws_url: "ws://127.0.0.1:8546".to_string(),
                timeout_seconds: 30,
                max_retries: 3,
            },
            trading: TradingConfig {
                keystore_path: PathBuf::from("keystore"),
                max_gas_price_gwei: 500.0,
                default_slippage: 0.01,
                max_slippage: 0.05,
                min_profit_threshold: 0.001,
                flashbots_enabled: true,
                flashbots_rpc: None,
                zmq_bind_address: "tcp://127.0.0.1:5559".to_string(),
            },
            risk: RiskConfig {
                max_position_usd: 10_000.0,
                max_daily_loss_usd: 1_000.0,
                min_eth_balance: 0.1,
                max_consecutive_failures: 5,
                circuit_breaker_cooldown_seconds: 300,
                blacklisted_tokens: vec![],
                whitelisted_tokens: None,
            },
            performance: PerformanceConfig {
                max_execution_time_ms: 200,
                alert_expiry_seconds: 60,
                position_cache_ttl_seconds: 5,
                gas_price_cache_ttl_ms: 500,
                mempool_scan_interval_ms: 100,
                max_concurrent_operations: 10,
            },
            monitoring: MonitoringConfig {
                metrics_enabled: true,
                metrics_port: 9090,
                health_check_enabled: true,
                health_check_port: 8080,
                alert_webhook_url: None,
                log_level: "info".to_string(),
            },
            security: SecurityConfig {
                simulate_before_execute: true,
                require_manual_approval: false,
                high_value_threshold_usd: None,
                allowed_operators: vec![],
            },
        }
    }
}