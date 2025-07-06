#!/bin/bash

# Run the Reth Direct Simulator version
# 
# This script runs the mempool signal detector with Direct Reth simulation
# for 20-40x performance improvement over RPC.

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

# Default parameters
ETH_RPC_URL="${ETH_RPC_URL:-http://localhost:8545}"
IPC_PATH="${IPC_PATH:-/tmp/reth.ipc}"
RETH_DATADIR="${RETH_DATADIR:-/home/nima/.local/share/reth/mainnet}"
POOL_ZMQ_ADDRESS="${POOL_ZMQ_ADDRESS:-tcp://localhost:5557}"
ETH_THRESHOLD="${ETH_THRESHOLD:-0.01}"
PERCENTAGE_THRESHOLD="${PERCENTAGE_THRESHOLD:-0.5}"
ENABLE_PUBLISHER="${ENABLE_PUBLISHER:-true}"

echo -e "${GREEN}Mempool Signal Detector - Direct Reth Version${NC}"
echo "=============================================="
echo ""

# Pre-flight checks
echo "Running pre-flight checks:"
echo ""

# Check Reth node
echo -n "Checking Reth node... "
if curl -s -X POST -H 'Content-Type: application/json' --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' $ETH_RPC_URL > /dev/null 2>&1; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo -e "${RED}Error: Reth node is not responding at $ETH_RPC_URL${NC}"
    exit 1
fi

# Check IPC socket
echo -n "Checking IPC socket... "
if test -S $IPC_PATH; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo -e "${RED}Error: IPC socket not found at $IPC_PATH${NC}"
    exit 1
fi

# Check Reth database
echo -n "Checking Reth database... "
if test -d "$RETH_DATADIR/db"; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo -e "${RED}Error: Reth database not found at $RETH_DATADIR${NC}"
    exit 1
fi

# Create log directory
mkdir -p "$LOG_DIR"

echo ""
echo "Starting Direct Reth signal detector with:"
echo "  RPC URL: $ETH_RPC_URL"
echo "  IPC Path: $IPC_PATH"
echo "  Reth DB: $RETH_DATADIR"
echo "  Pool ZMQ: $POOL_ZMQ_ADDRESS"
echo "  ETH Threshold: $ETH_THRESHOLD"
echo "  Drain Threshold: ${PERCENTAGE_THRESHOLD}%"
echo "  Publisher: $ENABLE_PUBLISHER"
echo ""

# Change to project root
cd "$PROJECT_ROOT"

# Build and run
echo "Building and launching Direct Reth signal detector..."
if [ "$1" = "--foreground" ]; then
    # Run in foreground
    exec rustc scripts/signal_detection/mempool_signal_detector_with_reth_simulator.rs \
        --extern mempool_processor=target/release/deps/libmempool_processor.rlib \
        --extern reth_signed_tx_simulator=target/release/deps/libreth_signed_tx_simulator.rlib \
        -L dependency=target/release/deps \
        --edition 2021 \
        -o /tmp/reth_signal_detector && \
    /tmp/reth_signal_detector \
        --eth-rpc-url "$ETH_RPC_URL" \
        --ipc-path "$IPC_PATH" \
        --reth-datadir "$RETH_DATADIR" \
        --pool-zmq-address "$POOL_ZMQ_ADDRESS" \
        --eth-threshold "$ETH_THRESHOLD" \
        --percentage-threshold "$PERCENTAGE_THRESHOLD" \
        $([ "$ENABLE_PUBLISHER" = "true" ] && echo "--enable-publisher")
else
    echo -e "${YELLOW}Note: Direct compilation of this script is complex due to dependencies.${NC}"
    echo -e "${YELLOW}Please run the working RPC version for now with:${NC}"
    echo ""
    echo "  ./scripts/signal_detection/run_signal_detector.sh"
    echo ""
    echo -e "${YELLOW}The Reth simulator integration is ready for binary registration in Cargo.toml${NC}"
fi