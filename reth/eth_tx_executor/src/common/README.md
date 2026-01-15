# Common Utilities 🔧

**Shared types, configurations, and utilities used across eth_kartal modules**

Provides foundational components that ensure consistency and reduce code duplication across the entire codebase.

## 🎯 Purpose

- **Shared Types**: Common data structures and enums
- **Error Handling**: Centralized error definitions
- **Configuration**: System-wide settings management
- **Utilities**: Helper functions and constants

## 📁 Current Status

⚠️ **INCOMPLETE MODULE** - Currently placeholder with minimal implementation

```rust
// mod.rs - Current state (12 lines)
// TODO: Uncomment and implement shared components
// pub mod config;
// pub mod errors;
// pub mod types;
// pub mod utils;
```

## 🔧 Planned Implementation

### Error Types (`errors.rs`)
```rust
#[derive(Debug, thiserror::Error)]
pub enum EthKartalError {
    #[error("RPC connection failed: {0}")]
    RpcError(String),
    
    #[error("Transaction execution failed: {0}")]
    ExecutionError(String),
    
    #[error("Insufficient balance: expected {expected}, found {actual}")]
    InsufficientBalance { expected: U256, actual: U256 },
    
    #[error("Alert processing error: {0}")]
    AlertError(String),
    
    #[error("Pool operation failed: {0}")]
    PoolError(String),
    
    #[error("Risk management violation: {0}")]
    RiskError(String),
}
```

### Configuration (`config.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthKartalConfig {
    // Network configuration
    pub rpc_url: String,
    pub reth_ws_url: String,
    pub chain_id: u64,
    
    // Execution settings
    pub private_key: String,
    pub max_gas_price: U256,
    pub default_slippage: f64,
    
    // Alert processing
    pub zmq_endpoint: String,
    pub alert_timeout_ms: u64,
    
    // Risk management
    pub max_daily_loss: U256,
    pub circuit_breaker_threshold: u32,
    pub max_position_size: U256,
    
    // Performance
    pub mempool_max_txs: usize,
    pub cache_ttl_seconds: u64,
}

impl Default for EthKartalConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            reth_ws_url: "ws://127.0.0.1:8546".to_string(),
            chain_id: 1,
            private_key: std::env::var("PRIVATE_KEY").unwrap_or_default(),
            max_gas_price: U256::from(500_000_000_000u64), // 500 gwei
            default_slippage: 0.01, // 1%
            zmq_endpoint: "tcp://localhost:5559".to_string(),
            alert_timeout_ms: 5000,
            max_daily_loss: U256::from(1_000_000_000_000_000_000u64), // 1 ETH
            circuit_breaker_threshold: 5,
            max_position_size: U256::from(10_000_000_000_000_000_000u64), // 10 ETH
            mempool_max_txs: 50_000,
            cache_ttl_seconds: 60,
        }
    }
}
```

### Shared Types (`types.rs`)
```rust
/// Transaction execution result
pub type ExecutionResult<T> = Result<T, EthKartalError>;

/// Common address type alias
pub type EthAddress = Address;

/// Gas price in wei
pub type GasPrice = U256;

/// Timestamp in seconds
pub type Timestamp = u64;

/// Percentage as decimal (e.g., 0.05 = 5%)
pub type Percentage = f64;

/// Performance timing in milliseconds
pub type Milliseconds = u64;

/// Token balance in smallest units
pub type TokenBalance = U256;

/// Transaction priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    Critical = 3,
    High = 2,
    Normal = 1,
}

/// Network conditions
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkCondition {
    Optimal,     // Low congestion, fast execution
    Normal,      // Standard conditions
    Congested,   // High gas prices, slower execution
    Degraded,    // Network issues, unreliable
}

/// Performance metrics structure
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub gas_efficiency_ratio: f64,
}
```

### Utilities (`utils.rs`)
```rust
/// Format U256 as human readable ETH amount
pub fn format_eth_amount(amount: U256) -> String {
    let eth_amount = amount.as_u128() as f64 / 1e18;
    format!("{:.6} ETH", eth_amount)
}

/// Convert percentage to basis points
pub fn percentage_to_bps(percentage: f64) -> u32 {
    (percentage * 10_000.0) as u32
}

/// Calculate percentage change
pub fn percentage_change(old: U256, new: U256) -> f64 {
    if old.is_zero() {
        return 0.0;
    }
    let old_f = old.as_u128() as f64;
    let new_f = new.as_u128() as f64;
    ((new_f - old_f) / old_f) * 100.0
}

/// Validate Ethereum address
pub fn is_valid_address(address: &str) -> bool {
    address.len() == 42 && 
    address.starts_with("0x") && 
    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Create timeout future
pub fn create_timeout<T>(duration: Duration) -> impl Future<Output = Option<T>> {
    async move {
        tokio::time::sleep(duration).await;
        None
    }
}

/// Retry logic with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    max_retries: u32,
    initial_delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Debug,
{
    let mut delay = initial_delay;
    
    for attempt in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt == max_retries - 1 {
                    return Err(e);
                }
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    }
    
    unreachable!()
}
```

## 🎛️ Constants (`constants.rs`)
```rust
// Network constants
pub const MAINNET_CHAIN_ID: u64 = 1;
pub const GOERLI_CHAIN_ID: u64 = 5;
pub const SEPOLIA_CHAIN_ID: u64 = 11155111;

// Gas constants
pub const MIN_GAS_PRICE: u64 = 1_000_000_000; // 1 gwei
pub const MAX_GAS_PRICE: u64 = 500_000_000_000; // 500 gwei
pub const STANDARD_GAS_LIMIT: u64 = 200_000;

// Timing constants
pub const MAX_EXECUTION_TIME_MS: u64 = 200;
pub const CACHE_TTL_SECONDS: u64 = 60;
pub const HEARTBEAT_INTERVAL_MS: u64 = 30_000;

// Uniswap constants
pub const UNISWAP_V2_FEE_BPS: u32 = 30; // 0.3%
pub const MAX_SLIPPAGE_BPS: u32 = 1000; // 10%

// Risk management
pub const MAX_DAILY_TRADES: u32 = 1000;
pub const EMERGENCY_STOP_LOSS_BPS: u32 = 2000; // 20%
```

## 🔧 Integration Points

### Module Usage Pattern
```rust
use crate::common::{
    EthKartalError, EthKartalConfig, ExecutionResult,
    Priority, NetworkCondition, PerformanceMetrics,
    format_eth_amount, retry_with_backoff
};

// Error handling
fn process_alert() -> ExecutionResult<()> {
    // Implementation with consistent error types
}

// Configuration access
let config = EthKartalConfig::from_env()?;

// Utility usage
let formatted = format_eth_amount(balance);
```

## 🧪 Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_eth_amount() {
        let amount = U256::from(1_000_000_000_000_000_000u64); // 1 ETH
        assert_eq!(format_eth_amount(amount), "1.000000 ETH");
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff() {
        // Test retry logic with mock failures
    }
}
```

## 📊 Configuration Management

### Environment Variable Loading
```rust
impl EthKartalConfig {
    pub fn from_env() -> ExecutionResult<Self> {
        let mut config = Self::default();
        
        if let Ok(rpc_url) = std::env::var("RPC_URL") {
            config.rpc_url = rpc_url;
        }
        
        if let Ok(private_key) = std::env::var("PRIVATE_KEY") {
            config.private_key = private_key;
        }
        
        // Validate required fields
        if config.private_key.is_empty() {
            return Err(EthKartalError::ConfigError(
                "PRIVATE_KEY environment variable required".to_string()
            ));
        }
        
        Ok(config)
    }
    
    pub fn from_file(path: &str) -> ExecutionResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
```

## 🔄 Implementation Priority

### High Priority (Immediate Need)
1. **Error Types**: Standardize error handling across modules
2. **Configuration**: Centralized settings management
3. **Basic Utilities**: Address validation, formatting

### Medium Priority
1. **Advanced Utilities**: Retry logic, timeout handling
2. **Performance Metrics**: Standardized measurement
3. **Network Condition Detection**: Dynamic behavior adjustment

### Low Priority
1. **Advanced Configuration**: Hot reloading, validation
2. **Telemetry Integration**: Metrics collection
3. **Development Utilities**: Testing helpers

## 🚧 Current Blockers

The common module is currently a placeholder, which creates:

1. **Inconsistent Error Handling**: Each module defines its own error types
2. **Configuration Duplication**: Settings scattered across modules
3. **Code Duplication**: Utility functions repeated

## 🎯 Implementation Plan

### Step 1: Core Error Types
```bash
# Create and implement error.rs
touch src/common/errors.rs
# Define EthKartalError with all variants
```

### Step 2: Configuration System
```bash
# Create config.rs with environment loading
touch src/common/config.rs
# Implement EthKartalConfig with defaults
```

### Step 3: Essential Utilities
```bash
# Create utils.rs with basic helpers
touch src/common/utils.rs
# Implement formatting and validation functions
```

### Step 4: Module Integration
```bash
# Update each module to use common types
# Replace local error types with EthKartalError
# Use centralized configuration
```

## 📚 Dependencies

```toml
[dependencies]
# Serialization
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Error handling
thiserror = "1.0"

# Async utilities
tokio = { version = "1.0", features = ["time"] }

# Ethereum types
ethers = "2.0"
```

---

**Priority**: HIGH - This module is essential for code organization and maintainability across the entire codebase.