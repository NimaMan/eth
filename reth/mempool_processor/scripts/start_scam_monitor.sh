#!/bin/bash
# Start the scam monitor in background with logging

LOG_DIR="/home/nima/code/crypto/logs/scam_monitor"
mkdir -p "$LOG_DIR"

LOG_FILE="$LOG_DIR/scam_monitor_$(date +%Y%m%d_%H%M%S).log"

echo "🚀 Starting Mempool Scam Monitor"
echo "📁 Logs will be written to: $LOG_FILE"
echo "👁️  Monitoring pool: 0xACE9FEee4072aD385d02C8A6c4b69c66D72F64D6"
echo ""
echo "To view logs in real-time:"
echo "  tail -f $LOG_FILE"
echo ""
echo "To check for detected scams:"
echo "  grep 'SCAM DETECTED' $LOG_FILE"
echo ""

# Start the real mempool monitor in background
nohup python3 /home/nima/code/crypto/rust/mempool_processor/mempool_scam_monitor.py > "$LOG_FILE" 2>&1 &
PID=$!

echo "✅ Monitor started with PID: $PID"
echo ""
echo "To stop the monitor:"
echo "  kill $PID"

# Save PID for easy stopping
echo $PID > "$LOG_DIR/monitor.pid"

# Show initial output
sleep 2
echo "Initial output:"
echo "---"
head -20 "$LOG_FILE"