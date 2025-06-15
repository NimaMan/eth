#!/bin/bash

# Real-time Scam Detection Service Startup Script

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BLUE}${BOLD}============================================${NC}"
echo -e "${BLUE}${BOLD}  REAL-TIME SCAM DETECTION SERVICE v2.0${NC}"
echo -e "${BLUE}${BOLD}============================================${NC}\n"

# Configuration
ETH_RPC_URL="${ETH_RPC_URL:-http://127.0.0.1:8545}"
ETH_WS_URL="${ETH_WS_URL:-ws://127.0.0.1:8546}"
POOL_ZMQ_ADDRESS="${POOL_ZMQ_ADDRESS:-tcp://localhost:5557}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-eth_db}"
DB_USER="${DB_USER:-postgres}"
DB_PASSWORD="${DB_PASSWORD:-postgres}"
ETH_THRESHOLD="${ETH_THRESHOLD:-0.01}"
PERCENTAGE_THRESHOLD="${PERCENTAGE_THRESHOLD:-0.95}"

echo -e "${GREEN}🔧 Configuration:${NC}"
echo -e "  RPC URL: ${ETH_RPC_URL}"
echo -e "  WebSocket URL: ${ETH_WS_URL}"
echo -e "  Pool ZMQ: ${POOL_ZMQ_ADDRESS}"
echo -e "  Database: ${DB_USER}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
echo -e "  ETH Threshold: ${ETH_THRESHOLD}"
echo -e "  Percentage Threshold: ${PERCENTAGE_THRESHOLD}"
echo ""

# Check prerequisites
echo -e "${GREEN}🔍 Checking prerequisites...${NC}"

# Check if Reth node is running
if nc -z localhost 8545 2>/dev/null && nc -z localhost 8546 2>/dev/null; then
    echo -e "  ✅ Ethereum node (Reth) is running"
else
    echo -e "  ${RED}❌ Ethereum node not detected on localhost:8545/8546${NC}"
    echo -e "  ${YELLOW}Please start your Reth node first${NC}"
    exit 1
fi

# Check if pool publisher is running
if nc -z localhost 5557 2>/dev/null; then
    echo -e "  ✅ Pool publisher is running"
else
    echo -e "  ${YELLOW}⚠️  Pool publisher not detected on localhost:5557${NC}"
    echo -e "  ${YELLOW}The service will wait for pool data...${NC}"
fi

# Check database connection
if PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "SELECT 1" >/dev/null 2>&1; then
    echo -e "  ✅ Database connection successful"
else
    echo -e "  ${RED}❌ Cannot connect to database${NC}"
    echo -e "  ${YELLOW}Please check your database settings${NC}"
    exit 1
fi

echo ""

# Build the service
echo -e "${GREEN}🔨 Building scam detection service...${NC}"
if cargo build --release --bin realtime_scam_detection_service; then
    echo -e "  ✅ Build successful"
else
    echo -e "  ${RED}❌ Build failed${NC}"
    exit 1
fi

echo ""

# Start the service
echo -e "${GREEN}🚀 Starting real-time scam detection service...${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"

# Run with all parameters
cargo run --release --bin realtime_scam_detection_service -- \
    --eth-rpc-url "$ETH_RPC_URL" \
    --eth-ws-url "$ETH_WS_URL" \
    --pool-zmq-address "$POOL_ZMQ_ADDRESS" \
    --db-host "$DB_HOST" \
    --db-port "$DB_PORT" \
    --db-name "$DB_NAME" \
    --db-user "$DB_USER" \
    --db-password "$DB_PASSWORD" \
    --eth-threshold "$ETH_THRESHOLD" \
    --percentage-threshold "$PERCENTAGE_THRESHOLD" \
    --verbose