# Mempool Processor - Real-time Ethereum Market Decision Engine

A high-performance Rust system for real-time mempool monitoring, transaction simulation, and market event detection on Ethereum. This module processes pending transactions to identify market opportunities, risks, and anomalies before they are confirmed on-chain.

## 🏗️ System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                           MEMPOOL PROCESSOR DATA FLOW                                │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  INPUTS                          PROCESSING                         OUTPUTS         │
│  ──────                          ──────────                         ───────         │
│                                                                                     │
│  Reth Node                   ┌─────────────────┐                                  │
│  ├─ WebSocket :8546  ───────►│ Mempool Fetcher │                                  │
│  │  └─ Pending TXs           │  (WebSocket)    │                                  │
│  │                           └────────┬─────────┘                                  │
│  │                                    │ TX Stream                                  │
│  │                                    ▼                                            │
│  │                           ┌─────────────────┐         ┌──────────────────┐     │
│  └─ HTTP RPC :8545  ────────►│  TX Simulator   │────────►│ Decision Engine  │     │
│     └─ debug_traceCall       │ (State Changes) │         │ (Market Events)  │     │
│                              └─────────────────┘         └─────┬────────────┘     │
│                                                                 │                  │
│  Python Services                    ┌──────────────────┐       │ Events           │
│  ├─ Pool Publisher  ────────────────┤ Pool Subscriber  │───────┘                  │
│  │  ├─ PUB :5557                   │  (Pool Cache)    │                          │
│  │  └─ REP :5558                   └──────────────────┘                          │
│  │                                                                                │
│  └─ Live Block Processor                                   ┌─────────────────┐    │
│     └─ Pool State Updates                                  │   PostgreSQL    │    │
│                                                           │   Database      │    │
│                                                           └─────────────────┘    │
│                                                                    ▲             │
│                                                                    │             │
│                                     ┌──────────────────────────────┼─────────┐   │
│                                     │         OUTPUT CHANNELS      │         │   │
│                                     ├──────────────────────────────┴─────────┤   │
│                                     │                                        │   │
│                                     │  1. Log Files (timestamped)           │   │
│                                     │     └─ /logs/mempool/*.log            │   │
│                                     │                                        │   │
│                                     │  2. Database Records                   │   │
│                                     │     └─ mempool_market_events table    │   │
│                                     │                                        │   │
│                                     │  3. ZMQ Publisher :5559 ──────────────┼───►│ eth_kartal
│                                     │     └─ Market event signals           │   │ (Action Engine)
│                                     │                                        │   │
│                                     └────────────────────────────────────────┘   │
│                                                                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

## 📊 Data Flow Details

### 1. Transaction Input Pipeline
```
WebSocket Connection (ws://127.0.0.1:8546)
    ↓
Pending Transaction Stream (10μs latency)
    ↓
Transaction Queue (10,000 buffer)
    ↓
Batch Processing (up to 100 TXs)
```

### 2. Simulation & Analysis Pipeline
```
Transaction Batch
    ↓
Concurrent debug_traceCall (~5ms per TX)
    ↓
State Change Extraction (ETH & Token transfers)
    ↓
Pool Impact Calculation
    ↓
Decision Engine Analysis
    ↓
Categorized Market Events
```

### 3. Market Event Categories

| Event Type | Trigger Condition | Severity | Action |
|------------|------------------|----------|---------|
| **ScamAlert** | Liquidity drain > 50% | Critical | Immediate action required |
| **LiquidityWarning** | Liquidity change 20-50% | High | Monitor closely |
| **TokenSupplyAlert** | Supply increase > 10% | High | Possible hidden mint |
| **VolumeSpike** | Volume > 5x average | Medium | Market manipulation check |
| **PriceImpact** | Price change > 15% | Medium | Arbitrage opportunity |

## 🚀 Quick Start

```bash
# 1. Ensure services are running
# - Reth node with WebSocket and debug API
# - PostgreSQL database
# - Python pool publisher

# 2. Set up environment (optional - defaults provided)
export MEMPOOL_LOG_DIR="/home/nima/code/crypto/logs/mempool"
export DB_USER="postgres"
export DB_PASSWORD="postgres"
export DB_HOST="localhost"
export DB_PORT="5432"
export DB_NAME="eth_db"

# 3. Run the market monitor
cargo run --release --bin mempool_scam_monitor

# 4. View real-time events
tail -f $MEMPOOL_LOG_DIR/market_events_*.log
```

## 📁 Input/Output Specifications

### Inputs

#### 1. WebSocket Pending Transactions
```json
{
  "jsonrpc": "2.0",
  "method": "eth_subscription",
  "params": {
    "subscription": "0x1234...",
    "result": {
      "hash": "0xabc123...",
      "from": "0xSender...",
      "to": "0xContract...",
      "value": "0x1234",
      "gas": "0x5678"
    }
  }
}
```

#### 2. Pool State Updates (ZMQ)
```json
{
  "type": "pool_updates",
  "timestamp": 1234567890.123,
  "data": {
    "0xPoolAddress": {
      "eth_reserve": 123.456,
      "token_reserve": 250000.0,
      "token_address": "0xToken...",
      "block_number": 12345678,
      "update_time": 1234567890.123
    }
  }
}
```

#### 3. debug_traceCall Response
```json
{
  "from": "0xSender",
  "to": "0xPool",
  "value": "0x0",
  "logs": [
    {
      "address": "0xToken",
      "topics": ["0xTransferTopic", "0xFrom", "0xTo"],
      "data": "0xAmount"
    }
  ]
}
```

### Outputs

#### 1. Market Event Signal (ZMQ Publisher :5559)
```json
{
  "event_type": "liquidity_warning",
  "severity": "high",
  "confidence": 0.95,
  "tx_hash": "0xabc123...",
  "pool": "0xPoolAddress",
  "token": "0xTokenAddress",
  "metrics": {
    "eth_change": -30.5,
    "eth_percent": -25.3,
    "token_change": 0,
    "token_percent": 0,
    "new_eth_reserve": 92.5,
    "new_token_reserve": 250000.0
  },
  "detection_time": 1234567890.123,
  "block_number": 12345678
}
```

#### 2. Log File Entry
```
2025-06-16T12:34:56.789Z | LIQUIDITY_WARNING | HIGH | tx: 0xabc... | pool: 0xdef... | ETH: -30.5 (-25.3%) | Token: 0 (0%) | confidence: 0.95 | latency: 2.3ms
```

#### 3. Database Record
```sql
-- Table: mempool_market_events
INSERT INTO mempool_market_events (
    event_type,
    severity,
    tx_hash,
    pool_address,
    token_address,
    eth_change,
    token_change,
    confidence,
    detection_time,
    metrics_json
) VALUES (...);
```

## 🔧 Components

### 1. **Mempool Fetcher** (`src/mempool_fetcher/`)
- WebSocket connection to local Reth node
- Sub-millisecond transaction detection (avg 10μs)
- 10,000 transaction buffer for burst handling
- Real-time latency tracking
- Concurrent batch processing

### 2. **Transaction Simulator** (`src/tx_simulator/`)
- Uses `debug_traceCall` RPC for fast simulation (~5ms)
- Extracts ETH and token balance changes
- Calculates net position changes per address
- Identifies pool interactions and DEX operations

### 3. **Pool Subscriber** (`src/pool_subscriber/`)
- ZeroMQ integration with Python pool publisher
- Real-time pool state updates (ETH & token reserves)
- Thread-safe in-memory cache
- Maintains state for ~2000 active pools

### 4. **Decision Engine** (`src/decision_engine/`)
- Multi-threshold market event detection
- Dynamic threshold adjustment based on pool size
- Confidence scoring based on data quality
- Categorized event generation

## 📈 Performance Metrics

Based on production monitoring:

| Metric | Average | Min | Max | Target |
|--------|---------|-----|-----|---------|
| **WebSocket Latency** | 0.01ms | 0.00ms | 0.06ms | <1ms |
| **TX Processing** | 2.29ms | 0.05ms | 20ms* | <10ms |
| **State Simulation** | 5ms | 3ms | 15ms | <10ms |
| **Event Detection** | 0.1ms | 0.05ms | 0.5ms | <1ms |
| **End-to-End** | 7.4ms | 3.1ms | 35ms | <20ms |

*Outliers excluded, typical performance shown

## 🛠️ Configuration

### Required Services
```bash
# 1. Reth node with WebSocket and debug API
reth node \
  --http --http.api eth,net,web3,debug \
  --ws --ws.api eth,net,web3,debug \
  --ws.port 8546

# 2. PostgreSQL database
psql -U postgres -c "CREATE DATABASE eth_db;"
psql -U postgres -d eth_db -c "
CREATE TABLE mempool_market_events (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(50),
    severity VARCHAR(20),
    tx_hash VARCHAR(66),
    pool_address VARCHAR(42),
    token_address VARCHAR(42),
    eth_change DECIMAL,
    token_change DECIMAL,
    confidence DECIMAL,
    detection_time TIMESTAMP,
    metrics_json JSONB
);"

# 3. Python pool publisher (separate repo)
cd /home/nima/code/crypto/py/eth_token
python -m eth_token.services.pool_level_publisher
```

### Environment Variables
| Variable | Default | Description |
|----------|---------|-------------|
| `MEMPOOL_LOG_DIR` | `/home/nima/code/crypto/logs/mempool` | Log file directory |
| `DB_USER` | `postgres` | PostgreSQL username |
| `DB_PASSWORD` | `postgres` | PostgreSQL password |
| `DB_HOST` | `localhost` | Database host |
| `DB_PORT` | `5432` | Database port |
| `DB_NAME` | `eth_db` | Database name |
| `ETH_RPC_URL` | `http://127.0.0.1:8545` | Ethereum RPC endpoint |
| `ETH_WS_URL` | `ws://127.0.0.1:8546` | Ethereum WebSocket endpoint |
| `ZMQ_POOL_SUB` | `tcp://localhost:5557` | Pool updates subscriber |
| `ZMQ_POOL_REQ` | `tcp://localhost:5558` | Pool data request |
| `ZMQ_EVENT_PUB` | `tcp://localhost:5559` | Market events publisher |

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Run integration test with pool subscriber
cargo run --example test_pool_subscriber

# Test transaction simulation
cargo run --example test_tx_simulator

# Monitor WebSocket performance
cargo run --bin websocket_latency_test

# Test decision engine thresholds
cargo test decision_engine -- --nocapture
```

## 🚨 Production Deployment

### System Requirements
- 4+ CPU cores (8 recommended)
- 8GB RAM minimum (16GB recommended)
- SSD storage for logs
- Local Reth node with <50ms latency
- Network: 1Gbps+ for mempool traffic

### Deployment Steps
```bash
# Build optimized binary
cargo build --release --bin mempool_scam_monitor

# Create systemd service
sudo tee /etc/systemd/system/mempool-monitor.service << EOF
[Unit]
Description=Mempool Market Monitor
After=network.target postgresql.service

[Service]
Type=simple
User=eth
Environment="RUST_LOG=info"
Environment="MEMPOOL_LOG_DIR=/var/log/mempool"
ExecStart=/usr/local/bin/mempool_scam_monitor
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl enable mempool-monitor
sudo systemctl start mempool-monitor
```

### Monitoring
```bash
# Check service status
sudo systemctl status mempool-monitor

# View logs
journalctl -u mempool-monitor -f

# Check event detection
tail -f /var/log/mempool/market_events_*.log

# Database queries
psql -U postgres -d eth_db -c "
SELECT event_type, COUNT(*), AVG(confidence) 
FROM mempool_market_events 
WHERE detection_time > NOW() - INTERVAL '1 hour'
GROUP BY event_type;"

# Performance metrics
grep "End-to-end" /var/log/mempool/processing_times_*.log | \
  awk -F',' '{sum+=$3; count++} END {print "Avg:", sum/count, "ms"}'
```

## ⚡ Performance Optimizations

1. **Parallel Processing**
   - Concurrent `debug_traceCall` for multiple transactions
   - Batch WebSocket message processing
   - Lock-free pool cache reads

2. **Smart Filtering**
   - Only simulate transactions to DEX/DeFi contracts
   - Skip low-value transactions (<0.01 ETH)
   - Priority queue for high-value TXs

3. **Caching Strategy**
   - In-memory pool state cache (2000 pools)
   - 60-second staleness threshold
   - Lazy eviction of inactive pools

4. **Resource Management**
   - Connection pooling for RPC calls
   - Bounded transaction queue (10k max)
   - Automatic log rotation

## 🐛 Troubleshooting

### Common Issues

1. **"State pruned" errors**
   ```bash
   # Ensure Reth has full state
   reth db stats
   # If pruned, resync with: --full
   ```

2. **High latency spikes**
   ```bash
   # Check Reth performance
   curl -X POST http://127.0.0.1:8545 \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"eth_syncing","params":[],"id":1}'
   ```

3. **No events detected**
   ```bash
   # Verify pool subscriber connection
   nc -zv localhost 5557
   # Check pool count
   grep "Pool count" /var/log/mempool/mempool_monitor.log
   ```

### Debug Commands
```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin mempool_scam_monitor

# Trace specific module
RUST_LOG=mempool_processor::decision_engine=trace cargo run

# Performance profiling
cargo build --release --features profiling
perf record -g target/release/mempool_scam_monitor
perf report
```

## 📚 Additional Documentation

- Architecture Details: `docs/ARCHITECTURE.md`
- API Reference: `cargo doc --open`
- Performance Analysis: `docs/PERFORMANCE.md`
- Integration Guide: `docs/INTEGRATION.md`

## 🔄 Recent Updates

- **2025-06-16**: Refactored to Decision Engine with multiple event types
- **2025-06-16**: Added token reserve tracking and supply monitoring
- **2025-06-16**: Implemented ZMQ publisher for eth_kartal integration
- **2025-06-15**: Removed REVM, switched to debug_traceCall
- **2025-06-15**: Added comprehensive timing measurements

---

**Production Status**: ✅ ACTIVE - Processing mainnet transactions with multi-category market event detection.