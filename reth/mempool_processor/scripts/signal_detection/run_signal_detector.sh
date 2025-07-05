#!/bin/bash

# Mempool Signal Detector Launch Script
# 
# This script starts the mempool signal detection service with proper configuration
# and monitoring. It ensures all dependencies are running before starting.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."
LOG_DIR="/home/nima/code/crypto/logs/mempool"
PID_FILE="$LOG_DIR/signal_detector.pid"

# Default parameters (can be overridden by environment)
ETH_RPC_URL="${ETH_RPC_URL:-http://localhost:8545}"
IPC_PATH="${IPC_PATH:-/tmp/reth.ipc}"
POOL_ZMQ_ADDRESS="${POOL_ZMQ_ADDRESS:-tcp://localhost:5557}"
ETH_THRESHOLD="${ETH_THRESHOLD:-0.01}"
PERCENTAGE_THRESHOLD="${PERCENTAGE_THRESHOLD:-0.5}"
ENABLE_PUBLISHER="${ENABLE_PUBLISHER:-true}"

echo -e "${GREEN}Mempool Signal Detector${NC}"
echo "========================"
echo ""

# Function to check if a service is running
check_service() {
    local service_name=$1
    local check_command=$2
    
    echo -n "Checking $service_name... "
    if eval $check_command > /dev/null 2>&1; then
        echo -e "${GREEN}OK${NC}"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        return 1
    fi
}

# Pre-flight checks
echo "Running pre-flight checks:"
echo ""

# Check Reth node
if ! check_service "Reth node" "curl -s -X POST -H 'Content-Type: application/json' --data '{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}' $ETH_RPC_URL"; then
    echo -e "${RED}Error: Reth node is not responding at $ETH_RPC_URL${NC}"
    echo "Please start Reth node first"
    exit 1
fi

# Check IPC socket
if ! check_service "IPC socket" "test -S $IPC_PATH"; then
    echo -e "${RED}Error: IPC socket not found at $IPC_PATH${NC}"
    echo "Please ensure Reth is running with IPC enabled"
    exit 1
fi

# Check Python pool publisher
if ! check_service "Pool publisher" "nc -zv localhost 5557 2>&1 | grep -q succeeded"; then
    echo -e "${YELLOW}Warning: Pool publisher not responding on port 5557${NC}"
    echo "Signal detector will start but may not have pool data"
fi

# Check if already running
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if ps -p $OLD_PID > /dev/null 2>&1; then
        echo -e "${YELLOW}Warning: Signal detector already running (PID: $OLD_PID)${NC}"
        echo -n "Stop existing instance? (y/n): "
        read answer
        if [ "$answer" = "y" ]; then
            echo "Stopping existing instance..."
            kill $OLD_PID
            sleep 2
        else
            echo "Exiting without starting new instance"
            exit 0
        fi
    fi
fi

# Create log directory if needed
mkdir -p "$LOG_DIR"

echo ""
echo "Starting signal detector with:"
echo "  RPC URL: $ETH_RPC_URL"
echo "  IPC Path: $IPC_PATH"
echo "  Pool ZMQ: $POOL_ZMQ_ADDRESS"
echo "  ETH Threshold: $ETH_THRESHOLD"
echo "  Drain Threshold: ${PERCENTAGE_THRESHOLD}%"
echo "  Publisher: $ENABLE_PUBLISHER"
echo ""

# Build the command
CMD="cargo run --bin mempool_signal_detector --release --"
CMD="$CMD --eth-rpc-url $ETH_RPC_URL"
CMD="$CMD --ipc-path $IPC_PATH"
CMD="$CMD --pool-zmq-address $POOL_ZMQ_ADDRESS"
CMD="$CMD --eth-threshold $ETH_THRESHOLD"
CMD="$CMD --percentage-threshold $PERCENTAGE_THRESHOLD"

if [ "$ENABLE_PUBLISHER" = "true" ]; then
    CMD="$CMD --enable-publisher"
fi

if [ "$1" = "--verbose" ]; then
    CMD="$CMD --verbose"
fi

# Change to project root
cd "$PROJECT_ROOT"

# Start the service
echo "Launching signal detector..."
if [ "$1" = "--foreground" ]; then
    # Run in foreground
    exec $CMD
else
    # Run in background
    nohup $CMD > "$LOG_DIR/signal_detector.out" 2>&1 &
    PID=$!
    echo $PID > "$PID_FILE"
    
    # Wait a moment to check if it started successfully
    sleep 2
    if ps -p $PID > /dev/null; then
        echo -e "${GREEN}Signal detector started successfully (PID: $PID)${NC}"
        echo ""
        echo "Monitor logs with:"
        echo "  tail -f $LOG_DIR/signal_engine_full_tx_ipc.log"
        echo ""
        echo "Stop with:"
        echo "  $SCRIPT_DIR/stop_signal_detector.sh"
    else
        echo -e "${RED}Failed to start signal detector${NC}"
        echo "Check logs at: $LOG_DIR/signal_detector.out"
        exit 1
    fi
fi