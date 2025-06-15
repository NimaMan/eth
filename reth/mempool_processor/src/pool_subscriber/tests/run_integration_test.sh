#!/bin/bash

echo "🧪 Pool Subscriber Integration Test"
echo "==================================="
echo ""

# Start Python mock publisher in background
echo "1️⃣  Starting Python mock publisher..."
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw
python src/pool_subscriber/tests/demo_pool_subscriber.py &
PYTHON_PID=$!

# Give it time to start
sleep 1

# Run Rust subscriber test
echo ""
echo "2️⃣  Starting Rust subscriber test..."
echo ""
cargo run --example test_pool_subscriber 2>&1

# Kill Python process
kill $PYTHON_PID 2>/dev/null

echo ""
echo "✅ Integration test completed!"