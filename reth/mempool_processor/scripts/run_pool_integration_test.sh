#!/bin/bash
# Integration test runner for pool level communication

echo "🔄 Pool Level Integration Test Runner"
echo "===================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to check if a process is running
check_process() {
    if pgrep -f "$1" > /dev/null; then
        return 0
    else
        return 1
    fi
}

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    
    # Kill Python test if running
    if [ ! -z "$PYTHON_PID" ]; then
        kill $PYTHON_PID 2>/dev/null
        echo "Stopped Python integration test"
    fi
    
    # Kill Rust test if running
    if [ ! -z "$RUST_PID" ]; then
        kill $RUST_PID 2>/dev/null
        echo "Stopped Rust integration test"
    fi
    
    exit 0
}

# Set trap for cleanup
trap cleanup EXIT INT TERM

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from mempool_processor directory${NC}"
    exit 1
fi

# Activate conda environment
echo -e "${YELLOW}Activating conda environment...${NC}"
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw

# Build Rust example
echo -e "\n${YELLOW}Building Rust integration test...${NC}"
cargo build --example integration_test_pool_subscriber
if [ $? -ne 0 ]; then
    echo -e "${RED}Failed to build Rust example${NC}"
    exit 1
fi

# Start Python integration test in background
echo -e "\n${GREEN}Starting Python pool publisher...${NC}"
python tests/integration_test_pool_levels.py &
PYTHON_PID=$!
echo "Python PID: $PYTHON_PID"

# Wait for Python to start
sleep 2

# Check if Python is still running
if ! kill -0 $PYTHON_PID 2>/dev/null; then
    echo -e "${RED}Python integration test failed to start${NC}"
    exit 1
fi

# Start Rust integration test
echo -e "\n${GREEN}Starting Rust pool subscriber...${NC}"
cargo run --example integration_test_pool_subscriber &
RUST_PID=$!
echo "Rust PID: $RUST_PID"

# Monitor both processes
echo -e "\n${GREEN}Integration test running. Press Ctrl+C to stop.${NC}"
echo -e "${YELLOW}Watch the output from both processes above.${NC}"
echo ""

# Wait for either process to exit or user interrupt
while kill -0 $PYTHON_PID 2>/dev/null && kill -0 $RUST_PID 2>/dev/null; do
    sleep 1
done

# If we get here, one process died
if ! kill -0 $PYTHON_PID 2>/dev/null; then
    echo -e "\n${RED}Python process exited unexpectedly${NC}"
fi

if ! kill -0 $RUST_PID 2>/dev/null; then
    echo -e "\n${RED}Rust process exited unexpectedly${NC}"
fi