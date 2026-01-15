#!/bin/bash
# Start ETH Kartal in development mode

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting ETH Kartal Development Environment${NC}"
echo "================================================"

# Check if Reth is running
if ! pgrep -f "reth" > /dev/null; then
    echo -e "${RED}❌ Reth node is not running!${NC}"
    echo "Please start your local Reth node first."
    exit 1
else
    echo -e "${GREEN}✅ Reth node is running${NC}"
fi

# Check if mempool processor is running
if ! pgrep -f "mempool_signal_detection" > /dev/null; then
    echo -e "${YELLOW}⚠️  Mempool processor is not running${NC}"
    echo "ETH Kartal will wait for alerts..."
else
    echo -e "${GREEN}✅ Mempool processor is running${NC}"
fi

# Create logs directory if it doesn't exist
mkdir -p logs

# Set development environment variables
export ETH_KARTAL_ENVIRONMENT=development
export ETH_KARTAL_LOG_LEVEL=debug
export RUST_BACKTRACE=1
export RUST_LOG=eth_kartal=debug,info

# Check if private key is set (for development)
if [ -z "$ETH_KARTAL_PRIVATE_KEY_DEV" ]; then
    echo -e "${YELLOW}⚠️  ETH_KARTAL_PRIVATE_KEY_DEV not set${NC}"
    echo "Using default development wallet..."
fi

echo ""
echo "Starting ETH Kartal with development configuration..."
echo "Logs: ./logs/kartal_dev.log"
echo "Metrics: http://localhost:9090/metrics"
echo "Health: http://localhost:8080/health"
echo ""

# Run ETH Kartal
cargo run -- --config config/dev.toml