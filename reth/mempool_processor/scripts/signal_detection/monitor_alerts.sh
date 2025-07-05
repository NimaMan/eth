#!/bin/bash

# Monitor Signal Detector Alerts in Real-Time

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
LOG_DIR="/home/nima/code/crypto/logs/mempool"
LOG_FILE="$LOG_DIR/signal_engine_full_tx_ipc.log"

echo -e "${BLUE}Signal Detector Alert Monitor${NC}"
echo "============================="
echo ""
echo "Monitoring: $LOG_FILE"
echo "Press Ctrl+C to stop"
echo ""

# Function to format alert lines
format_alert() {
    while IFS= read -r line; do
        if echo "$line" | grep -q "SCAM DETECTED"; then
            echo -e "${RED}$line${NC}"
        elif echo "$line" | grep -q "LIQUIDITY WARNING"; then
            echo -e "${YELLOW}$line${NC}"
        elif echo "$line" | grep -q "removeLiquidity"; then
            echo -e "${MAGENTA}$line${NC}"
        elif echo "$line" | grep -q "Performance stats"; then
            echo -e "${GREEN}$line${NC}"
        elif echo "$line" | grep -q "ERROR"; then
            echo -e "${RED}$line${NC}"
        else
            echo "$line"
        fi
    done
}

# Check if log file exists
if [ ! -f "$LOG_FILE" ]; then
    echo -e "${YELLOW}Warning: Log file does not exist yet${NC}"
    echo "Waiting for signal detector to create log file..."
    while [ ! -f "$LOG_FILE" ]; do
        sleep 1
    done
    echo -e "${GREEN}Log file created!${NC}"
    echo ""
fi

# Monitor the log file
if [ "$1" = "--all" ]; then
    # Show all logs with formatting
    tail -f "$LOG_FILE" | format_alert
elif [ "$1" = "--alerts-only" ]; then
    # Show only alerts
    tail -f "$LOG_FILE" | grep -E "(SCAM|WARNING|removeLiquidity|ERROR)" | format_alert
else
    # Default: Show last 20 lines then follow with alerts highlighted
    tail -n 20 "$LOG_FILE" | format_alert
    echo -e "\n${YELLOW}--- Following new alerts ---${NC}\n"
    tail -f "$LOG_FILE" | format_alert
fi