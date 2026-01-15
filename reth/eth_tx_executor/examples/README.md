# ETH Kartal Examples

Comprehensive examples demonstrating key functionality of the eth_kartal transaction execution engine.

## Core Examples

### **`signal_to_tx_demo.rs`** - Complete Signal Processing
Demonstrates the full alert-to-execution pipeline:
- ZMQ alert processing
- Risk assessment and validation  
- Gas optimization
- MEV-protected execution
- Trade logging

```bash
cargo run --example signal_to_tx_demo
```

### **`secure_wallet_demo.rs`** - Wallet Management
Shows secure keystore management:
- Keystore creation and loading
- Secure password handling
- Auto-lock functionality
- Transaction signing

```bash
cargo run --example secure_wallet_demo
```

### **`swap_simulator.rs`** - Interactive Trading
Interactive swap simulation with real pool data:
- Multiple token pairs
- Real-time quotes
- Slippage configuration
- Gas optimization

```bash
cargo run --example swap_simulator
```

### **`flashbots_demo.rs`** - MEV Protection
Demonstrates Flashbots integration:
- Bundle building
- Private mempool submission
- MEV protection strategies
- Fallback mechanisms

```bash
cargo run --example flashbots_demo
```

## Advanced Examples

### **`accurate_performance_test.rs`** - Performance Analysis
Measures actual execution performance:
- End-to-end latency testing
- Component timing breakdown
- Throughput measurement
- SLA compliance verification

```bash
cargo run --release --example accurate_performance_test
```

### **`secure_wallet_swap_simulator.rs`** - Production Simulation
Combines secure wallet with swap execution:
- Keystore-based authentication
- Real transaction building
- Risk management integration
- Comprehensive logging

```bash
cargo run --example secure_wallet_swap_simulator
```

### **`mock_flashbots_relay.rs`** - Testing Infrastructure
Mock Flashbots relay for testing:
- Bundle simulation
- Local testing without mainnet
- Integration test support

```bash
cargo run --example mock_flashbots_relay
```

## Testing Examples

### **`test_bundle_building.rs`** - Bundle Construction
Tests Flashbots bundle building:
- Transaction ordering
- Bundle validation
- Tip calculation
- Bundle hash generation

### **`test_e2e_flashbots_flow.rs`** - End-to-End Testing
Complete Flashbots integration test:
- Bundle submission
- Confirmation tracking
- Error handling
- Fallback scenarios

### **`secure_wallet_working.rs`** - Wallet Verification
Verifies wallet functionality:
- Keystore integrity
- Signing correctness
- Security features

## Prerequisites

All examples require:

### **Environment Setup**
```bash
# Set required environment variables
export ETH_RPC_URL="http://127.0.0.1:8545"
export ETH_KEYSTORE_PATH="/path/to/your/keystore.json"

# Optional for database integration
export DATABASE_URL="postgres://user:pass@localhost/eth_db"
```

### **Local Reth Node**
Most examples work best with a local Reth node for fast, reliable RPC access:
```bash
# Start Reth node
reth node --http --http.port 8545 --ws --ws.port 8546
```

### **Test Keystore**
Create a test keystore for examples:
```bash
cargo run --bin keystore_manager create --output test_keystore.json --chain-id 1
```

## Usage Patterns

### **Basic Simulation**
For learning and testing without real transactions:
```bash
# Safe simulation mode
cargo run --example swap_simulator
```

### **Performance Testing**
For measuring and optimizing system performance:
```bash
# Performance benchmarking
cargo run --release --example accurate_performance_test
```

### **Integration Testing**
For testing with real infrastructure:
```bash
# Full integration test
export ETH_KEYSTORE_PATH="/path/to/test/keystore.json"
cargo run --example secure_wallet_swap_simulator
```

### **MEV Protection Testing**
For testing Flashbots integration:
```bash
# Flashbots testing
cargo run --example flashbots_demo
```

## Example Output

### Signal Processing (`signal_to_tx_demo.rs`)
```
🚀 ETH Kartal Signal Processing Demo
📡 Starting ZMQ alert receiver on tcp://127.0.0.1:5555
📨 Alert received: buy_signal_001 | Token: 0xA0b8... | Action: BUY
⚖️ Risk decision: ALLOW | Alert: buy_signal_001 | Original: 500000000000000000
📤 Transaction submitted: 0x1a2b... | Alert: buy_signal_001 | Path: FlashbotsBundle  
✅ Execution successful: 0x1a2b... | Latency: 187ms
  📊 Performance breakdown:
    • Alert→Start: 1ms
    • Position check: 12ms
    • Gas ranking: 8ms
    • Price quote: 15ms
    • TX build: 6ms
    • TX submit: 145ms
```

### Performance Testing (`accurate_performance_test.rs`)
```
🔬 ETH Kartal Performance Analysis
📊 Test Configuration:
  • Signal count: 100
  • Target latency: <200ms
  • SLA threshold: 95%

📈 Results:
  • Mean latency: 142ms
  • 95th percentile: 186ms
  • 99th percentile: 234ms
  • SLA compliance: 97.2%
  • Theoretical TPS: 187,611

✅ Performance targets met!
```

### Secure Wallet (`secure_wallet_demo.rs`)
```
🔐 ETH Kartal Secure Wallet Demo
📂 Loading keystore: /path/to/keystore.json
🔓 Wallet unlocked for address: 0xYourAddress...
🔒 Wallet auto-locked after 5 minutes
✅ Security features verified
```

## Development Tips

### **Adding New Examples**
1. Create new `.rs` file in `examples/` directory
2. Follow naming pattern: `feature_description.rs`
3. Add comprehensive documentation
4. Include error handling and logging
5. Update this README

### **Example Structure**
```rust
//! Example: Feature Description
//! 
//! Demonstrates specific functionality with clear explanations

use eth_kartal::*;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 ETH Kartal Example: Feature Description");
    
    // Example implementation
    
    Ok(())
}
```

### **Testing Examples**
```bash
# Test all examples compile
cargo check --examples

# Run specific example
cargo run --example example_name

# Run with logging
RUST_LOG=debug cargo run --example example_name
```

## Integration with Main System

These examples demonstrate components that integrate with the main eth_kartal system:

- **Alert Processing** → Real-time signal handling
- **Risk Management** → Position and loss protection  
- **Gas Optimization** → Mempool analysis and gas pricing
- **MEV Protection** → Flashbots bundle submission
- **Secure Wallet** → Encrypted key management
- **Trade Logging** → Comprehensive audit trails

Each example can be used as a starting point for building production integrations or for understanding system capabilities.