#!/bin/bash
# Test ETH Kartal with live alerts from mempool processor

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== ETH KARTAL LIVE ALERT TEST ===${NC}"
echo "Testing with real alerts from mempool processor"
echo ""

# Check if mempool processor is running
if pgrep -f "mempool_signal_detection" > /dev/null; then
    echo -e "${GREEN}✅ Mempool processor is running${NC}"
else
    echo -e "${RED}❌ Mempool processor not found!${NC}"
    echo "Please start the mempool processor first"
    exit 1
fi

# Check if publishing on 5559
if lsof -i :5559 | grep -q LISTEN; then
    echo -e "${GREEN}✅ ZMQ publisher active on port 5559${NC}"
else
    echo -e "${YELLOW}⚠️  No listener on port 5559${NC}"
fi

echo ""
echo -e "${BLUE}Starting ETH Kartal in test mode...${NC}"

# Start kartal with live output
RUST_LOG=eth_kartal=info cargo run --bin kartal -- \
    --wallet-address 0x742d35Cc6634C0532925a3b844Bc9e7595f5CC1a \
    --test-mode 2>&1 | while IFS= read -r line; do
    
    # Color code the output
    if [[ "$line" == *"SCAM DETECTED"* ]]; then
        echo -e "${RED}$line${NC}"
    elif [[ "$line" == *"DECISION:"* ]]; then
        echo -e "${GREEN}$line${NC}"
    elif [[ "$line" == *"Processing alert"* ]]; then
        echo -e "${YELLOW}$line${NC}"
    elif [[ "$line" == *"No position"* ]] || [[ "$line" == *"Skip"* ]]; then
        echo -e "${BLUE}$line${NC}"
    elif [[ "$line" == *"TEST MODE"* ]]; then
        echo -e "${YELLOW}$line${NC}"
    else
        echo "$line"
    fi
done