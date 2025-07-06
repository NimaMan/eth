#!/bin/bash

echo "🧪 Testing Direct Reth Signal Detector"
echo "====================================="

# Test 1: Check help command
echo "1. Testing help command..."
cargo run --release --bin mempool_signal_detector_direct_reth -- --help | head -5

echo ""
echo "2. Starting signal detector with 1 transaction limit..."

# Run with timeout and capture output
timeout 20s cargo run --release --bin mempool_signal_detector_direct_reth -- \
    --max-transactions 1 \
    --verbose \
    2>&1 | tee /tmp/signal_detector_test.log &

DETECTOR_PID=$!

# Wait a bit for initialization
sleep 5

# Check if it's still running
if kill -0 $DETECTOR_PID 2>/dev/null; then
    echo "✅ Signal detector is running (PID: $DETECTOR_PID)"
    echo "📝 Output being logged to /tmp/signal_detector_test.log"
    
    # Wait for it to complete or timeout
    wait $DETECTOR_PID
    echo "📊 Final output:"
    tail -20 /tmp/signal_detector_test.log | grep -E "(initialized|processing|detected|complete)"
else
    echo "❌ Signal detector failed to start"
    echo "📝 Error output:"
    cat /tmp/signal_detector_test.log
fi

echo ""
echo "✅ Test complete!"