//! System-wide constants and configuration values
//! 
//! Centralizes all magic numbers, addresses, and configuration constants
//! used throughout the eth_kartal system.

use ethers::prelude::*;
use lazy_static::lazy_static;

/// Network addresses for Ethereum mainnet
pub mod addresses {
    use super::*;
    
    lazy_static! {
        /// WETH (Wrapped Ether) address on mainnet
        pub static ref WETH: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap();
        
        /// USDC address on mainnet
        pub static ref USDC: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap();
        
        /// USDT address on mainnet
        pub static ref USDT: Address = "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().unwrap();
        
        /// DAI address on mainnet
        pub static ref DAI: Address = "0x6B175474E89094C44Da98b954EedeAC495271d0F".parse().unwrap();
        
        /// Uniswap V2 Factory
        pub static ref UNISWAP_V2_FACTORY: Address = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse().unwrap();
        
        /// Uniswap V2 Router
        pub static ref UNISWAP_V2_ROUTER: Address = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap();
        
        /// Uniswap V3 Factory
        pub static ref UNISWAP_V3_FACTORY: Address = "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse().unwrap();
        
        /// Uniswap V3 Router
        pub static ref UNISWAP_V3_ROUTER: Address = "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse().unwrap();
    }
}

/// System limits and thresholds
pub mod limits {
    use ethers::types::U256;
    
    /// Maximum gas price (1000 gwei)
    pub const MAX_GAS_PRICE_GWEI: u64 = 1000;
    
    /// Minimum ETH balance to maintain (0.01 ETH)
    pub const MIN_ETH_BALANCE: &str = "0.01";
    
    /// Maximum position size per token (in USD)
    pub const MAX_POSITION_USD: u64 = 100_000;
    
    /// Maximum daily loss (in USD)
    pub const MAX_DAILY_LOSS_USD: u64 = 10_000;
    
    /// Maximum slippage allowed (5%)
    pub const MAX_SLIPPAGE: f64 = 0.05;
    
    /// Minimum profit threshold (0.1%)
    pub const MIN_PROFIT_THRESHOLD: f64 = 0.001;
    
    /// Convert gwei to wei
    pub fn max_gas_price_wei() -> U256 {
        U256::from(MAX_GAS_PRICE_GWEI) * U256::from(10).pow(U256::from(9))
    }
}

/// Timing constants
pub mod timing {
    use std::time::Duration;
    
    /// Maximum transaction execution time
    pub const MAX_EXECUTION_TIME_MS: u64 = 200;
    
    /// Alert expiry time
    pub const ALERT_EXPIRY_SECONDS: u64 = 60;
    
    /// Nonce refresh interval
    pub const NONCE_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
    
    /// Position cache TTL
    pub const POSITION_CACHE_TTL: Duration = Duration::from_secs(5);
    
    /// Gas price cache TTL
    pub const GAS_PRICE_CACHE_TTL: Duration = Duration::from_millis(500);
    
    /// Mempool scan interval
    pub const MEMPOOL_SCAN_INTERVAL: Duration = Duration::from_millis(100);
    
    /// Circuit breaker cooldown
    pub const CIRCUIT_BREAKER_COOLDOWN: Duration = Duration::from_secs(300);
}

/// Network configuration
pub mod network {
    /// Default HTTP request timeout
    pub const HTTP_TIMEOUT_SECONDS: u64 = 10;
    
    /// Maximum retry attempts
    pub const MAX_RETRIES: u32 = 3;
    
    /// Retry delay base (exponential backoff)
    pub const RETRY_DELAY_MS: u64 = 100;
    
    /// WebSocket ping interval
    pub const WS_PING_INTERVAL_SECONDS: u64 = 30;
    
    /// Maximum concurrent requests
    pub const MAX_CONCURRENT_REQUESTS: usize = 10;
}

/// ZMQ configuration
pub mod zmq {
    /// Alert receiver bind address
    pub const ALERT_RECEIVER_BIND: &str = "tcp://127.0.0.1:5559";
    
    /// High water mark for ZMQ socket
    pub const HIGH_WATER_MARK: i32 = 1000;
    
    /// Receive timeout
    pub const RECEIVE_TIMEOUT_MS: i32 = 100;
}

/// Flashbots configuration
pub mod flashbots {
    /// Flashbots relay URL
    pub const RELAY_URL: &str = "https://relay.flashbots.net";
    
    /// Bundle submission timeout
    pub const BUNDLE_TIMEOUT_SECONDS: u64 = 5;
    
    /// Maximum bundle size
    pub const MAX_BUNDLE_SIZE: usize = 10;
    
    /// Default tip percentage
    pub const DEFAULT_TIP_PERCENTAGE: f64 = 0.01;
}

/// Risk management thresholds
pub mod risk {
    /// Maximum consecutive failures before circuit break
    pub const MAX_CONSECUTIVE_FAILURES: u32 = 5;
    
    /// Failure rate threshold (80%)
    pub const FAILURE_RATE_THRESHOLD: f64 = 0.8;
    
    /// Minimum success rate required
    pub const MIN_SUCCESS_RATE: f64 = 0.7;
    
    /// Maximum gas cost as percentage of expected profit
    pub const MAX_GAS_COST_RATIO: f64 = 0.5;
}

/// Performance targets
pub mod performance {
    /// Target latencies for each operation (milliseconds)
    pub mod targets {
        pub const ALERT_PROCESSING: u64 = 5;
        pub const POSITION_CHECK: u64 = 10;
        pub const GAS_RANKING: u64 = 20;
        pub const PRICE_QUOTE: u64 = 30;
        pub const TX_BUILD: u64 = 5;
        pub const TX_SUBMIT: u64 = 50;
        pub const TOTAL: u64 = 200;
    }
}

/// Environment detection
pub fn is_production() -> bool {
    std::env::var("ETH_KARTAL_ENV").unwrap_or_default() == "production"
}

pub fn is_development() -> bool {
    !is_production()
}

/// Get configuration path based on environment
pub fn config_path() -> &'static str {
    if is_production() {
        "config/prod.toml"
    } else {
        "config/dev.toml"
    }
}