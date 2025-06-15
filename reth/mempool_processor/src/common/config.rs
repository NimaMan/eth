// Centralized configuration management for the mempool processor

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Global configuration for the mempool processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolProcessorConfig {
    /// Scam detection configuration
    pub scam_detection: ScamDetectionConfig,
    
    /// Pool state configuration
    pub pool_state: PoolStateConfig,
    
    /// Transaction processing configuration
    pub tx_processing: TxProcessingConfig,
    
    /// Performance configuration
    pub performance: PerformanceConfig,
}

/// Configuration for scam detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScamDetectionConfig {
    /// Base ETH threshold for scam detection (in ETH)
    pub eth_threshold: f64,
    
    /// Base percentage threshold for large withdrawals (0.0-1.0)
    pub percentage_threshold: f64,
    
    /// Minimum pool liquidity to consider for scam detection (in ETH)
    pub min_pool_liquidity: f64,
    
    /// Dynamic threshold settings
    pub dynamic_thresholds: DynamicThresholdConfig,
}

/// Dynamic threshold configuration based on pool size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicThresholdConfig {
    /// Threshold for very small pools (< 1 ETH)
    pub very_small_pool_percentage: f64,
    
    /// Threshold for small pools (1-5 ETH)
    pub small_pool_percentage: f64,
    
    /// ETH amount below which pools are considered "small"
    pub small_pool_threshold: f64,
    
    /// ETH amount below which pools are considered "very small"
    pub very_small_pool_threshold: f64,
}

/// Configuration for pool state management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStateConfig {
    /// Maximum age for pool state before considering it stale
    pub max_state_age: Duration,
    
    /// How often to refresh pool state from source
    pub refresh_interval: Duration,
    
    /// Whether to warn on stale pool state
    pub warn_on_stale: bool,
}

/// Configuration for transaction processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxProcessingConfig {
    /// Maximum number of transactions to process in a batch
    pub max_batch_size: usize,
    
    /// Transaction cache capacity
    pub tx_cache_capacity: usize,
    
    /// Maximum age for cached transactions
    pub tx_cache_max_age: Duration,
    
    /// Whether to process all transactions or only pool transactions
    pub process_all_transactions: bool,
    
    /// Timeout for RPC requests
    pub rpc_timeout: Duration,
    
    /// List of token contract addresses to watch (in checksum format)
    pub watched_tokens: Vec<String>,
}

/// Performance and monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// How often to report metrics
    pub metrics_interval: Duration,
    
    /// Number of transactions for warmup period
    pub warmup_transactions: u64,
    
    /// Target SLA for end-to-end processing time
    pub sla_target: Duration,
    
    /// Polling interval for transaction fetching
    pub poll_interval: Duration,
}

impl Default for MempoolProcessorConfig {
    fn default() -> Self {
        Self {
            scam_detection: ScamDetectionConfig {
                eth_threshold: 0.15,
                percentage_threshold: 0.5,
                min_pool_liquidity: 0.5,
                dynamic_thresholds: DynamicThresholdConfig {
                    very_small_pool_percentage: 0.8,
                    small_pool_percentage: 0.7,
                    small_pool_threshold: 5.0,
                    very_small_pool_threshold: 1.0,
                },
            },
            pool_state: PoolStateConfig {
                max_state_age: Duration::from_secs(60),
                refresh_interval: Duration::from_secs(30),
                warn_on_stale: true,
            },
            tx_processing: TxProcessingConfig {
                max_batch_size: 1000,
                tx_cache_capacity: 50000,
                tx_cache_max_age: Duration::from_secs(60),
                process_all_transactions: true,
                rpc_timeout: Duration::from_millis(2000),
                watched_tokens: vec![],
            },
            performance: PerformanceConfig {
                metrics_interval: Duration::from_secs(60),
                warmup_transactions: 100000,
                sla_target: Duration::from_millis(50),
                poll_interval: Duration::from_millis(100),
            },
        }
    }
}

impl MempoolProcessorConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Override with environment variables if set
        if let Ok(val) = std::env::var("ETH_THRESHOLD") {
            if let Ok(threshold) = val.parse::<f64>() {
                config.scam_detection.eth_threshold = threshold;
            }
        }
        
        if let Ok(val) = std::env::var("PERCENTAGE_THRESHOLD") {
            if let Ok(threshold) = val.parse::<f64>() {
                config.scam_detection.percentage_threshold = threshold;
            }
        }
        
        if let Ok(val) = std::env::var("MAX_BATCH_SIZE") {
            if let Ok(size) = val.parse::<usize>() {
                config.tx_processing.max_batch_size = size;
            }
        }
        
        if let Ok(val) = std::env::var("TX_CACHE_CAPACITY") {
            if let Ok(capacity) = val.parse::<usize>() {
                config.tx_processing.tx_cache_capacity = capacity;
            }
        }
        
        if let Ok(val) = std::env::var("WATCHED_TOKENS") {
            // Parse comma-separated list of token addresses
            let tokens: Vec<String> = val.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !tokens.is_empty() {
                config.tx_processing.watched_tokens = tokens;
            }
        }
        
        config
    }
    
    // TODO: Add TOML support by adding `toml` to Cargo.toml dependencies
    // /// Load configuration from a TOML file
    // pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
    //     let contents = std::fs::read_to_string(path)?;
    //     let config: Self = toml::from_str(&contents)?;
    //     Ok(config)
    // }
    
    // /// Save configuration to a TOML file
    // pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    //     let contents = toml::to_string_pretty(self)?;
    //     std::fs::write(path, contents)?;
    //     Ok(())
    // }
}