# Real-Time Scam Detection Service

## Overview

The Real-Time Scam Detection Service monitors Ethereum's mempool for potential scam transactions, particularly focusing on liquidity pool drains and rug pulls. It uses advanced transaction simulation and state change analysis to detect malicious activities before they are mined.

## Features

- **Real-time Mempool Monitoring**: WebSocket subscription for instant transaction notifications
- **Pool State Tracking**: Integrates with Python pool publisher for up-to-date liquidity data
- **Advanced Simulation**: Uses debug_traceCall for comprehensive state change analysis
- **Multi-level Detection**:
  - Direct pool interactions
  - Router-based interactions
  - Complex DeFi transactions
- **Database Logging**: Stores all detected scams for analysis
- **Low Latency**: Sub-10ms detection from mempool arrival

## Prerequisites

1. **Reth Node**: Running with WebSocket and debug API enabled
   ```bash
   # Check if running
   curl http://127.0.0.1:8545 -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
   ```

2. **Pool Publisher**: Python service publishing pool data
   ```bash
   # Check if running
   nc -z localhost 5557 && echo "Pool publisher is running"
   ```

3. **PostgreSQL Database**: For storing scam alerts
   ```bash
   # Test connection
   PGPASSWORD=postgres psql -h localhost -U postgres -d eth_db -c "SELECT 1"
   ```

## Quick Start

```bash
# Using the startup script
./scripts/start_realtime_scam_detection.sh

# Or manually with custom parameters
cargo run --release --bin realtime_scam_detection_service -- \
    --eth-rpc-url http://127.0.0.1:8545 \
    --eth-ws-url ws://127.0.0.1:8546 \
    --pool-zmq-address tcp://localhost:5557 \
    --db-host localhost \
    --db-name eth_db \
    --eth-threshold 0.01 \
    --percentage-threshold 0.95 \
    --verbose
```

## Configuration

### Command Line Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `--eth-rpc-url` | `http://localhost:8545` | Ethereum RPC endpoint |
| `--eth-ws-url` | `ws://localhost:8546` | WebSocket endpoint for mempool |
| `--pool-zmq-address` | `tcp://localhost:5557` | ZeroMQ address for pool updates |
| `--db-host` | `localhost` | PostgreSQL host |
| `--db-port` | `5432` | PostgreSQL port |
| `--db-name` | `eth_db` | Database name |
| `--db-user` | `postgres` | Database user |
| `--db-password` | `postgres` | Database password |
| `--eth-threshold` | `0.01` | Minimum ETH drain to consider |
| `--percentage-threshold` | `0.95` | Percentage drain threshold |
| `--verbose` | `false` | Enable verbose logging |

### Environment Variables

All command line arguments can also be set via environment variables:
- `ETH_RPC_URL`
- `ETH_WS_URL`
- `POOL_ZMQ_ADDRESS`
- `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASSWORD`

## Detection Logic

### Scam Types Detected

1. **Rug Pulls**: >95% liquidity removal from pools
2. **Large Drains**: Significant ETH removal (configurable threshold)
3. **Honeypots**: Pools that prevent selling (future feature)

### Detection Process

1. **Transaction Arrival**: WebSocket notification with timestamp
2. **Simulation**: debug_traceCall to get all state changes
3. **Analysis**: 
   - Check if any pools are affected
   - Calculate ETH/token balance changes
   - Determine drain percentage
4. **Alert**: If thresholds exceeded, log to database and console

## Performance Metrics

- **Discovery Latency**: Time from transaction broadcast to local detection
- **Processing Latency**: Time to simulate and analyze
- **Total Latency**: End-to-end detection time (typically <10ms)

## Output

### Console Output
```
🚨 SCAM DETECTED in 0x123... (7ms from mempool arrival)
   Pool 0x456... drained 15.5 ETH (98.2%)
```

### Database Schema
```sql
CREATE TABLE scam_alerts (
    id SERIAL PRIMARY KEY,
    transaction_hash VARCHAR(66) NOT NULL,
    pool_address VARCHAR(42) NOT NULL,
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    eth_amount NUMERIC(20, 8),
    percentage NUMERIC(5, 2),
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    block_number BIGINT,
    details JSONB
);
```

## Monitoring

Check service health:
```bash
# View logs
tail -f /home/nima/code/crypto/logs/mempool/realtime_scam_detection.log

# Check database for alerts
psql -h localhost -U postgres -d eth_db -c \
  "SELECT * FROM scam_alerts ORDER BY detected_at DESC LIMIT 10;"
```

## Troubleshooting

### No Transactions Processed
- Check WebSocket connection: `wscat -c ws://localhost:8546`
- Verify mempool subscription is active

### No Pools Detected
- Ensure pool publisher is running
- Check ZMQ connection: `nc -z localhost 5557`

### High Latency
- Check node performance
- Verify debug API is enabled
- Consider reducing simulation complexity

## Integration

The service can be integrated with:
- Alert systems (Discord, Telegram)
- Trading bots for protection
- Analytics dashboards
- MEV protection systems