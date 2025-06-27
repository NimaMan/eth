# eth_kartal 🛡️

**High-Performance Ethereum Transaction Execution Engine**

A sub-200ms transaction execution system designed for scam protection and automated trading on Ethereum. Built for speed, reliability, and MEV protection.

## 🎯 Mission

Execute protective transactions faster than malicious actors by:
- Processing real-time scam detection alerts
- Optimizing gas prices for mempool positioning  
- Executing transactions within 200ms of alert receipt
- Protecting against MEV attacks and front-running

## ⚡ Performance Metrics

- **Target Latency**: <200ms alert-to-execution
- **Measured Performance**: ~90ms total execution time
- **Initialization**: 48ms (one-time cost)
- **Mempool Positioning**: Top 1-5% based on priority

## 🏗️ Architecture

```
ZMQ Alerts → Alert Processor → Transaction Executor
                                      ↓
           Gas Optimizer ← Position Calculator ← Mempool Tracker
                  ↓                                   ↓
           Risk Manager → Pool Factory → DEX Pools
                                      ↓
                              RPC Submission → Ethereum
```

## 📁 Module Overview

| Module | Purpose | Status | Lines |
|--------|---------|--------|-------|
| `alert_processor/` | ZMQ alert reception & parsing | ✅ Complete | ~280 |
| `tx_executor/` | Transaction building & execution | 🟡 Sell only | ~520 |
| `ranking/` | Gas optimization & mempool analysis | ✅ Complete | ~1,200 |
| `pools/` | DEX protocol abstractions | 🟡 V2 only | ~400 |
| `wallet/` | Position tracking & balances | ✅ Complete | ~300 |
| `risk/` | Circuit breakers & safety | ✅ Complete | ~1,700 |

## 🚀 Quick Start

### Prerequisites

```bash
# Reth node running locally
reth node --http --ws --http.api eth,net,web3 --ws.api eth,net,web3

# Environment setup
export PRIVATE_KEY="0x..." 
export RPC_URL="http://127.0.0.1:8545"
export RETH_WS_URL="ws://127.0.0.1:8546"
```

### Build & Test

```bash
# Build the project
cargo build --release

# Run performance test
cargo run --example performance_test

# Run the main executor
cargo run --bin kartal
```

## 🔧 Configuration

Key configuration via environment variables:

```bash
# Execution
PRIVATE_KEY="0x..."              # Wallet private key
RPC_URL="http://127.0.0.1:8545"  # Ethereum RPC endpoint
RETH_WS_URL="ws://127.0.0.1:8546" # WebSocket for mempool

# Alerts
ZMQ_ENDPOINT="tcp://localhost:5559" # Alert source

# Risk Management  
MAX_DAILY_LOSS="1000000000000000000" # 1 ETH in wei
CIRCUIT_BREAKER_THRESHOLD="5"        # Max failures before halt
```

## 📊 Current Capabilities

### ✅ Implemented
- **Alert Processing**: ZMQ subscription with automatic reconnection
- **Sell Transactions**: Complete Uniswap V2 sell execution
- **Gas Optimization**: Dynamic pricing based on mempool analysis
- **Position Tracking**: Real-time token balance management
- **Risk Management**: Circuit breakers and loss limits
- **Performance Metrics**: Detailed execution timing

### 🟡 Partial
- **DEX Support**: Uniswap V2 only (V3/V4 planned)
- **Transaction Types**: Sell only (buy implementation needed)
- **MEV Protection**: Public mempool only (Flashbots planned)

### ❌ Planned
- **Flashbots Integration**: Private mempool submission
- **Multi-Protocol DEX**: V3, V4, SushiSwap support
- **Advanced Analytics**: Historical performance tracking

## 🧪 Testing

### Performance Test
```bash
cargo run --example performance_test
```
Validates complete alert→execution pipeline with timing metrics.

### Unit Tests
```bash
cargo test
```
Tests individual components and integration points.

## 📈 Performance Analysis

Recent performance test results:
- **Executor Initialization**: 48ms
- **Alert Processing**: <1ms  
- **Position Check**: 1ms
- **Gas Ranking**: <1ms
- **Price Quote**: <1ms
- **TX Build**: <1ms
- **TX Submit**: <1ms
- **Total**: ~90ms (target: <200ms) ✅

## 🛠️ Development

### Adding New DEX Support
1. Implement the `Pool` trait in `src/pools/`
2. Add factory methods in `PoolFactory`
3. Update router logic for protocol selection

### Adding New Alert Types
1. Extend `Action` enum in `alert_processor/types.rs`
2. Add execution logic in `tx_executor/executor.rs`
3. Update risk management rules if needed

## 🔐 Security Considerations

- **Private Key Management**: Never commit keys to repository
- **Risk Limits**: Automatic halt on excessive losses
- **MEV Protection**: Gas optimization and Flashbots integration
- **Circuit Breakers**: Automatic disable on repeated failures

## 📋 TODO

### High Priority
- [ ] Implement buy transaction logic
- [ ] Add Flashbots submission support
- [ ] Complete common module with shared types

### Medium Priority  
- [ ] Add Uniswap V3 pool support
- [ ] Enhanced MEV detection algorithms
- [ ] Integration tests with live alerts

### Low Priority
- [ ] Additional DEX protocols
- [ ] Advanced performance analytics
- [ ] Historical data analysis

## 🤝 Contributing

1. Follow the modular architecture patterns
2. Add comprehensive tests for new functionality
3. Maintain performance targets (<200ms execution)
4. Document all public APIs and configuration

## 📄 License

MIT License - See LICENSE file for details.

---

**⚠️ Warning**: This system handles real cryptocurrency transactions. Always test thoroughly on testnets before mainnet deployment.