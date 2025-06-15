#!/bin/bash

echo "=== Mempool Scam Monitor Log Locations ==="
echo
echo "1. Scam Detection Log File:"
echo "   Location: /home/nima/code/crypto/logs/mempool/scam_detections.log"
if [ -f "/home/nima/code/crypto/logs/mempool/scam_detections.log" ]; then
    echo "   Status: EXISTS"
    echo "   Size: $(ls -lh /home/nima/code/crypto/logs/mempool/scam_detections.log | awk '{print $5}')"
    echo "   Last 5 lines:"
    tail -5 /home/nima/code/crypto/logs/mempool/scam_detections.log
else
    echo "   Status: NOT CREATED YET (no scams detected)"
fi

echo
echo "2. Console Output:"
echo "   When running: cargo run --release --bin mempool_scam_monitor"
echo "   Shows:"
echo "   - Real-time transaction processing"
echo "   - Timing statistics every 100 transactions"
echo "   - Scam alerts when detected"

echo
echo "3. To run the service and see logs:"
echo "   cargo run --release --bin mempool_scam_monitor"

echo
echo "4. To run in background and save console logs:"
echo "   cargo run --release --bin mempool_scam_monitor 2>&1 | tee mempool_monitor.log"