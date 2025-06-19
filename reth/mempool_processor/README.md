# Mempool Processor - Real-time Ethereum Scam Detection System

## Overview

High-performance Rust system for real-time Ethereum mempool monitoring, transaction simulation, and scam detection. Detects liquidity drains and rugpulls in under 2ms, enabling protective trading and analysis.

## 🎯 Production Performance

- **Average Latency**: 1.86ms (IPC detection + simulation)
- **Throughput**: 500+ transactions/second
- **Detection Rate**: 18 scams detected, 59.41 ETH saved in 1h 43m
- **Resource Usage**: 30MB RAM, 1.4% CPU
- **Uptime**: Continuous operation with no memory leaks

## 🏗️ Architecture

```
mempool_signal_detection_full_tx_ipc (Main Binary)
│
├── IPC Full TX Client ──────> Reth Node (localhost:8545)
│   └── Unix socket subscription for transaction hashes
│   └── Fetch full transaction data via IPC
│
├── Transaction Simulator ────> debug_traceCall RPC
│   └── Simulate transaction execution
│   └── Extract state changes and pool effects
│
├── Pool Subscriber ─────────> Python ZeroMQ Service
│   └── Real-time pool state updates
│   └── In-memory cache of 1365+ pools
│
├── Signal Engine ───────────> Scam Detection Logic
│   └── Analyze pool ETH changes
│   └── Detect liquidity drains (>50%)
│   └── Generate market events
│
└── Database Writer ─────────> PostgreSQL (eth_db)
    └── Log scam predictions
    └── Store market events
```

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+
- Running Reth node with IPC enabled
- PostgreSQL database (eth_db)
- Python pool subscriber service (port 5557)

### Run Production System
```bash
# Build optimized binary
cargo build --release --bin mempool_signal_detection_full_tx_ipc

# Run with default settings (50% drain threshold)
./target/release/mempool_signal_detection_full_tx_ipc

# Run with custom thresholds
./target/release/mempool_signal_detection_full_tx_ipc \
  --eth-threshold 0.01 \
  --percentage-threshold 0.3
```

### Monitor Performance
```bash
# Watch real-time logs
tail -f /home/nima/code/crypto/logs/mempool/timing_reports_full_tx_*.log

# Check scam detections
tail -f /home/nima/code/crypto/logs/mempool/scam_alerts_full_tx_*.log
```

## 📁 Project Structure

### Core Modules (Active)
```
src/
├── bin/
│   ├── mempool_signal_detection_full_tx_ipc.rs  # Main production binary
│   ├── mempool_tracker.rs                       # Basic tracking tool
│   └── metrics_api_server.rs                    # HTTP metrics API
│
├── signal_engine/        # Market event and scam detection
│   ├── engine.rs        # Core detection algorithms
│   ├── service.rs       # Service wrapper with DB integration
│   └── types.rs         # Event types (ScamAlert, LiquidityWarning, etc.)
│
├── tx_simulator/         # Transaction simulation using debug_traceCall
│   ├── debug_tracecall_simulator.rs             # Fast production simulator
│   ├── debug_tracecall_state_diff_calculator.rs # State change analysis
│   └── state_diff_types.rs                      # Core types
│
├── pool_subscriber/      # Real-time pool state via ZeroMQ
│   ├── cache.rs         # In-memory pool state cache
│   ├── types.rs         # Pool update structures
│   └── tests/           # Unit tests
│
├── database/            # PostgreSQL integration
│   └── scam_prediction_writer.rs  # Writes scam detections to DB
│
├── mempool_fetcher/     # Transaction detection methods
│   ├── ipc_ipc/         # IPC subscription + fetch
│   ├── ipc_ipc_variants/
│   │   └── full_tx_client.rs     # Main active IPC implementation
│   ├── websocket/       # WebSocket client (unused)
│   └── processor/       # Transaction processing
│       ├── processor.rs # Transaction processor
│       ├── pools.rs     # Pool tracking
│       └── scam_prediction_writer.rs
│
└── common/              # Shared utilities
    ├── address.rs       # Address utilities
    ├── constants.rs     # System constants
    └── types.rs         # Common types
```

### Module Status
- ✅ **Active**: signal_engine, tx_simulator, pool_subscriber, database, common
- ✅ **Partial**: mempool_fetcher (only IPC variants used)
- ❌ **Removed**: DevP2P, validation_testing, 9 broken binaries

### Dependencies
```
mempool_signal_detection_full_tx_ipc
├── signal_engine (scam detection)
├── tx_simulator (debug_traceCall)
├── pool_subscriber (ZeroMQ updates)
├── database (PostgreSQL writer)
├── mempool_fetcher/ipc_ipc_variants (Full TX IPC)
└── common (utilities)
```

## 🔧 Configuration

### Environment Variables
```bash
ETH_RPC_URL=http://localhost:8545      # Reth HTTP RPC
IPC_PATH=/tmp/reth.ipc                 # Reth IPC socket
POOL_ZMQ_ADDRESS=tcp://localhost:5557  # Pool updates
DB_HOST=localhost                      # PostgreSQL host
DB_NAME=eth_db                         # Database name
```

### Detection Thresholds
- **ETH Threshold**: 0.01 ETH (minimum pool size)
- **Percentage Threshold**: 50% (drain percentage for scam alert)
- **Liquidity Warning**: 20% (significant change warning)

## 📊 Metrics & Monitoring

### Log Files
- **Timing Reports**: Detailed performance metrics per transaction
- **Scam Alerts**: Detected rugpulls with transaction details
- **Market Events**: All liquidity changes and warnings

### Example Scam Detection
```
🚨 ScamAlert | TX: 0x7ea69e87... | Pool: 0x97dC7F34... 
ETH: 15.308459 -> 0.000000 (-100.00% loss) 
Lost: 15.308459 ETH | IPC: 0.769ms
```

## 🛠️ Development

### Build & Test
```bash
# Build all
cargo build

# Run tests
cargo test

# Check for issues
cargo check
cargo clippy
```

### Adding New Detection Logic
1. Modify `src/signal_engine/engine.rs`
2. Add new event types in `src/signal_engine/types.rs`
3. Update thresholds in configuration

## 📈 Performance Tuning

### Current Optimizations
- Unix socket IPC for minimal latency
- Concurrent transaction processing
- In-memory pool state caching
- Batch database writes

### Bottlenecks
- RPC simulation calls (1.8ms average)
- Network latency to Reth node
- Pool state update frequency

## 🤝 Contributing

1. Check `AUDIT_REPORT.md` for codebase overview
2. Follow existing code patterns
3. Ensure all tests pass
4. Update documentation

## 📜 License

Proprietary - See LICENSE file

## 🔗 Related Projects

- [eth_kartal](../eth_kartal) - Transaction execution engine
- [revm_tx_simulator](../revm_tx_simulator) - REVM-based simulation
- [sarigoz](../../py/sarigoz) - Web analytics frontend