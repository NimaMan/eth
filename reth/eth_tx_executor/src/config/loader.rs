//! Configuration loading utilities

use super::Config;
use crate::common::{errors::ConfigError, Result};
use std::fs;
use std::path::Path;

/// Configuration loader
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|_| ConfigError::FileNotFound {
            path: path.display().to_string(),
        })?;

        toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()).into())
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Config> {
        // Start with defaults
        let mut config = Config::default();

        // Override with environment variables
        if let Ok(chain_id) = std::env::var("ETH_KARTAL_CHAIN_ID") {
            config.network.chain_id = chain_id.parse().map_err(|_| ConfigError::InvalidValue {
                field: "chain_id".to_string(),
                reason: "Must be a valid u64".to_string(),
            })?;
        }

        if let Ok(rpc_url) = std::env::var("ETH_KARTAL_RPC_URL") {
            config.network.rpc_url = rpc_url;
        }

        if let Ok(ws_url) = std::env::var("ETH_KARTAL_WS_URL") {
            config.network.ws_url = ws_url;
        }

        if let Ok(keystore) = std::env::var("ETH_KARTAL_KEYSTORE_PATH") {
            config.trading.keystore_path = keystore.into();
        }

        if let Ok(max_gas) = std::env::var("ETH_KARTAL_MAX_GAS_PRICE_GWEI") {
            config.trading.max_gas_price_gwei =
                max_gas.parse().map_err(|_| ConfigError::InvalidValue {
                    field: "max_gas_price_gwei".to_string(),
                    reason: "Must be a valid f64".to_string(),
                })?;
        }

        if let Ok(log_level) = std::env::var("ETH_KARTAL_LOG_LEVEL") {
            config.monitoring.log_level = log_level;
        }

        Ok(config)
    }

    /// Load from file with environment overrides
    pub fn from_file_with_env<P: AsRef<Path>>(path: P) -> Result<Config> {
        // Load base config from file
        let mut config = Self::from_file(path)?;

        // Apply environment overrides
        if let Ok(env_config) = Self::from_env() {
            // Selectively override fields that are set in environment
            if std::env::var("ETH_KARTAL_CHAIN_ID").is_ok() {
                config.network.chain_id = env_config.network.chain_id;
            }
            if std::env::var("ETH_KARTAL_RPC_URL").is_ok() {
                config.network.rpc_url = env_config.network.rpc_url;
            }
            if std::env::var("ETH_KARTAL_WS_URL").is_ok() {
                config.network.ws_url = env_config.network.ws_url;
            }
            if std::env::var("ETH_KARTAL_KEYSTORE_PATH").is_ok() {
                config.trading.keystore_path = env_config.trading.keystore_path;
            }
            if std::env::var("ETH_KARTAL_MAX_GAS_PRICE_GWEI").is_ok() {
                config.trading.max_gas_price_gwei = env_config.trading.max_gas_price_gwei;
            }
            if std::env::var("ETH_KARTAL_LOG_LEVEL").is_ok() {
                config.monitoring.log_level = env_config.monitoring.log_level;
            }
        }

        Ok(config)
    }
}
