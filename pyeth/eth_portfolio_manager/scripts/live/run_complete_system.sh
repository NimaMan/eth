#!/bin/bash
# run_complete_system.sh
#
# Run both the Python pool tracking and Rust mempool processor components together
# This script demonstrates how to run the complete system for testing and deployment

# Exit on error
set -e

# Configuration variables (modify as needed)
ETHEREUM_NODE_HTTP="http://localhost:8545"
ETHEREUM_NODE_WS="ws://localhost:8546"
ETH_RESERVE_THRESHOLD=0.05
WARMUP_BLOCKS=1200
ENABLE_DB_LOGGING=true
PG_HOST="localhost"
PG_PORT=5432
PG_DBNAME="eth_db"
PG_USER="postgres"
PG_PASSWORD="postgres"
ZMQ_PUB_ENDPOINT="tcp://*:5557"
ZMQ_REP_ENDPOINT="tcp://*:5558"
ZMQ_SUB_ENDPOINT="tcp://localhost:5557"
ZMQ_REQ_ENDPOINT="tcp://localhost:5558"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# ASCII art header
echo -e "${GREEN}"
echo "===================================================================================="
echo "  ______ _______ _     _    _____  _____  _____   _____ _______ _____  _____"
echo " |  ____|__   __| |   | |  |  __ \|  __ \|  __ \ / ____|__   __|  __ \|  __ \\"
echo " | |__     | |  | |___| |  | |__) | |__) | |  | | |       | |  | |__) | |  | |"
echo " |  __|    | |  |  ___  |  |  ___/|  ___/| |  | | |       | |  |  _  /| |  | |"
echo " | |____   | |  | |   | |  | |    | |    | |__| | |____   | |  | | \ \| |__| |"
echo " |______|  |_|  |_|   |_|  |_|    |_|    |_____/ \_____|  |_|  |_|  \_\_____/"
echo "                                                                    "
echo "===================================================================================="
echo -e "${NC}"
echo -e "${YELLOW}Starting complete Ethereum Mempool Monitoring System${NC}"
echo ""

# Export environment variables
export ETH_NODE_URL="$ETHEREUM_NODE_HTTP"
export WS_RPC_URL="$ETHEREUM_NODE_WS"
export HTTP_RPC_URL="$ETHEREUM_NODE_HTTP"
export ETH_RESERVE_THRESHOLD="$ETH_RESERVE_THRESHOLD"
export ENABLE_DB_LOGGING="$ENABLE_DB_LOGGING"
export PG_HOST="$PG_HOST"
export PG_PORT="$PG_PORT"
export PG_DBNAME="$PG_DBNAME"
export PG_USER="$PG_USER"
export PG_PASSWORD="$PG_PASSWORD"

# Function to clean up processes on exit
cleanup() {
    echo -e "${YELLOW}Stopping all processes...${NC}"
    
    # Kill background processes
    if [ -n "$RUST_PID" ]; then
        echo "Stopping Rust mempool processor (PID: $RUST_PID)..."
        kill -TERM "$RUST_PID" 2>/dev/null || true
    fi
    
    if [ -n "$PYTHON_PID" ]; then
        echo "Stopping Python pool tracker (PID: $PYTHON_PID)..."
        kill -TERM "$PYTHON_PID" 2>/dev/null || true
    fi
    
    echo -e "${GREEN}Cleanup complete${NC}"
}

# Register the cleanup function for script exit
trap cleanup EXIT INT TERM

# Determine script directory path
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
RUST_DIR="$PROJECT_ROOT/rust/mempool_processor"
PYTHON_DIR="$PROJECT_ROOT/py/eth_portfolio_manager"

echo -e "${YELLOW}Project directories:${NC}"
echo "Script directory: $SCRIPT_DIR"
echo "Project root: $PROJECT_ROOT"
echo "Rust directory: $RUST_DIR"
echo "Python directory: $PYTHON_DIR"
echo ""

# Check if Ethereum node is available
echo -e "${YELLOW}Checking Ethereum node connection...${NC}"
if curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' "$ETHEREUM_NODE_HTTP" > /dev/null; then
    echo -e "${GREEN}Ethereum node is available at $ETHEREUM_NODE_HTTP${NC}"
else
    echo -e "${RED}Cannot connect to Ethereum node at $ETHEREUM_NODE_HTTP${NC}"
    echo "Please check that your Ethereum node is running"
    exit 1
fi

# Start Rust mempool processor in background
echo -e "${YELLOW}Starting Rust mempool processor...${NC}"
cd "$RUST_DIR"
cargo run --bin mempool_processor -- \
    --ws-rpc-url "$ETHEREUM_NODE_WS" \
    --http-rpc-url "$ETHEREUM_NODE_HTTP" \
    --eth-reserve-threshold "$ETH_RESERVE_THRESHOLD" \
    --verbose \
    $([ "$ENABLE_DB_LOGGING" = true ] && echo "--enable-db-logging") \
    --pg-host "$PG_HOST" \
    --pg-port "$PG_PORT" \
    --pg-dbname "$PG_DBNAME" \
    --pg-user "$PG_USER" \
    --pg-password "$PG_PASSWORD" &

RUST_PID=$!
echo -e "${GREEN}Rust mempool processor started with PID: $RUST_PID${NC}"

# Wait a moment to allow Rust to start and bind to ZMQ
sleep 2

# Start Python pool tracking in background
echo -e "${YELLOW}Starting Python pool tracking...${NC}"
cd "$PYTHON_DIR"
python -m scripts.live.run_live_portfolio_with_pools \
    --warmup "$WARMUP_BLOCKS" \
    --pub-endpoint "$ZMQ_PUB_ENDPOINT" \
    --rep-endpoint "$ZMQ_REP_ENDPOINT" &

PYTHON_PID=$!
echo -e "${GREEN}Python pool tracking started with PID: $PYTHON_PID${NC}"

echo ""
echo -e "${GREEN}Complete system is now running!${NC}"
echo "Press Ctrl+C to stop all components"

# Wait for both processes
wait $RUST_PID $PYTHON_PID 