# Configuration Module

Centralized configuration management for eth_kartal with support for environment variables, TOML files, and runtime validation.

## Purpose

The config module provides:
- **Unified configuration** across all system components
- **Environment variable integration** for deployment flexibility
- **Configuration validation** to catch errors early
- **Hot-reload capabilities** for dynamic configuration updates

## Components

### `Config` Struct
Central configuration container with all system settings:

```rust
pub struct Config {
    pub ethereum: EthereumConfig,
    pub risk: RiskConfig,
    pub execution: ExecutionConfig,
    pub logging: LoggingConfig,
    pub alerts: AlertConfig,
}
```

### Configuration Categories

#### **Ethereum Configuration**
```rust
pub struct EthereumConfig {
    pub rpc_url: String,           // Ethereum RPC endpoint
    pub chain_id: u64,             // Network chain ID (1 = mainnet)
    pub keystore_path: PathBuf,    // Path to encrypted keystore
    pub auto_lock_timeout: Option<Duration>, // Wallet auto-lock timeout
}
```

#### **Risk Management Configuration**
```rust
pub struct RiskConfig {
    pub max_position_usd: f64,      // Maximum position size in USD
    pub max_daily_loss_usd: f64,    // Daily loss limit in USD
    pub max_slippage_percent: f64,  // Maximum allowed slippage
    pub emergency_stop: bool,       // Emergency trading halt
    pub blacklisted_tokens: Vec<Address>, // Blocked token addresses
}
```

#### **Execution Configuration**
```rust
pub struct ExecutionConfig {
    pub flashbots_enabled: bool,        // Enable MEV protection
    pub max_gas_price_gwei: u64,        // Gas price ceiling
    pub target_block_position: u64,     // Target position in block
    pub mempool_timeout_ms: u64,        // Mempool analysis timeout
    pub simulation_enabled: bool,       // Pre-execution simulation
}
```

## Usage Examples

### Loading Configuration

**From Environment Variables:**
```rust
use eth_kartal::config::Config;

// Load with defaults, override from environment
let config = Config::from_env()?;
```

**From TOML File:**
```rust
// Load from config.toml
let config = Config::from_file("config.toml")?;
```

**Mixed Sources:**
```rust
// Load from file, then override with environment
let mut config = Config::from_file("config.toml")?;
config.merge_env()?;
```

### Configuration File Format

**config.toml:**
```toml
[ethereum]
rpc_url = "http://127.0.0.1:8545"
chain_id = 1
keystore_path = "/path/to/keystore.json"
auto_lock_timeout_secs = 300

[risk]
max_position_usd = 10000.0
max_daily_loss_usd = 1000.0
max_slippage_percent = 10.0
emergency_stop = false

[execution]
flashbots_enabled = true
max_gas_price_gwei = 100
target_block_position = 3
mempool_timeout_ms = 5000
simulation_enabled = true

[logging]
level = "info"
file_path = "/var/log/eth_kartal.log"
database_enabled = true

[alerts]
endpoint = "tcp://127.0.0.1:5555"
timeout_ms = 1000
max_concurrent = 100
```

### Environment Variable Mapping

Configuration values can be overridden with environment variables:

```bash
# Ethereum settings
export ETH_RPC_URL="http://127.0.0.1:8545"
export ETH_CHAIN_ID="1"  
export ETH_KEYSTORE_PATH="/path/to/keystore.json"

# Risk management
export RISK_MAX_POSITION_USD="10000.0"
export RISK_MAX_DAILY_LOSS_USD="1000.0"
export RISK_EMERGENCY_STOP="false"

# Execution settings
export EXEC_FLASHBOTS_ENABLED="true"
export EXEC_MAX_GAS_PRICE_GWEI="100"

# Logging
export LOG_LEVEL="info"
export DATABASE_URL="postgres://user:pass@localhost/eth_db"

# Alerts
export ALERT_ENDPOINT="tcp://127.0.0.1:5555"
```

## Configuration Validation

The config module performs comprehensive validation:

### **Network Validation**
```rust
// Validates RPC connectivity
config.ethereum.validate().await?;
```

### **Path Validation**
```rust
// Ensures keystore file exists and is readable
if !config.ethereum.keystore_path.exists() {
    return Err(ConfigError::FileNotFound { 
        path: config.ethereum.keystore_path.display().to_string() 
    });
}
```

### **Range Validation**
```rust
// Validates numeric ranges
if config.risk.max_slippage_percent > 100.0 {
    return Err(ConfigError::InvalidValue {
        field: "max_slippage_percent".to_string(),
        reason: "Cannot exceed 100%".to_string(),
    });
}
```

## Error Handling

Configuration errors are strongly typed:

```rust
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Parse error: {0}")]
    ParseError(String),
}
```

## Runtime Configuration Updates

Support for dynamic configuration changes:

```rust
// Watch for configuration file changes
let config_watcher = config.watch_for_changes("config.toml").await?;

// Handle configuration updates
while let Some(new_config) = config_watcher.recv().await {
    // Apply new configuration
    executor.update_config(new_config).await?;
}
```

## Best Practices

### **Secure Configuration**
```rust
// Use environment variables for sensitive data
export ETH_KEYSTORE_PATH="/secure/path/keystore.json"
export DATABASE_URL="postgres://user:$(cat /secrets/db_pass)@localhost/db"
```

### **Environment-Specific Configs**
```bash
# Development
cp config.dev.toml config.toml

# Production  
cp config.prod.toml config.toml
```

### **Configuration Validation in CI/CD**
```bash
# Validate configuration before deployment
cargo run --bin config-validator -- --config config.prod.toml
```

## Integration with Other Modules

### **Risk Manager Integration**
```rust
let risk_manager = RiskManager::new(config.risk.clone());
```

### **Executor Integration**
```rust
let executor = TransactionExecutor::new(ExecutorConfig {
    keystore_path: config.ethereum.keystore_path.clone(),
    chain_id: config.ethereum.chain_id,
    rpc_url: config.ethereum.rpc_url.clone(),
    flashbots_enabled: config.execution.flashbots_enabled,
    risk_config: config.risk.clone(),
}).await?;
```

### **Alert Receiver Integration**
```rust
let receiver = AlertReceiver::new(ReceiverConfig {
    endpoint: config.alerts.endpoint.clone(),
    timeout: Duration::from_millis(config.alerts.timeout_ms),
    max_concurrent: config.alerts.max_concurrent,
}, alert_tx);
```

## Testing Configuration

### **Unit Tests**
```rust
#[tokio::test]
async fn test_config_validation() {
    let mut config = Config::default();
    config.risk.max_slippage_percent = 150.0; // Invalid
    
    assert!(config.validate().is_err());
}
```

### **Integration Tests**
```rust
#[tokio::test]
async fn test_config_file_loading() {
    let config = Config::from_file("tests/fixtures/valid_config.toml").unwrap();
    assert_eq!(config.ethereum.chain_id, 1);
}
```

## Performance Considerations

- **Lazy loading** of configuration sections
- **Caching** of validated configuration values
- **Minimal parsing overhead** with serde optimizations
- **Fast environment variable lookups** with lazy_static

## Security Considerations

- **No secrets in config files** - use environment variables
- **File permission validation** for keystore paths
- **Configuration sanitization** to prevent injection attacks
- **Audit logging** of configuration changes