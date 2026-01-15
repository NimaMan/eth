# ETH Kartal - High-Performance Transaction Execution Engine

Ultra-fast transaction execution system for Ethereum mainnet designed for sub-200ms alert-to-execution latency in automated trading and MEV protection scenarios.

## 🎯 Purpose

ETH Kartal executes trading signals with minimal latency by:
- Processing incoming alerts from mempool monitoring systems
- Applying risk management and position limits
- Executing trades through optimal gas pricing and MEV protection
- Providing comprehensive audit trails and performance metrics

## 📊 Signal Format

### Input Signal Structure
```json
{
  "id": "alert_12345",
  "action": "BUY" | "SELL",
  "token_address": "0x...",
  "pool_address": "0x...",
  "timestamp": 1699123456,
  "params": {
    "amount": "1000000000000000000",  // Amount in wei or U256::MAX for "all"
    "slippage": 0.03,                 // 3% slippage (0.001-0.10 range)
    "priority": "CRITICAL" | "HIGH" | "MEDIUM" | "LOW",
    "deadline_seconds": 60,
    "mev_protected": true
  }
}
```

### Example Trading Signals

**Buy Signal (ETH → Token)**
```json
{
  "id": "buy_signal_001",
  "action": "BUY", 
  "token_address": "0xA0b86a33E6417aFb8D3c5C61308B6fCFa36C6a00b",
  "pool_address": "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
  "timestamp": 1699123456,
  "params": {
    "amount": "500000000000000000",  // 0.5 ETH
    "slippage": 0.025,               // 2.5%
    "priority": "HIGH",
    "deadline_seconds": 30,
    "mev_protected": true
  }
}
```

**Sell Signal (Token → ETH)**
```json
{
  "id": "sell_signal_002", 
  "action": "SELL",
  "token_address": "0xA0b86a33E6417aFb8D3c5C61308B6fCFa36C6a00b",
  "pool_address": "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
  "timestamp": 1699123457,
  "params": {
    "amount": "18446744073709551615", // U256::MAX = sell all tokens
    "slippage": 0.05,                 // 5%
    "priority": "CRITICAL",
    "deadline_seconds": 15,
    "mev_protected": true
  }
}
```

## ⚡ Execution Flow

```
Alert Received → Input Validation → Risk Assessment → Pool Query → Gas Optimization → Transaction Build → MEV Protection → Execution → Logging
     ~1ms           ~2ms              ~5ms            ~15ms         ~10ms              ~8ms           ~50ms         ~100ms      ~5ms
```

### Detailed Execution Steps

1. **Alert Processing** (`~1ms`)
   - ZMQ message received and parsed
   - Signal ID generated for tracking
   - Alert logged to database

2. **Input Validation** (`~2ms`)
   - Token/pool address validation
   - Slippage bounds checking (0.1% - 10%)
   - Amount and deadline validation

3. **Risk Assessment** (`~5ms`)
   - Position limit checks
   - Daily loss limit validation
   - Token blacklist verification
   - Dynamic risk scoring

4. **Pool Interaction** (`~15ms`)
   - Pool reserves fetched
   - Amount out calculated
   - Slippage applied to minimum output

5. **Gas Optimization** (`~10ms`)
   - Mempool analysis for optimal gas price
   - Position prediction for target block
   - Execution path selection (Public/Flashbots/Multi)

6. **Transaction Building** (`~8ms`)
   - Swap parameters constructed
   - Transaction built with optimal gas
   - Nonce reserved from manager

7. **MEV Protection** (`~50ms`)
   - Flashbots bundle creation (if enabled)
   - Bundle simulation and submission
   - Fallback to public mempool if needed

8. **Execution** (`~100ms`)
   - Transaction signed with secure wallet
   - Submitted via optimal execution path
   - Nonce tracking updated

9. **Result Logging** (`~5ms`)
   - Execution outcome recorded
   - Performance metrics logged
   - Database updated with final status

### Success Path
```
✅ Alert → ✅ Validation → ✅ Risk OK → ✅ Pool Found → ✅ Gas Optimal → ✅ TX Built → ✅ Submitted → ✅ Confirmed
```

### Failure Paths
```
❌ Invalid Token → Validation Error → Early Exit
❌ Risk Block   → Risk Rejection → Early Exit  
❌ No Liquidity → Pool Error → Early Exit
❌ High Gas     → Gas Failure → Retry/Exit
❌ TX Revert    → Execution Failure → Logged
```

## 🔧 Configuration

### Environment Variables
```bash
# Required
ETH_KEYSTORE_PATH="/path/to/keystore.json"
ETH_RPC_URL="http://127.0.0.1:8545"

# Optional
DATABASE_URL="postgres://user:pass@localhost/eth_db"
ETH_CHAIN_ID="1"
ALERT_ENDPOINT="tcp://127.0.0.1:5555"
FLASHBOTS_ENABLED="true"
```

### Risk Configuration
```toml
[risk]
max_position_usd = 10000.0
max_daily_loss_usd = 1000.0
max_slippage_percent = 10.0
emergency_stop = false

[gas]
max_gas_price_gwei = 100
target_block_position = 3
mempool_timeout_ms = 5000
```

## 🚀 Usage

### Command Line Interface
```bash
# Production trading
./kartal --keystore-path /path/to/keystore.json --rpc-url http://127.0.0.1:8545

# Test mode (simulation only)
./kartal --test-mode --keystore-path /path/to/keystore.json

# With custom alert endpoint
./kartal --alert-endpoint tcp://127.0.0.1:5556 --keystore-path /path/to/keystore.json
```

### Programmatic Usage
```rust
use eth_kartal::{TransactionExecutor, ExecutorConfig, Alert};

let config = ExecutorConfig {
    keystore_path: "/path/to/keystore.json".into(),
    chain_id: 1,
    rpc_url: "http://127.0.0.1:8545".to_string(),
    flashbots_enabled: true,
    // ...
};

let executor = TransactionExecutor::new(config).await?;
let result = executor.execute_alert(alert).await;
```

## 📈 Performance Metrics

### Latency Targets
- **Total execution**: < 200ms (95th percentile)
- **Alert to start**: < 5ms
- **Risk assessment**: < 10ms
- **Gas optimization**: < 15ms
- **Transaction build**: < 10ms
- **Network submission**: < 150ms

### Throughput Capacity
- **Theoretical**: 187,611 TPS (based on internal processing)
- **Network limited**: ~50 TPS (Ethereum block gas limit)
- **Practical**: 10-20 TPS (considering gas competition)

## 🛡️ Security Features

### Wallet Security
- **Encrypted keystores** with secure password handling
- **Auto-lock** functionality after 5 minutes of inactivity
- **Memory protection** with secure key clearing
- **Access control** with unlock verification

### Risk Management
- **Position limits** to prevent overexposure
- **Daily loss limits** with automatic halt
- **Token blacklisting** for known scam tokens
- **Circuit breakers** for unusual market conditions

### MEV Protection
- **Flashbots integration** for private mempool submission
- **Bundle protection** against front-running
- **Dynamic execution paths** based on market conditions
- **Sandwich attack prevention**

## 📋 Module Documentation

Each module contains detailed documentation:

- [`alert_processor/`](src/alert_processor/README.md) - Signal ingestion and parsing
- [`common/`](src/common/README.md) - Shared types and utilities
- [`config/`](src/config/README.md) - Configuration management
- [`flashbots/`](src/flashbots/README.md) - MEV protection and private pools
- [`logging/`](src/logging/README.md) - Trade logging and audit trails
- [`pools/`](src/pools/README.md) - DEX pool abstractions
- [`ranking/`](src/ranking/README.md) - Gas optimization and mempool analysis
- [`risk/`](src/risk/README.md) - Risk management and circuit breakers
- [`tx_executor/`](src/tx_executor/README.md) - Transaction execution engine
- [`wallet/`](src/wallet/README.md) - Secure wallet management

## 🔍 Monitoring & Debugging

### Logs Output
```
2024-01-15T10:30:15Z INFO  📨 Alert received: buy_signal_001 | Token: 0xA0b8... | Action: BUY | Amount: 500000000000000000 | Priority: HIGH
2024-01-15T10:30:15Z INFO  ⚖️ Risk decision: ALLOW | Alert: buy_signal_001 | Original: 500000000000000000 | Final: 500000000000000000
2024-01-15T10:30:15Z INFO  📤 Transaction submitted: 0x1a2b... | Alert: buy_signal_001 | Nonce: 42 | Gas: 25000000000 | Path: FlashbotsBundle
2024-01-15T10:30:15Z INFO  ✅ Execution successful: 0x1a2b... | Alert: buy_signal_001 | Latency: 187ms
2024-01-15T10:30:15Z INFO    📊 Performance breakdown:
2024-01-15T10:30:15Z INFO      • Alert→Start: 1ms
2024-01-15T10:30:15Z INFO      • Position check: 12ms
2024-01-15T10:30:15Z INFO      • Gas ranking: 8ms
2024-01-15T10:30:15Z INFO      • Price quote: 15ms
2024-01-15T10:30:15Z INFO      • TX build: 6ms
2024-01-15T10:30:15Z INFO      • TX submit: 145ms
```

### Database Schema
Execution data is stored in PostgreSQL tables:
- `trade_signals` - Signal tracking with status state machine  
- `executions` - Detailed performance metrics
- `wallets` - Wallet configuration and stats

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific module tests  
cargo test tx_executor

# Run with logging
RUST_LOG=debug cargo test

# Performance benchmarks
cargo test --release --test benchmarks
```

## 📦 Dependencies

### Core Dependencies
- `ethers` - Ethereum interaction
- `tokio` - Async runtime
- `sqlx` - Database connectivity
- `serde` - Serialization
- `tracing` - Structured logging

### Security Dependencies
- `eth-keystore` - Encrypted key management
- `secrecy` - Secret value protection  
- `zeroize` - Secure memory clearing

## 🔗 Integration

ETH Kartal integrates with:
- **Mempool Processors** - Real-time signal generation
- **PostgreSQL** - Trade logging and analytics
- **Flashbots** - MEV protection
- **Local Reth Node** - Fast RPC access
- **ZMQ Messaging** - High-speed alert ingestion

## 📄 License

This project is for educational and research purposes. Production use requires appropriate trading licenses and compliance with applicable financial regulations.