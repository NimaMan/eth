# Mempool Signal Detection System

## Overview

The Mempool Signal Detection System is a high-performance, real-time monitoring service that detects potential scam transactions and liquidity drains in the Ethereum mempool BEFORE they are confirmed on-chain. This gives traders and protocols crucial seconds to react and protect their positions.

## Architecture

```
┌─────────────────────┐     ┌─────────────────────┐     ┌─────────────────────┐
│   Reth Node (IPC)   │────▶│  Signal Detector    │────▶│   Alert Publisher   │
│  /tmp/reth.ipc      │     │  (This Service)     │     │   tcp://*:5559     │
└─────────────────────┘     └─────────────────────┘     └─────────────────────┘
           │                            │                            │
           │                            ▼                            ▼
           │                 ┌─────────────────────┐     ┌─────────────────────┐
           │                 │   Pool Subscriber   │     │   Trading Systems   │
           └────────────────▶│  tcp://localhost:   │     │   (eth_kartal)      │
                            │      5557/5558       │     └─────────────────────┘
                            └─────────────────────┘
```

## Key Features

### 1. Ultra-Fast Transaction Detection
- **Sub-10μs latency** from mempool to detection using IPC
- Non-blocking I/O for maximum throughput
- Processes 150-200 transactions per second

### 2. Liquidity Removal Detection
Identifies transactions that remove liquidity from DEX pools by checking function signatures:
- `0x02751cec` - removeLiquidityETH
- `0xbaa2abde` - removeLiquidity
- `0xaf2979eb` - removeLiquidityETHSupportingFeeOnTransferTokens
- `0x5b0d5984` - removeLiquidityETHWithPermit
- `0xded9382a` - removeLiquidityETHWithPermitSupportingFeeOnTransferTokens

### 3. Scam Detection Algorithm
Detects potential scams based on:
- **ETH Drain Percentage**: Transactions removing >50% of pool ETH
- **Minimum ETH Threshold**: Pools dropping below 0.01 ETH
- **Token Supply Anomalies**: Sudden massive token mints
- **Price Impact**: Extreme price movements in single transaction

### 4. Real-Time Pool State Tracking
- Subscribes to Python pool publisher for current reserves
- Maintains in-memory cache of pool states
- Filters pools below ETH threshold

### 5. Transaction Simulation
- Uses debug_traceCall for accurate state prediction
- Calculates exact pool effects before execution
- Provides confidence scores for alerts

## Running the Service

### Basic Usage
```bash
cd /home/nima/code/crypto/rust/mempool_processor
cargo run --bin mempool_signal_detector --release
```

### With Custom Parameters
```bash
cargo run --bin mempool_signal_detector --release -- \
  --eth-rpc-url http://localhost:8545 \
  --ipc-path /tmp/reth.ipc \
  --pool-zmq-address tcp://localhost:5557 \
  --eth-threshold 0.01 \
  --percentage-threshold 0.5 \
  --enable-publisher \
  --verbose
```

### Environment Variables
```bash
export ETH_RPC_URL=http://localhost:8545
export IPC_PATH=/tmp/reth.ipc
export POOL_ZMQ_ADDRESS=tcp://localhost:5557
export DB_HOST=localhost
export DB_PORT=5432
export DB_NAME=eth_db
export DB_USER=postgres
export DB_PASSWORD=postgres
export ENABLE_PUBLISHER=true
export ALERT_ZMQ_ADDRESS=tcp://*:5559
```

## Configuration

### Command Line Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `--eth-rpc-url` | http://localhost:8545 | Ethereum JSON-RPC endpoint |
| `--ipc-path` | /tmp/reth.ipc | IPC socket path for mempool access |
| `--pool-zmq-address` | tcp://localhost:5557 | ZMQ address for pool updates |
| `--eth-threshold` | 0.01 | Minimum ETH in pool (scam if below) |
| `--percentage-threshold` | 0.5 | Drain percentage threshold (50%) |
| `--enable-publisher` | false | Enable ZMQ alert publishing |
| `--alert-zmq-address` | tcp://*:5559 | ZMQ publisher endpoint |
| `--verbose` | false | Enable debug logging |

### Database Configuration

| Argument | Default | Description |
|----------|---------|-------------|
| `--db-host` | localhost | PostgreSQL host |
| `--db-port` | 5432 | PostgreSQL port |
| `--db-name` | eth_db | Database name |
| `--db-user` | postgres | Database user |
| `--db-password` | postgres | Database password |

## Detection Flow

### 1. Transaction Reception
```rust
// Receive from IPC with <10μs latency
let tx = ipc_client.get_transactions(100).await?;
```

### 2. Initial Filtering
```rust
// Check if it's a liquidity removal
if let Some(function_name) = is_liquidity_removal(&tx.input_data) {
    // Priority processing for potential scams
}
```

### 3. Pool State Lookup
```rust
// Get current pool reserves from cache
if let Some(pool_state) = pool_cache.get_pool(&pool_address) {
    // Pool found with current reserves
}
```

### 4. Transaction Simulation
```rust
// Simulate transaction effects
let simulation = simulator.simulate_transaction(&tx).await?;
```

### 5. Scam Detection
```rust
// Check for scam conditions
if eth_drain_percent >= 50.0 || new_eth_reserve < 0.01 {
    // SCAM DETECTED!
}
```

### 6. Alert Publishing
```rust
// Publish alert to subscribers
publisher.publish_alert(MarketEvent {
    event_type: EventType::ScamAlert,
    severity: Severity::Critical,
    // ... details
}).await?;
```

## Alert Format

Alerts are published as JSON messages via ZMQ:

```json
{
  "event_type": "ScamAlert",
  "severity": "Critical",
  "confidence": 0.95,
  "tx_hash": "0x...",
  "pool_address": "0x...",
  "token_address": "0x...",
  "metrics": {
    "eth_change": -45.5,
    "eth_percent": -91.0,
    "new_eth_reserve": 4.5,
    "token_change": 1000000.0
  },
  "detection_time": 1234567890.123,
  "block_number": 18500000,
  "details": "Critical liquidity drain detected: 91% ETH removal"
}
```

## Performance Metrics

### Latency Breakdown
- **IPC Reception**: 2-7μs
- **Function Detection**: <1μs  
- **Pool Lookup**: <1μs
- **Simulation**: 50-200ms (RPC dependent)
- **Alert Publishing**: <1ms

### Throughput
- **Transactions/sec**: 150-200
- **Simulations/sec**: 5-20 (limited by RPC)
- **Alerts/sec**: No limit

## Monitoring

### Log Files
```
/home/nima/code/crypto/logs/mempool/signal_engine_full_tx_ipc.log
```

### Key Metrics to Monitor
1. **Detection Latency**: Should stay <10μs
2. **Simulation Queue**: Should not grow unbounded
3. **Pool Cache Hit Rate**: Should be >90%
4. **Alert Count**: Spike indicates potential attack

### Health Checks
```bash
# Check if service is running
ps aux | grep mempool_signal_detector

# Check latest alerts
tail -f /home/nima/code/crypto/logs/mempool/signal_engine_full_tx_ipc.log | grep SCAM

# Monitor performance
grep "Performance stats" /home/nima/code/crypto/logs/mempool/signal_engine_full_tx_ipc.log
```

## Integration

### Subscribing to Alerts

Python example:
```python
import zmq

context = zmq.Context()
subscriber = context.socket(zmq.SUB)
subscriber.connect("tcp://localhost:5559")
subscriber.setsockopt_string(zmq.SUBSCRIBE, "")

while True:
    message = subscriber.recv_string()
    alert = json.loads(message)
    if alert['severity'] == 'Critical':
        # React to scam alert
        protect_positions(alert['pool_address'])
```

### Database Schema

Alerts are stored in PostgreSQL:
```sql
CREATE TABLE scam_alerts (
    id SERIAL PRIMARY KEY,
    tx_hash VARCHAR(66) NOT NULL,
    pool_address VARCHAR(42) NOT NULL,
    token_address VARCHAR(42) NOT NULL,
    eth_drained DECIMAL(18,6),
    percentage_drained DECIMAL(5,2),
    detection_time TIMESTAMP,
    block_number BIGINT,
    confidence DECIMAL(3,2)
);
```

## Troubleshooting

### Service Won't Start
1. Check Reth node is running: `ps aux | grep reth`
2. Verify IPC socket exists: `ls -la /tmp/reth.ipc`
3. Check Python pool publisher: `nc -zv localhost 5557`

### No Alerts Generated
1. Verify pool data is fresh: Check Python publisher logs
2. Ensure simulation RPC works: `curl http://localhost:8545`
3. Check thresholds aren't too high

### High Latency
1. Check IPC connection: Should be <10μs
2. Verify no blocking operations in hot path
3. Monitor RPC response times

## Security Considerations

1. **IPC Socket**: Ensure proper permissions on `/tmp/reth.ipc`
2. **ZMQ Ports**: Firewall ports 5557-5559 from external access
3. **Database**: Use strong passwords, limit connections
4. **Alerts**: Validate all alerts before taking action

## Future Enhancements

1. **Machine Learning**: Detect complex scam patterns
2. **MEV Protection**: Front-run protection for users
3. **Cross-Chain**: Monitor multiple chains simultaneously
4. **Historical Analysis**: Learn from past scams