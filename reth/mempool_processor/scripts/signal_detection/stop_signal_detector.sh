#!/bin/bash

# Stop the Mempool Signal Detector

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
LOG_DIR="/home/nima/code/crypto/logs/mempool"
PID_FILE="$LOG_DIR/signal_detector.pid"

echo -e "${YELLOW}Stopping Mempool Signal Detector${NC}"
echo "================================"
echo ""

if [ ! -f "$PID_FILE" ]; then
    echo -e "${YELLOW}No PID file found at $PID_FILE${NC}"
    echo "Signal detector may not be running"
    
    # Try to find it anyway
    PIDS=$(pgrep -f "mempool_signal_detector" || true)
    if [ ! -z "$PIDS" ]; then
        echo "Found signal detector process(es): $PIDS"
        echo -n "Kill these processes? (y/n): "
        read answer
        if [ "$answer" = "y" ]; then
            kill $PIDS
            echo -e "${GREEN}Processes terminated${NC}"
        fi
    else
        echo "No signal detector processes found"
    fi
    exit 0
fi

PID=$(cat "$PID_FILE")
echo "Found PID: $PID"

if ps -p $PID > /dev/null 2>&1; then
    echo "Sending SIGTERM to process $PID..."
    kill $PID
    
    # Wait for graceful shutdown
    for i in {1..10}; do
        if ! ps -p $PID > /dev/null 2>&1; then
            echo -e "${GREEN}Signal detector stopped successfully${NC}"
            rm -f "$PID_FILE"
            exit 0
        fi
        echo -n "."
        sleep 1
    done
    
    echo ""
    echo -e "${YELLOW}Process still running, sending SIGKILL...${NC}"
    kill -9 $PID
    sleep 1
    
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "${RED}Failed to stop signal detector${NC}"
        exit 1
    else
        echo -e "${GREEN}Signal detector forcefully stopped${NC}"
        rm -f "$PID_FILE"
    fi
else
    echo -e "${YELLOW}Process $PID is not running${NC}"
    echo "Removing stale PID file"
    rm -f "$PID_FILE"
fi