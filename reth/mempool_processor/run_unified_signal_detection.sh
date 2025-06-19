#!/bin/bash

# Unified Mempool Signal Detection Runner Script
# Supports both single (low-latency) and batch (high-throughput) modes

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default configuration
MODE="single"
DURATION=300
MIN_TX_VALUE=0.001
VERBOSE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --batch)
            MODE="batch"
            shift
            ;;
        --single)
            MODE="single"
            shift
            ;;
        --duration)
            DURATION="$2"
            shift 2
            ;;
        --min-value)
            MIN_TX_VALUE="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --mode MODE         Operation mode: 'single' or 'batch' (default: single)"
            echo "  --single            Use single transaction mode (low latency)"
            echo "  --batch             Use batch processing mode (high throughput)"
            echo "  --duration SECONDS  How long to run (default: 300)"
            echo "  --min-value ETH     Minimum transaction value in ETH (default: 0.001)"
            echo "  --verbose, -v       Enable verbose logging"
            echo "  --help, -h          Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 --single                    # Low latency mode"
            echo "  $0 --batch --duration 600      # High throughput for 10 minutes"
            echo "  $0 --mode batch --min-value 0.01 --verbose"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}🚀 Starting Unified Mempool Signal Detection${NC}"
echo "============================================"
echo -e "Mode: ${YELLOW}$MODE${NC}"
echo -e "Duration: ${YELLOW}$DURATION seconds${NC}"
echo -e "Min TX Value: ${YELLOW}$MIN_TX_VALUE ETH${NC}"
echo ""

# Check if Reth is running
if ! pgrep -x "reth" > /dev/null; then
    echo -e "${RED}❌ Error: Reth node is not running${NC}"
    echo "Please start Reth first"
    exit 1
fi

# Check IPC socket
if [ ! -S "/tmp/reth.ipc" ]; then
    echo -e "${RED}❌ Error: Reth IPC socket not found at /tmp/reth.ipc${NC}"
    exit 1
fi

# Build the project
echo -e "${YELLOW}Building project...${NC}"
cd /home/nima/code/crypto/rust/mempool_processor
cargo build --release --bin mempool_signal_detection_unified

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Prepare command
CMD="./target/release/mempool_signal_detection_unified"
CMD="$CMD --mode $MODE"
CMD="$CMD --duration $DURATION"
CMD="$CMD --min-tx-value $MIN_TX_VALUE"

# Mode-specific configuration
if [ "$MODE" = "batch" ]; then
    echo -e "${YELLOW}Batch mode configuration:${NC}"
    echo "  Batch size: 20 transactions"
    echo "  Batch timeout: 30ms"
    echo "  Parallel connections: 3"
    echo ""
fi

if [ "$VERBOSE" = true ]; then
    CMD="$CMD --verbose"
fi

# Create log directory if it doesn't exist
mkdir -p /home/nima/code/crypto/logs/mempool

# Run the unified signal detection
echo -e "${GREEN}Starting signal detection...${NC}"
echo "Command: $CMD"
echo ""

# Execute with real-time output
$CMD

# Check exit status
if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ Signal detection completed successfully${NC}"
else
    echo ""
    echo -e "${RED}❌ Signal detection failed${NC}"
    exit 1
fi