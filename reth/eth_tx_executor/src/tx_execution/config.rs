/*
* Transaction Execution Configuration
*
* This module contains configuration structures for the transaction execution system.
*/

use ethers::types::{Address, U256};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use std::collections::HashMap;

/// Base configuration for the transaction execution system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxExecutionConfig {
    /// RPC endpoints
    pub rpc_endpoints: Vec<String>,
    
    /// Chain ID
    pub chain_id: u64,
    
    /// Private keys for transaction signing
    #[serde(skip_serializing)]
    pub private_keys: HashMap<Address, String>,
    
    /// Default wallet address to use if not specified
    pub default_wallet: Option<Address>,
    
    /// Confirmation blocks required
    pub confirmation_blocks: u64,
    
    /// Maximum pending time before resubmission
    #[serde(with = "duration_serde")]
    pub max_pending_time: Duration,
    
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    
    /// Gas price configuration
    pub gas_price_config: GasPriceConfig,
    
    /// Transaction monitoring configuration
    pub monitoring_config: MonitoringConfig,
    
    /// Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,
}

impl Default for TxExecutionConfig {
    fn default() -> Self {
        Self {
            rpc_endpoints: vec!["http://localhost:8545".to_string()],
            chain_id: 1, // Ethereum mainnet
            private_keys: HashMap::new(),
            default_wallet: None,
            confirmation_blocks: 3,
            max_pending_time: Duration::from_secs(60 * 2), // 2 minutes
            max_retry_attempts: 3,
            gas_price_config: GasPriceConfig::default(),
            monitoring_config: MonitoringConfig::default(),
            circuit_breaker_config: CircuitBreakerConfig::default(),
        }
    }
}

/// Gas price configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPriceConfig {
    /// Use EIP-1559 transactions
    pub use_eip1559: bool,
    
    /// Base fee multiplier for EIP-1559 (max_fee_per_gas = base_fee * base_fee_multiplier)
    pub base_fee_multiplier: f64,
    
    /// Priority fee for EIP-1559 in wei
    pub priority_fee: U256,
    
    /// Priority fee for urgent transactions in wei
    pub urgent_priority_fee: U256,
    
    /// Maximum gas price in wei
    pub max_gas_price: U256,
    
    /// Default gas limit multiplier
    pub gas_limit_multiplier: f64,
}

impl Default for GasPriceConfig {
    fn default() -> Self {
        Self {
            use_eip1559: true,
            base_fee_multiplier: 1.5,
            priority_fee: U256::from(1_500_000_000u64), // 1.5 Gwei
            urgent_priority_fee: U256::from(3_000_000_000u64), // 3 Gwei
            max_gas_price: U256::from(300_000_000_000u64), // 300 Gwei
            gas_limit_multiplier: 1.2,
        }
    }
}

/// Transaction monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Polling interval for transaction status
    #[serde(with = "duration_serde")]
    pub polling_interval: Duration,
    
    /// Maximum time to wait for a transaction
    #[serde(with = "duration_serde")]
    pub max_wait_time: Duration,
    
    /// Check for replacement transactions
    pub check_for_replacements: bool,
    
    /// Check for pending transactions on startup
    pub check_pending_on_startup: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            polling_interval: Duration::from_secs(5),
            max_wait_time: Duration::from_secs(300), // 5 minutes
            check_for_replacements: true,
            check_pending_on_startup: true,
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    
    /// Maximum consecutive failures before circuit breaks
    pub max_consecutive_failures: u32,
    
    /// Time to keep circuit open
    #[serde(with = "duration_serde")]
    pub open_time: Duration,
    
    /// Maximum gas price for circuit breaker
    pub max_gas_price_threshold: U256,
    
    /// Drop transactions if they are in pending state for too long
    pub drop_long_pending: bool,
    
    /// Maximum pending time before considering dropping
    #[serde(with = "duration_serde")]
    pub max_pending_time_before_drop: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_consecutive_failures: 3,
            open_time: Duration::from_secs(300), // 5 minutes
            max_gas_price_threshold: U256::from(250_000_000_000u64), // 250 Gwei
            drop_long_pending: true,
            max_pending_time_before_drop: Duration::from_secs(600), // 10 minutes
        }
    }
}

/// Module for serializing/deserializing Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
} 