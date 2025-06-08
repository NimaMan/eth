#!/bin/bash
# Runner script for live pool detection test
# This script starts the Python pool publisher and runs the Rust test

set -e  # Exit on error

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
MEMPOOL_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "=== Live Pool Detection Test Runner ==="
echo

# Check if Python pool publisher exists
if [ ! -f "$SCRIPT_DIR/pool_publisher_test.py" ]; then
    echo "❌ Error: pool_publisher_test.py not found in $SCRIPT_DIR"
    exit 1
fi

# Activate Python environment
echo "🐍 Activating Python environment..."
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw

# Start Python pool publisher in background
echo "🚀 Starting Python pool publisher..."
python3 "$SCRIPT_DIR/pool_publisher_test.py" &
PYTHON_PID=$!
echo "   Pool publisher PID: $PYTHON_PID"

# Give publisher time to start
echo "⏳ Waiting 2 seconds for publisher initialization..."
sleep 2

# Check if Rust test binary exists
if [ ! -f "$MEMPOOL_DIR/target/release/test_live_pool_detection" ]; then
    echo "📦 Building Rust test binary..."
    cd "$MEMPOOL_DIR"
    cargo build --release --bin test_live_pool_detection
fi

# Run the Rust test
echo
echo "🦀 Starting Rust pool detection test..."
echo "=" * 80
cd "$MEMPOOL_DIR"
./target/release/test_live_pool_detection

# Store test result
TEST_RESULT=$?

# Cleanup Python publisher
echo
echo "🧹 Cleaning up..."
kill $PYTHON_PID 2>/dev/null || true
wait $PYTHON_PID 2>/dev/null || true

# Summary
echo
echo "=== Test Complete ==="
if [ $TEST_RESULT -eq 0 ]; then
    echo "✅ Test passed successfully!"
else
    echo "❌ Test failed with exit code: $TEST_RESULT"
fi

exit $TEST_RESULT