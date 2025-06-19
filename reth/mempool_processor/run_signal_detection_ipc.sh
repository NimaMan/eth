#!/bin/bash

# Script to run the IPC-based mempool signal detection service
# This version uses IPC socket for full transaction data, eliminating HTTP RPC bottleneck

set -e

# Configuration
ETH_RPC_URL="${ETH_RPC_URL:-http://localhost:8545}"
ETH_IPC_PATH="${ETH_IPC_PATH:-/tmp/reth.ipc}"
POOL_ZMQ_ADDRESS="${POOL_ZMQ_ADDRESS:-tcp://localhost:5557}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-eth_db}"
DB_USER="${DB_USER:-postgres}"
DB_PASSWORD="${DB_PASSWORD:-postgres}"

# Detection thresholds
ETH_THRESHOLD="${ETH_THRESHOLD:-0.01}"
PERCENTAGE_THRESHOLD="${PERCENTAGE_THRESHOLD:-0.5}"

# Verbose logging
VERBOSE="${VERBOSE:-false}"

# Build the binary in release mode for performance
echo "🔨 Building mempool signal detection service (IPC version)..."
cargo build --release --bin mempool_signal_detection_ipc

# Check if IPC socket exists
if [ ! -S "$ETH_IPC_PATH" ]; then
    echo "❌ Error: IPC socket not found at $ETH_IPC_PATH"
    echo "   Make sure your Reth node is running with IPC enabled"
    exit 1
fi

echo "✅ IPC socket found at $ETH_IPC_PATH"

# Run the service
echo "🚀 Starting Mempool Signal Detection Service (IPC Version)"
echo "   IPC Socket: $ETH_IPC_PATH"
echo "   RPC URL: $ETH_RPC_URL"
echo "   Database: $DB_NAME@$DB_HOST:$DB_PORT"
echo "   ETH Threshold: $ETH_THRESHOLD ETH"
echo "   Percentage Threshold: ${PERCENTAGE_THRESHOLD}%"

# Run with appropriate flags
if [ "$VERBOSE" = "true" ]; then
    VERBOSE_FLAG="--verbose"
else
    VERBOSE_FLAG=""
fi

exec ./target/release/mempool_signal_detection_ipc \
    --eth-rpc-url "$ETH_RPC_URL" \
    --eth-ipc-path "$ETH_IPC_PATH" \
    --pool-zmq-address "$POOL_ZMQ_ADDRESS" \
    --db-host "$DB_HOST" \
    --db-port "$DB_PORT" \
    --db-name "$DB_NAME" \
    --db-user "$DB_USER" \
    --db-password "$DB_PASSWORD" \
    --eth-threshold "$ETH_THRESHOLD" \
    --percentage-threshold "$PERCENTAGE_THRESHOLD" \
    $VERBOSE_FLAG