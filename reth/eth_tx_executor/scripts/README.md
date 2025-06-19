# Scripts Documentation

## Overview
This directory contains utility scripts for development, testing, and deployment of ETH Kartal.

## Development Scripts

### `start_dev.sh`
Starts the development environment with all required services.

```bash
#!/bin/bash
# Starts ETH Kartal in development mode with local services

# Start local services
echo "Starting local services..."
# Ensure Reth is running
# Start mempool processor if needed

# Set development environment
export ETH_KARTAL_ENVIRONMENT=development
export ETH_KARTAL_LOG_LEVEL=debug
export RUST_BACKTRACE=1

# Start with development config
cargo run -- --config config/dev.toml
```

### `test_integration.sh`
Runs integration tests against local/testnet environment.

```bash
#!/bin/bash
# Run integration tests

# Start test environment
./scripts/start_test_env.sh

# Run tests
cargo test --features integration_tests

# Generate coverage report
cargo tarpaulin --out Html
```

## Testing Scripts

### `simulate_alert.sh`
Sends test alerts to the system for development.

```bash
#!/bin/bash
# Send test scam alert via ZMQ

python3 - <<EOF
import zmq
import json
import time

context = zmq.Context()
socket = context.socket(zmq.PUB)
socket.connect("tcp://localhost:5558")

alert = {
    "alert_id": "test_" + str(int(time.time())),
    "severity": "Critical",
    "tx_hash": "0x1234...",
    "pool_address": "0xABCD...",
    "token_address": "0xDEFG...",
    "eth_change_percent": -85.0,
    "confidence": 0.95
}

socket.send_json(alert)
print(f"Sent test alert: {alert}")
EOF
```

### `check_health.sh`
Monitors system health and performance.

```bash
#!/bin/bash
# Check system health

# Check if process is running
if pgrep -f "eth_kartal" > /dev/null; then
    echo "✅ ETH Kartal is running"
else
    echo "❌ ETH Kartal is not running"
    exit 1
fi

# Check metrics endpoint
curl -s http://localhost:9090/metrics > /dev/null
if [ $? -eq 0 ]; then
    echo "✅ Metrics endpoint is healthy"
else
    echo "❌ Metrics endpoint is down"
fi

# Check recent logs for errors
if tail -n 1000 /var/log/kartal/app.log | grep -i "error" > /dev/null; then
    echo "⚠️  Recent errors found in logs"
else
    echo "✅ No recent errors"
fi
```

## Deployment Scripts

### `deploy_production.sh`
Deploys to production with safety checks.

```bash
#!/bin/bash
# Production deployment script

set -e  # Exit on error

# Pre-deployment checks
echo "Running pre-deployment checks..."

# 1. Run tests
cargo test --release

# 2. Check configuration
cargo run -- --config config/prod.toml --validate-only

# 3. Build release binary
cargo build --release

# 4. Backup current version
cp /usr/local/bin/eth_kartal /usr/local/bin/eth_kartal.backup

# 5. Deploy new version
sudo systemctl stop eth_kartal
cp target/release/eth_kartal /usr/local/bin/
sudo systemctl start eth_kartal

# 6. Verify deployment
sleep 5
./scripts/check_health.sh
```

### `rollback.sh`
Emergency rollback to previous version.

```bash
#!/bin/bash
# Emergency rollback

echo "⚠️  Rolling back ETH Kartal..."

# Stop service
sudo systemctl stop eth_kartal

# Restore backup
sudo cp /usr/local/bin/eth_kartal.backup /usr/local/bin/eth_kartal

# Start service
sudo systemctl start eth_kartal

# Verify
./scripts/check_health.sh
```

## Monitoring Scripts

### `watch_performance.sh`
Real-time performance monitoring.

```bash
#!/bin/bash
# Monitor performance metrics

watch -n 1 '
echo "=== ETH Kartal Performance ==="
echo
curl -s http://localhost:9090/metrics | grep -E "(alerts_processed|trades_executed|profit_total|circuit_breaker_status)"
echo
echo "=== Recent Alerts ==="
tail -n 5 /var/log/kartal/alerts.log
echo
echo "=== System Resources ==="
ps aux | grep eth_kartal | grep -v grep
'
```

### `export_metrics.sh`
Export metrics for analysis.

```bash
#!/bin/bash
# Export metrics to CSV

DATE=$(date +%Y%m%d_%H%M%S)
OUTPUT="metrics_${DATE}.csv"

echo "timestamp,alerts_processed,trades_executed,success_rate,total_profit_eth" > $OUTPUT

while true; do
    METRICS=$(curl -s http://localhost:9090/metrics)
    # Parse and append metrics
    echo "$(date +%s),$METRICS" >> $OUTPUT
    sleep 60
done
```

## Utility Scripts

### `generate_config.sh`
Generate configuration from template.

```bash
#!/bin/bash
# Generate environment-specific config

ENV=$1
if [ -z "$ENV" ]; then
    echo "Usage: $0 <environment>"
    exit 1
fi

# Copy template
cp config/template.toml config/${ENV}.toml

# Replace placeholders
sed -i "s/{{ENVIRONMENT}}/${ENV}/g" config/${ENV}.toml
sed -i "s/{{RPC_ENDPOINT}}/${ETH_RPC_URL}/g" config/${ENV}.toml

echo "Generated config//${ENV}.toml"
```

### `analyze_logs.sh`
Analyze logs for patterns and issues.

```bash
#!/bin/bash
# Log analysis tool

LOG_FILE=${1:-/var/log/kartal/app.log}

echo "=== Log Analysis for $LOG_FILE ==="
echo
echo "Error Summary:"
grep -i "error" $LOG_FILE | cut -d' ' -f4- | sort | uniq -c | sort -nr | head -10
echo
echo "Alert Types:"
grep "alert_type" $LOG_FILE | jq -r .alert_type | sort | uniq -c
echo
echo "Trade Success Rate:"
SUCCESS=$(grep "trade_result.*success" $LOG_FILE | wc -l)
FAILED=$(grep "trade_result.*failed" $LOG_FILE | wc -l)
TOTAL=$((SUCCESS + FAILED))
if [ $TOTAL -gt 0 ]; then
    RATE=$((SUCCESS * 100 / TOTAL))
    echo "Success: $SUCCESS/$TOTAL (${RATE}%)"
fi
```

## Development Workflow

1. **Start Development**
   ```bash
   ./scripts/start_dev.sh
   ```

2. **Test Changes**
   ```bash
   ./scripts/test_integration.sh
   ./scripts/simulate_alert.sh
   ```

3. **Monitor Performance**
   ```bash
   ./scripts/watch_performance.sh
   ```

4. **Deploy to Production**
   ```bash
   ./scripts/deploy_production.sh
   ```

5. **If Issues Arise**
   ```bash
   ./scripts/rollback.sh
   ```

## Script Best Practices

1. Always use `set -e` for deployment scripts
2. Add plenty of echo statements for visibility
3. Check return codes for critical operations
4. Keep backups before making changes
5. Log all script actions with timestamps
6. Make scripts idempotent where possible