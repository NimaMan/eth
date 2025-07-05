# Mempool Processor Scripts

This directory contains operational scripts for running and managing the mempool processor components.

## Directory Structure

```
scripts/
├── signal_detection/     # Signal detection and scam alert system
│   ├── README.md        # Detailed documentation
│   ├── run_signal_detector.sh     # Start the service
│   ├── stop_signal_detector.sh    # Stop the service
│   ├── monitor_alerts.sh          # Monitor alerts in real-time
│   └── mempool_signal_detector.rs # Main service code
└── README.md            # This file
```

## Quick Start

### 1. Start Signal Detection Service

```bash
cd scripts/signal_detection
./run_signal_detector.sh
```

### 2. Monitor Alerts

In another terminal:
```bash
./monitor_alerts.sh
```

### 3. Stop Service

```bash
./stop_signal_detector.sh
```

## Components

### Signal Detection (`signal_detection/`)

Real-time mempool monitoring for scam detection and liquidity drain alerts.

**Key Features:**
- Ultra-fast IPC mempool access (<10μs latency)
- Liquidity removal transaction detection
- Pool state monitoring via ZMQ
- Transaction simulation for impact analysis
- Alert publishing for downstream systems

**Requirements:**
- Running Reth node with IPC enabled
- Python pool publisher on ports 5557/5558
- PostgreSQL database for alert storage

See [signal_detection/README.md](signal_detection/README.md) for detailed documentation.

## Common Operations

### Check Service Status

```bash
# Check if signal detector is running
ps aux | grep mempool_signal_detector

# View recent alerts
grep "SCAM DETECTED" /home/nima/code/crypto/logs/mempool/signal_engine_full_tx_ipc.log | tail -10

# Monitor performance
grep "Performance stats" /home/nima/code/crypto/logs/mempool/signal_engine_full_tx_ipc.log | tail -5
```

### Troubleshooting

#### Service Won't Start
1. Ensure Reth is running: `systemctl status reth` or check process
2. Verify IPC socket: `ls -la /tmp/reth.ipc`
3. Check Python services: `nc -zv localhost 5557`

#### No Alerts Generated
1. Check pool data freshness
2. Verify simulation RPC endpoint
3. Review threshold settings

#### High Resource Usage
1. Check simulation queue depth
2. Monitor transaction throughput
3. Verify no memory leaks

## Environment Variables

```bash
# Core settings
export ETH_RPC_URL=http://localhost:8545
export IPC_PATH=/tmp/reth.ipc

# Pool data source
export POOL_ZMQ_ADDRESS=tcp://localhost:5557

# Alert publishing
export ENABLE_PUBLISHER=true
export ALERT_ZMQ_ADDRESS=tcp://*:5559

# Database
export DB_HOST=localhost
export DB_NAME=eth_db
export DB_USER=postgres
export DB_PASSWORD=postgres

# Detection thresholds
export ETH_THRESHOLD=0.01
export PERCENTAGE_THRESHOLD=0.5
```

## Integration Points

### Subscribing to Alerts

Services can subscribe to real-time alerts via ZMQ:

```python
import zmq
context = zmq.Context()
subscriber = context.socket(zmq.SUB)
subscriber.connect("tcp://localhost:5559")
subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
```

### Database Access

Alerts are stored in PostgreSQL for historical analysis:

```sql
SELECT * FROM scam_alerts 
WHERE detection_time > NOW() - INTERVAL '1 hour'
ORDER BY confidence DESC;
```

## Performance Tuning

### For Maximum Throughput
- Use release builds: `--release`
- Increase channel buffers in code
- Run on dedicated CPU cores
- Use local NVMe for logs

### For Minimum Latency
- Ensure IPC socket is on tmpfs
- Disable verbose logging
- Minimize simulation queue depth
- Use local Reth node

## Security

1. **Network**: Firewall ZMQ ports from external access
2. **IPC**: Restrict socket permissions
3. **Database**: Use strong passwords, SSL connections
4. **Logs**: Rotate and encrypt sensitive data

## Future Scripts

Planned additions:
- `backup_alerts.sh` - Backup alert database
- `performance_report.sh` - Generate performance reports
- `health_check.sh` - Comprehensive health monitoring
- `replay_mempool.sh` - Replay historical mempool data