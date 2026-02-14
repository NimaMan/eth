//! Configuration validation

use super::Config;
use crate::common::{errors::ConfigError, Result};

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a configuration
    pub fn validate(config: &Config) -> Result<()> {
        // Network validation
        Self::validate_network(&config.network)?;

        // Trading validation
        Self::validate_trading(&config.trading)?;

        // Risk validation
        Self::validate_risk(&config.risk)?;

        // Performance validation
        Self::validate_performance(&config.performance)?;

        // Monitoring validation
        Self::validate_monitoring(&config.monitoring)?;

        Ok(())
    }

    fn validate_network(network: &super::NetworkConfig) -> Result<()> {
        // Validate chain ID
        if network.chain_id != 1 && network.chain_id != 5 && network.chain_id != 11155111 {
            return Err(ConfigError::InvalidValue {
                field: "chain_id".to_string(),
                reason: "Must be 1 (mainnet), 5 (goerli), or 11155111 (sepolia)".to_string(),
            }
            .into());
        }

        // Validate URLs
        if !network.rpc_url.starts_with("http://") && !network.rpc_url.starts_with("https://") {
            return Err(ConfigError::InvalidValue {
                field: "rpc_url".to_string(),
                reason: "Must start with http:// or https://".to_string(),
            }
            .into());
        }

        if !network.ws_url.starts_with("ws://") && !network.ws_url.starts_with("wss://") {
            return Err(ConfigError::InvalidValue {
                field: "ws_url".to_string(),
                reason: "Must start with ws:// or wss://".to_string(),
            }
            .into());
        }

        Ok(())
    }

    fn validate_trading(trading: &super::TradingConfig) -> Result<()> {
        // Validate keystore path exists
        if !trading.keystore_path.exists() {
            return Err(ConfigError::FileNotFound {
                path: trading.keystore_path.display().to_string(),
            }
            .into());
        }

        // Validate gas price
        if trading.max_gas_price_gwei <= 0.0 || trading.max_gas_price_gwei > 10000.0 {
            return Err(ConfigError::InvalidValue {
                field: "max_gas_price_gwei".to_string(),
                reason: "Must be between 0 and 10000".to_string(),
            }
            .into());
        }

        // Validate slippage
        if trading.default_slippage < 0.0 || trading.default_slippage > trading.max_slippage {
            return Err(ConfigError::InvalidValue {
                field: "default_slippage".to_string(),
                reason: "Must be >= 0 and <= max_slippage".to_string(),
            }
            .into());
        }

        if trading.max_slippage > 0.1 {
            return Err(ConfigError::InvalidValue {
                field: "max_slippage".to_string(),
                reason: "Must be <= 0.1 (10%)".to_string(),
            }
            .into());
        }

        Ok(())
    }

    fn validate_risk(risk: &super::RiskConfig) -> Result<()> {
        // Validate position limits
        if risk.max_position_usd <= 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "max_position_usd".to_string(),
                reason: "Must be > 0".to_string(),
            }
            .into());
        }

        if risk.max_daily_loss_usd <= 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "max_daily_loss_usd".to_string(),
                reason: "Must be > 0".to_string(),
            }
            .into());
        }

        if risk.min_eth_balance <= 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "min_eth_balance".to_string(),
                reason: "Must be > 0".to_string(),
            }
            .into());
        }

        // Validate circuit breaker
        if risk.max_consecutive_failures == 0 {
            return Err(ConfigError::InvalidValue {
                field: "max_consecutive_failures".to_string(),
                reason: "Must be > 0".to_string(),
            }
            .into());
        }

        Ok(())
    }

    fn validate_performance(perf: &super::PerformanceConfig) -> Result<()> {
        // Validate execution time
        if perf.max_execution_time_ms == 0 || perf.max_execution_time_ms > 5000 {
            return Err(ConfigError::InvalidValue {
                field: "max_execution_time_ms".to_string(),
                reason: "Must be between 1 and 5000".to_string(),
            }
            .into());
        }

        // Validate cache TTLs
        if perf.position_cache_ttl_seconds == 0 {
            return Err(ConfigError::InvalidValue {
                field: "position_cache_ttl_seconds".to_string(),
                reason: "Must be > 0".to_string(),
            }
            .into());
        }

        Ok(())
    }

    fn validate_monitoring(monitoring: &super::MonitoringConfig) -> Result<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&monitoring.log_level.as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "log_level".to_string(),
                reason: format!("Must be one of: {:?}", valid_levels),
            }
            .into());
        }

        // Validate ports
        if monitoring.metrics_port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "metrics_port".to_string(),
                reason: "Must be > 0".to_string(),
            }
            .into());
        }

        if monitoring.health_check_port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "health_check_port".to_string(),
                reason: "Must be > 0".to_string(),
            }
            .into());
        }

        Ok(())
    }
}
