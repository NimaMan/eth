#!/bin/bash
# Comprehensive runner for live pool detection test
# Supports both test mode and live system integration

set -e  # Exit on error

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
MEMPOOL_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

# Parse command line arguments
USE_LIVE=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --live)
            USE_LIVE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --live     Use live pool data if available"
            echo "  --verbose  Show verbose output"
            echo "  --help     Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "=== Live Pool Detection Test Runner ==="
echo

# Check prerequisites
if ! command -v python3 &> /dev/null; then
    echo "❌ Error: python3 not found"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo not found"
    exit 1
fi

# Activate Python environment
echo "🐍 Activating Python environment..."
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw

# Determine which publisher to use
if [ "$USE_LIVE" = true ]; then
    PUBLISHER_SCRIPT="pool_publisher_live.py"
    echo "📡 Using live pool data mode"
else
    PUBLISHER_SCRIPT="pool_publisher_test.py"
    echo "🧪 Using test pool data mode"
fi

# Check if publisher script exists
if [ ! -f "$SCRIPT_DIR/$PUBLISHER_SCRIPT" ]; then
    echo "❌ Error: $PUBLISHER_SCRIPT not found in $SCRIPT_DIR"
    exit 1
fi

# Build Rust test if needed
if [ ! -f "$MEMPOOL_DIR/target/release/test_live_pool_detection" ]; then
    echo "📦 Building Rust test binary..."
    cd "$MEMPOOL_DIR"
    cargo build --release --bin test_live_pool_detection
fi

# Start Python pool publisher in background
echo "🚀 Starting Python pool publisher..."
if [ "$VERBOSE" = true ]; then
    python3 "$SCRIPT_DIR/$PUBLISHER_SCRIPT" &
else
    python3 "$SCRIPT_DIR/$PUBLISHER_SCRIPT" > /tmp/pool_publisher.log 2>&1 &
fi
PYTHON_PID=$!
echo "   Pool publisher PID: $PYTHON_PID"

# Give publisher time to start
echo "⏳ Waiting 3 seconds for publisher initialization..."
sleep 3

# Check if publisher is still running
if ! kill -0 $PYTHON_PID 2>/dev/null; then
    echo "❌ Pool publisher failed to start!"
    if [ "$VERBOSE" = false ]; then
        echo "   Check /tmp/pool_publisher.log for details"
        tail -20 /tmp/pool_publisher.log
    fi
    exit 1
fi

# Run the Rust test
echo
echo "🦀 Starting Rust pool detection test..."
echo "=" * 80
cd "$MEMPOOL_DIR"

# Set log level based on verbose flag
if [ "$VERBOSE" = true ]; then
    export RUST_LOG=debug
else
    export RUST_LOG=info
fi

./target/release/test_live_pool_detection

# Store test result
TEST_RESULT=$?

# Cleanup
echo
echo "🧹 Cleaning up..."
kill $PYTHON_PID 2>/dev/null || true
wait $PYTHON_PID 2>/dev/null || true

# Clean up log file
if [ "$VERBOSE" = false ] && [ -f /tmp/pool_publisher.log ]; then
    rm -f /tmp/pool_publisher.log
fi

# Summary
echo
echo "=== Test Complete ==="
if [ $TEST_RESULT -eq 0 ]; then
    echo "✅ Test passed successfully!"
    
    # Provide next steps
    echo
    echo "📝 Next Steps:"
    echo "1. Check if pool interactions were detected"
    echo "2. Verify address checksumming is working"
    echo "3. Monitor production service with: cargo run --release --bin scam_detection_service"
    
    if [ "$USE_LIVE" = false ]; then
        echo
        echo "💡 Tip: Run with --live flag to use real pool data if available"
    fi
else
    echo "❌ Test failed with exit code: $TEST_RESULT"
    echo
    echo "🔍 Troubleshooting:"
    echo "1. Ensure Reth node is running at localhost:8545"
    echo "2. Check that Python pool publisher started correctly"
    echo "3. Run with --verbose flag for detailed output"
fi

exit $TEST_RESULT