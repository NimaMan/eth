# Configuration Guide

## Overview
ETH Kartal uses TOML configuration files for different environments. Configuration can be overridden via environment variables using the `ETH_KARTAL_` prefix.

## Configuration Files

### `dev.toml` - Development Environment
- Local testing with testnet
- Verbose logging enabled
- Relaxed safety limits
- Mock services enabled

### `prod.toml` - Production Environment
- Mainnet configuration
- Strict safety controls
- Performance optimized
- Real money protection

## Configuration Sections

### System Configuration
```toml
[system]
environment = "production"          # development, staging, production
log_level = "info"                 # trace, debug, info, warn, error
log_file = "/var/log/kartal/app.log"
metrics_port = 9090
healthcheck_port = 8080
```

### Alert Processing
```toml
[alerts]
zmq_endpoint = "tcp://localhost:5558"
queue_size = 10000
processing_threads = 4
reconnect_interval_secs = 5
```

### Wallet Configuration
```toml
[wallet]
type = "local"                     # local, ledger, trezor
private_key_env = "ETH_KARTAL_PRIVATE_KEY"
# private_key_file = "/secure/path/to/keystore"
# hardware_derivation = "m/44'/60'/0'/0/0"
max_gas_price_gwei = 500
```

### Strategy Settings
```toml
[strategies]
enable_emergency_sell = true
enable_partial_exit = true
enable_arbitrage = false
enable_liquidity_provision = false

[strategies.thresholds]
emergency_sell_loss_percent = 80   # Trigger emergency at 80% loss
partial_exit_loss_percent = 50     # Trigger partial at 50% loss
min_pool_size_eth = 1.0           # Ignore pools smaller than 1 ETH
min_profit_eth = 0.05             # Minimum profit to execute

[strategies.slippage]
emergency_max_slippage = 0.30      # 30% max slippage for emergency
high_priority_slippage = 0.15      # 15% for high priority
normal_slippage = 0.05             # 5% for normal trades
```

### Risk Management
```toml
[risk]
max_position_eth = 10.0
max_daily_loss_eth = 5.0
max_consecutive_failures = 5
circuit_breaker_enabled = true

[risk.position_limits]
max_percent_of_pool = 0.05         # Max 5% of any pool
concentration_limit = 0.25         # Max 25% in one token
max_tokens_held = 20

[risk.circuit_breaker]
cooldown_seconds = 300             # 5 minute cooldown
emergency_shutdown_loss_eth = 10.0
```

### Transaction Execution
```toml
[execution]
simulation_required = true
max_gas_limit = 3000000
gas_price_multiplier = 1.2        # 20% above base
use_flashbots = true
use_mev_protection = true

[execution.rpcs]
primary = "http://localhost:8545"
fallback = [
    "https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY",
    "https://mainnet.infura.io/v3/YOUR_KEY"
]
flashbots_relay = "https://relay.flashbots.net"
```

### DEX Router Addresses
```toml
[routers]
uniswap_v2 = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
uniswap_v3 = "0xE592427A0AEce92De3Edee1F18E0157C05861564"
sushiswap = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F"
```

### Monitoring and Alerts
```toml
[monitoring]
enable_metrics = true
enable_alerts = true
alert_webhook = "https://discord.com/api/webhooks/..."
performance_log_interval_secs = 60

[monitoring.alerts]
critical_loss_eth = 1.0           # Alert on 1 ETH loss
high_slippage_percent = 10        # Alert on >10% slippage
low_success_rate = 0.8            # Alert if <80% success
```

## Environment Variable Overrides

Any configuration value can be overridden using environment variables:

```bash
# Override log level
export ETH_KARTAL_SYSTEM_LOG_LEVEL=debug

# Override max position
export ETH_KARTAL_RISK_MAX_POSITION_ETH=20.0

# Set private key (never commit!)
export ETH_KARTAL_PRIVATE_KEY=0x...

# Override RPC endpoint
export ETH_KARTAL_EXECUTION_RPCS_PRIMARY=http://my-node:8545
```

## Configuration Validation

The system validates configuration on startup:
1. Required fields are present
2. Addresses are valid checksummed format
3. Numeric values are within reasonable ranges
4. RPC endpoints are reachable
5. Wallet can sign transactions

## Security Considerations

### Sensitive Values
Never commit these to version control:
- Private keys
- API keys
- Webhook URLs
- RPC endpoints with keys

### Best Practices
1. Use environment variables for secrets
2. Encrypt configuration files at rest
3. Limit file permissions (600)
4. Rotate keys regularly
5. Use separate wallets per environment

## Example Usage

```rust
use eth_kartal::config::Config;

// Load from file
let config = Config::from_file("config/prod.toml")?;

// Load with env overrides
let config = Config::from_file_with_env("config/prod.toml")?;

// Access values
let max_gas = config.wallet.max_gas_price_gwei;
let threshold = config.strategies.thresholds.emergency_sell_loss_percent;
```

## Debugging Configuration

Enable config debugging:
```bash
export ETH_KARTAL_DEBUG_CONFIG=true
cargo run -- --print-config
```

This will:
1. Show loaded configuration
2. Highlight environment overrides
3. Validate all settings
4. Test external connections