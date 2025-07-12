/// Configuration module for tx_processor
/// 
/// Handles environment variables and configuration management

use std::path::PathBuf;
use eyre::Result;

/// Configuration for the tx_processor
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to Reth data directory
    pub reth_datadir: PathBuf,
    /// Maximum batch size for parallel processing
    pub max_batch_size: usize,
}

impl Config {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            reth_datadir: PathBuf::from(
                std::env::var("RETH_DATADIR")
                    .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string())
            ),
            max_batch_size: std::env::var("MAX_BATCH_SIZE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
        })
    }
    
    /// Create configuration with explicit values
    pub fn new(reth_datadir: impl Into<PathBuf>) -> Self {
        Self {
            reth_datadir: reth_datadir.into(),
            max_batch_size: 100,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env().unwrap_or_else(|_| Self {
            reth_datadir: PathBuf::from("/home/nima/.local/share/reth/mainnet"),
            max_batch_size: 100,
        })
    }
}

/// Get the default configuration
pub fn get_config() -> Config {
    Config::default()
}
