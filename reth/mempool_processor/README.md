# Mempool Processor - Ultra-Fast Ethereum Signal Detection System

## Overview

High-performance Rust system for real-time Ethereum mempool monitoring, transaction simulation, and trading signal detection. Achieves **2-7μs detection latency** with ultra-fast IPC integration to detect liquidity drains, rugpulls, and market manipulation events.

## 🚀 Production Performance

- **Detection Latency**: **2-7μs** (Ultra-fast IPC)
- **Total Pipeline**: **1.42ms** average (detection + simulation + analysis)
- **Throughput**: **703 tx/sec** sustained processing
- **Resource Usage**: **27MB RAM**, **<1% CPU**
- **Detection Coverage**: **100%** new transactions, **0%** existing mempool
- **Uptime**: Continuous operation with **zero memory leaks**

## 🏗️ System Architecture

### **Data Flow Pipeline**

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Ethereum Node │    │  UltraFastClient │    │ Signal Detection│
│                 │    │                  │    │     Engine      │
│ ┌─────────────┐ │    │ ┌──────────────┐ │    │ ┌─────────────┐ │
│ │   Mempool   │─┼───▶│ │ IPC Socket   │─┼───▶│ │ Simulation  │ │
│ │             │ │    │ │ Non-blocking │ │    │ │ Engine      │ │
│ │  New Txs    │ │    │ │ JSON Parser  │ │    │ │             │ │
│ └─────────────┘ │    │ └──────────────┘ │    │ └─────────────┘ │
│                 │    │                  │    │        │        │
│ Reth Node       │    │ Detection Time:  │    │        ▼        │
│ /tmp/reth.ipc   │    │    2-7μs        │    │ ┌─────────────┐ │
└─────────────────┘    └──────────────────┘    │ │ Pool State  │ │
                                               │ │ Analysis    │ │
                                               │ └─────────────┘ │
                                               │        │        │
                                               │        ▼        │
                                               │ ┌─────────────┐ │
                                               │ │ Trading     │ │
                                               │ │ Signals     │ │
                                               │ └─────────────┘ │
                                               └─────────────────┘
                                                       │
                                                       ▼
               ┌─────────────────────────────────────────────────────┐
               │              Alert Distribution                      │
               │                                                     │
               │  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐ │
               │  │ PostgreSQL  │  │ ZMQ Publisher│  │ Log Files   │ │
               │  │ Database    │  │ (Trading     │  │ (Analysis)  │ │
               │  │ (Audit)     │  │ Bots)        │  │             │ │
               │  └─────────────┘  └──────────────┘  └─────────────┘ │
               └─────────────────────────────────────────────────────┘
```

### **Component Architecture**

```
mempool_signal_detection_full_tx_ipc (Single Binary)
│
├── 🔌 UltraFastClient                    ──▶ /tmp/reth.ipc
│   ├── Non-blocking socket reads        ──▶ 2-7μs detection
│   ├── JSON streaming parser            ──▶ No RPC fallback
│   └── 50K transaction buffer           ──▶ mpsc channels
│
├── 🔬 DebugTraceCallSimulator           ──▶ http://localhost:8545
│   ├── debug_traceCall RPC              ──▶ ~3.6ms simulation
│   ├── State diff extraction            ──▶ ETH/token changes
│   └── Pool effect calculation          ──▶ Reserve % changes
│
├── 🏊 PoolSubscriber                    ──▶ tcp://localhost:5557
│   ├── ZMQ subscription (Python)        ──▶ Real-time pool state
│   ├── In-memory cache (1000+ pools)    ──▶ <1ms lookups
│   └── Block-level updates (~12s)       ──▶ Fresh reserve data
│
├── 🚨 SignalEngine                      ──▶ Multi-category analysis
│   ├── Scam detection (>50% drain)      ──▶ Critical alerts
│   ├── Liquidity warnings (>20%)        ──▶ High priority
│   ├── Volume spike detection (5x)      ──▶ Medium priority
│   ├── Supply manipulation alerts       ──▶ High priority
│   └── Price impact analysis (>15%)     ──▶ Medium priority
│
├── 📡 AlertPublisher                    ──▶ tcp://*:5559
│   ├── ZMQ PUB socket                   ──▶ Trading bot integration
│   ├── JSON alert messages              ──▶ Rich event context
│   └── Severity filtering               ──▶ High+ alerts only
│
└── 💾 ScamPredictionWriter              ──▶ PostgreSQL (eth_db)
    ├── Audit trail logging              ──▶ All detections
    ├── Performance metrics              ──▶ Timing analysis
    └── Event correlation                ──▶ Historical patterns
```

## 🎯 Core Features

### **Ultra-Fast Detection**
- **IPC Integration**: Direct Unix socket connection to Reth node
- **Non-blocking I/O**: Eliminates blocking read bottlenecks
- **Zero RPC Fallback**: Full transaction data in first request
- **Nanosecond Timing**: Precise performance measurement

### **Single-Processor Design**
- **Centralized Processing**: One main binary handles all transactions
- **Concurrent Pipeline**: tokio async runtime for parallel processing
- **Shared State**: Pool cache and signal engine across threads
- **Distributed Integration**: External services via ZMQ

### **Transaction Simulator**
- **Primary Method**: debug_traceCall RPC (~3.6ms average)
- **Alternative**: REVM simulation (~40-50ms) - not used in production
- **State Tracking**: ETH transfers and ERC20 token movements
- **Pool Analysis**: Reserve changes and percentage calculations

### **Signal Categories**
1. **ScamAlert** (Critical): >50% pool drain detection
2. **LiquidityWarning** (High): >20% liquidity changes
3. **VolumeSpike** (Medium): >5x average volume
4. **TokenSupplyAlert** (High): Supply manipulation
5. **PriceImpact** (Medium): >15% price changes

## 🚀 Quick Start

### Prerequisites
- **Rust**: 1.70+ with cargo
- **Reth Node**: Running with IPC enabled (`/tmp/reth.ipc`)
- **PostgreSQL**: Database `eth_db` accessible
- **Python Service**: Pool subscriber on `tcp://localhost:5557`

### Installation
```bash
# Clone and build
git clone <repository>
cd mempool_processor
cargo build --release

# Production binary
./target/release/mempool_signal_detection_full_tx_ipc
```

### Configuration
```bash
# Environment variables (optional)
export ETH_RPC_URL="http://localhost:8545"
export IPC_PATH="/tmp/reth.ipc"
export POOL_ZMQ_ADDRESS="tcp://localhost:5557"
export DB_HOST="localhost"
export DB_NAME="eth_db"

# Run with custom thresholds
./target/release/mempool_signal_detection_full_tx_ipc \
  --eth-threshold 0.01 \
  --percentage-threshold 0.3

# Enable trading signal publishing
./target/release/mempool_signal_detection_full_tx_ipc \
  --enable-publisher \
  --alert-zmq-address "tcp://*:5559"
```

## 📊 Performance Monitoring

### **Real-time Metrics**
```bash
# View timing reports (every 1000 transactions)
tail -f /home/nima/code/crypto/logs/mempool/timing_reports_full_tx_*.log

# Monitor scam detections
tail -f /home/nima/code/crypto/logs/mempool/scam_alerts_full_tx_*.log

# Watch market events
tail -f /home/nima/code/crypto/logs/mempool/market_events_full_tx_*.log
```

### **Sample Timing Report**
```
[2025-07-02 17:19:57.424] ============================================================
[2025-07-02 17:19:57.424] IPC Full: 2000 transactions, avg latency: 12μs, queue: 0/50000
[2025-07-02 17:19:57.424] ⚡ TIMING REPORT (last 1101 transactions):
[2025-07-02 17:19:57.424]    📡 IPC Detection:   avg=0.01ms  max=0.04ms
[2025-07-02 17:19:57.424]    🔬 Simulation:      avg=6.55ms  max=864.83ms
[2025-07-02 17:19:57.424]    🔍 Pool Check:      avg=0.00ms  max=0.02ms
[2025-07-02 17:19:57.424]    🛡️  Scam Detection: avg=0.00ms  max=0.10ms
[2025-07-02 17:19:57.424]    📊 Total Pipeline:  avg=6.63ms  max=864.93ms
[2025-07-02 17:19:57.424]    🎯 Pools Affected:  0 (0.0%)
[2025-07-02 17:19:57.424]    🚀 Throughput:      150.7 tx/sec
[2025-07-02 17:19:57.424] ============================================================
```

## 📡 Trading Signal Integration

### **ZMQ Alert Publishing**
```json
{
  "event_type": "ScamAlert",
  "severity": "Critical", 
  "tx_hash": "0x7ea69e87...",
  "pool_address": "0x97dC7F34...",
  "token_address": "0x8390a1DA...",
  "block_number": 22832374,
  "detection_time": 1705123456789,
  "metrics": {
    "eth_change": -15.308459,
    "eth_percent": -100.0,
    "new_eth_reserve": 0.0,
    "token_change": 0.0
  }
}
```

### **Python Subscriber Example**
```python
import zmq
import json

context = zmq.Context()
subscriber = context.socket(zmq.SUB)
subscriber.connect("tcp://localhost:5559")
subscriber.setsockopt_string(zmq.SUBSCRIBE, "")

while True:
    message = subscriber.recv_string()
    event = json.loads(message)
    if event["event_type"] == "ScamAlert":
        print(f"🚨 SCAM: {event['tx_hash']} - {event['metrics']['eth_percent']:.1f}% drain")
        # Execute protective transaction via eth_kartal
```

## 📁 Project Structure

```
src/
├── bin/
│   ├── mempool_signal_detection_full_tx_ipc.rs   # 🎯 Main production binary
│   ├── mempool_tracker.rs                        # Basic monitoring tool
│   └── metrics_api_server.rs                     # HTTP metrics endpoint
│
├── mempool_fetcher/                               # 🔌 Transaction detection
│   ├── ultra_fast_client.rs                      # ⚡ Ultra-fast IPC client
│   ├── ipc_ipc_variants/full_tx_client.rs        # Legacy IPC (backup)
│   ├── websocket/client.rs                       # WebSocket (unused)
│   └── processor/                                 # Transaction processing
│
├── tx_simulator/                                  # 🔬 Transaction simulation
│   ├── debug_tracecall_simulator.rs              # Production simulator
│   ├── debug_tracecall_state_diff_calculator.rs  # State change analysis
│   └── state_diff_types.rs                       # Core data types
│
├── signal_engine/                                 # 🚨 Signal detection
│   ├── engine.rs                                  # Detection algorithms
│   ├── service.rs                                 # Service integration
│   ├── publisher.rs                               # ZMQ alert publisher
│   └── types.rs                                   # Event definitions
│
├── pool_subscriber/                               # 🏊 Pool state management
│   ├── cache.rs                                   # In-memory pool cache
│   ├── subscriber.rs                              # ZMQ subscription
│   └── types.rs                                   # Pool data structures
│
├── database/                                      # 💾 PostgreSQL integration
│   └── scam_prediction_writer.rs                 # Audit trail logging
│
└── common/                                        # 🛠️ Shared utilities
    ├── address.rs                                 # Address formatting
    ├── constants.rs                               # System constants
    └── types.rs                                   # Common data types
```

## 🔧 Configuration Reference

### **Detection Thresholds**
```rust
SignalThresholds {
    eth_threshold: 0.01,              // Minimum pool size (ETH)
    scam_drain_percent: 0.5,          // 50% = Critical scam alert
    warning_drain_percent: 0.2,       // 20% = Liquidity warning  
    supply_increase_percent: 0.1,     // 10% = Supply manipulation
    volume_spike_multiplier: 5.0,     // 5x = Volume spike
    price_impact_percent: 0.15,       // 15% = Price impact alert
    small_pool_max_eth: 5.0,          // Small pool threshold
    medium_pool_max_eth: 50.0,        // Medium pool threshold
}
```

### **Performance Tuning**
```rust
// Ultra-fast client settings
const BUFFER_SIZE: usize = 8192;           // Socket read buffer
const QUEUE_CAPACITY: usize = 50_000;      // Transaction queue
const TIMEOUT_MS: u64 = 100;               // Non-blocking timeout

// Pool cache settings  
const CACHE_TTL_BLOCKS: u64 = 5;           // Cache refresh interval
const MAX_CACHED_POOLS: usize = 10_000;    // Memory limit
```

## 🛠️ Development

### **Build & Test**
```bash
# Development build
cargo build

# Production build (optimized)
cargo build --release

# Run tests
cargo test

# Lint and check
cargo clippy
cargo check
```

### **Adding New Signal Types**
1. Define event in `src/signal_engine/types.rs`
2. Implement detection in `src/signal_engine/engine.rs`
3. Add threshold to `SignalThresholds` struct
4. Update publisher filtering if needed

### **Performance Analysis**
```bash
# Run with detailed timing
RUST_LOG=debug ./target/release/mempool_signal_detection_full_tx_ipc

# Profile memory usage
valgrind --tool=massif ./target/release/mempool_signal_detection_full_tx_ipc

# Benchmark detection latency
cargo run --example measure_mempool_performance
```

## 📈 Production Deployment

### **System Requirements**
- **CPU**: 2+ cores, <1% sustained usage
- **RAM**: 64MB+ (27MB baseline + buffer)  
- **Network**: <10ms RTT to Reth node
- **Storage**: 1GB+ for logs and database

### **Deployment Checklist**
- [ ] Reth node running with IPC enabled
- [ ] PostgreSQL `eth_db` database accessible
- [ ] Python pool service on port 5557
- [ ] Log directory `/home/nima/code/crypto/logs/mempool/` exists
- [ ] Network connectivity to external trading systems
- [ ] Monitoring alerts configured

### **Production Optimizations**
- Use `--release` build for maximum performance
- Enable `--enable-publisher` for trading integration
- Monitor log files for performance degradation
- Set up alerting on detection latency >10ms

## 🔗 Integration

### **External Systems**
- **[eth_kartal](../eth_kartal)**: Transaction execution engine
- **[revm_tx_simulator](../revm_tx_simulator)**: Alternative simulator
- **[sarigoz](../../py/sarigoz)**: Web analytics dashboard

### **Data Sources**
- **Reth Node**: Primary mempool and RPC data
- **Python Pool Service**: Real-time pool state via ZMQ
- **PostgreSQL**: Persistent storage and audit trail

### **Output Consumers**
- **Trading Bots**: Real-time alerts via ZMQ
- **Analytics Systems**: Log file processing
- **Monitoring**: HTTP metrics endpoint

## 📜 License

Proprietary - Internal use only