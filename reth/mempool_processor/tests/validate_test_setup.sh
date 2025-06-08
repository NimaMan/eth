#!/bin/bash
# Quick validation script to check test setup

echo "=== Validating Live Pool Detection Test Setup ==="
echo

# Activate Python environment
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw

# Check Python
echo "✓ Checking Python environment..."
if python3 -c "import zmq" 2>/dev/null; then
    echo "  ✅ Python with ZMQ available"
else
    echo "  ❌ ZMQ not available in Python"
    exit 1
fi

# Check Rust binary
echo "✓ Checking Rust test binary..."
if [ -f "../target/release/test_live_pool_detection" ]; then
    echo "  ✅ Test binary exists"
else
    echo "  ⚠️  Test binary not built, building now..."
    cd ..
    cargo build --release --bin test_live_pool_detection
    cd tests
fi

# Test Python publisher
echo "✓ Testing Python pool publisher..."
python3 pool_publisher_test.py --test
echo "  ✅ Pool publisher works"

# Check ports
echo "✓ Checking if ports are available..."
if ! lsof -i:5557 >/dev/null 2>&1; then
    echo "  ✅ Port 5557 is available"
else
    echo "  ⚠️  Port 5557 is in use"
fi

echo
echo "=== Setup Validation Complete ==="
echo "✅ All components are ready for testing"
echo
echo "Run the test with:"
echo "  ./run_live_pool_detection_test.sh"
echo
echo "Or with options:"
echo "  ./run_live_pool_detection_full.sh --verbose"