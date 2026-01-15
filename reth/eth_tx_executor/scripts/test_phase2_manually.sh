#!/bin/bash
# Manual testing script for Phase 2 implementation
# Tests alert reception, decision making, and transaction building

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== ETH KARTAL PHASE 2 MANUAL TEST ===${NC}"
echo "Testing alert reception → decision → execution flow"
echo ""

# Configuration
TEST_WALLET="0x742d35Cc6634C0532925a3b844Bc9e7595f5CC1a" # Test wallet address
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
MEMPOOL_TOOLS="/home/nima/code/crypto/rust/mempool_processor/tools"

# Check if kartal is built
if [ ! -f "$PROJECT_ROOT/target/debug/kartal" ]; then
    echo -e "${YELLOW}Building ETH Kartal...${NC}"
    cd "$PROJECT_ROOT"
    cargo build
fi

# Function to send test alert
send_test_alert() {
    local severity=$1
    local drain_percent=$2
    local alert_name=$3
    
    echo -e "${YELLOW}Sending $alert_name alert (${severity}, ${drain_percent}% drain)...${NC}"
    
    python3 - <<EOF
import zmq
import json
import time

context = zmq.Context()
socket = context.socket(zmq.PUB)
socket.bind("tcp://127.0.0.1:5559")
time.sleep(1)  # Let socket bind

alert = {
    "alert_id": f"manual_test_{int(time.time())}",
    "timestamp": int(time.time() * 1000),
    "severity": "$severity",
    "event_type": "ScamAlert",
    "tx_hash": "0x" + "1" * 64,
    "detected_latency_us": 5000,
    "pool_address": "0x" + "2" * 40,
    "pool_version": "V2",
    "token_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  # USDC
    "token_symbol": "USDC",
    "token_decimals": 6,
    "current_eth_reserve": 100.0,
    "simulated_eth_reserve": 100.0 * (1 + $drain_percent / 100),
    "eth_change_amount": 100.0 * $drain_percent / 100,
    "eth_change_percent": $drain_percent,
    "current_price": 1.0,
    "simulated_price": 0.1,
    "price_impact_percent": -90.0,
    "confidence_score": 0.95,
    "gas_price_gwei": 30.0,
    "details": "Manual test alert: $alert_name"
}

socket.send_json(alert)
print(f"✅ Alert sent: {alert['alert_id']}")

socket.close()
context.term()
EOF
}

# Test 1: Start kartal in test mode
echo -e "${BLUE}Test 1: Starting ETH Kartal in test mode${NC}"

# Create a log file for this test run
LOG_FILE="/tmp/kartal_test_$(date +%Y%m%d_%H%M%S).log"

# Start kartal in background
RUST_LOG=eth_kartal=debug "$PROJECT_ROOT/target/debug/kartal" \
    --wallet-address "$TEST_WALLET" \
    --test-mode \
    > "$LOG_FILE" 2>&1 &

KARTAL_PID=$!
echo "Started ETH Kartal (PID: $KARTAL_PID)"
echo "Logs: $LOG_FILE"

# Wait for startup
sleep 3

# Check if kartal is still running
if ! kill -0 $KARTAL_PID 2>/dev/null; then
    echo -e "${RED}❌ ETH Kartal failed to start!${NC}"
    echo "Last 20 lines of log:"
    tail -20 "$LOG_FILE"
    exit 1
fi

echo -e "${GREEN}✅ ETH Kartal started successfully${NC}"
echo ""

# Test 2: Send different severity alerts
echo -e "${BLUE}Test 2: Sending test alerts${NC}"

# Emergency sell alert (95% drain)
send_test_alert "Critical" -95 "Emergency"
sleep 2

# Check log for emergency decision
if grep -q "EMERGENCY SELL" "$LOG_FILE"; then
    echo -e "${GREEN}✅ Emergency sell decision made correctly${NC}"
else
    echo -e "${RED}❌ Emergency sell decision NOT found${NC}"
fi

# Partial sell alert (55% drain)
send_test_alert "High" -55 "Partial"
sleep 2

# Check log for partial decision
if grep -q "PARTIAL SELL" "$LOG_FILE"; then
    echo -e "${GREEN}✅ Partial sell decision made correctly${NC}"
else
    echo -e "${RED}❌ Partial sell decision NOT found${NC}"
fi

# Monitor alert (30% drain)
send_test_alert "Medium" -30 "Monitor"
sleep 2

# Check log for monitor decision
if grep -q "MONITORING" "$LOG_FILE"; then
    echo -e "${GREEN}✅ Monitor decision made correctly${NC}"
else
    echo -e "${RED}❌ Monitor decision NOT found${NC}"
fi

echo ""

# Test 3: Verify test mode (no real transactions)
echo -e "${BLUE}Test 3: Verifying test mode behavior${NC}"

if grep -q "TEST MODE: Would sell" "$LOG_FILE"; then
    echo -e "${GREEN}✅ Test mode working - no real transactions${NC}"
else
    echo -e "${YELLOW}⚠️  Could not verify test mode behavior${NC}"
fi

# Test 4: Check performance
echo -e "${BLUE}Test 4: Checking alert processing performance${NC}"

# Extract processing times from logs (if logged)
# This is a placeholder - actual implementation would need timing logs
echo -e "${YELLOW}Performance metrics would be extracted from timing logs${NC}"

echo ""

# Test 5: Stress test with multiple alerts
echo -e "${BLUE}Test 5: Stress testing with rapid alerts${NC}"

for i in {1..10}; do
    send_test_alert "High" -60 "Stress_$i" &
done
wait

sleep 3

# Count processed alerts
PROCESSED_COUNT=$(grep -c "Processing alert" "$LOG_FILE" || true)
echo -e "Processed $PROCESSED_COUNT alerts"

if [ "$PROCESSED_COUNT" -ge 10 ]; then
    echo -e "${GREEN}✅ All stress test alerts processed${NC}"
else
    echo -e "${YELLOW}⚠️  Only $PROCESSED_COUNT/10+ alerts processed${NC}"
fi

echo ""

# Summary
echo -e "${BLUE}=== TEST SUMMARY ===${NC}"
echo "Log file: $LOG_FILE"
echo ""

# Check for errors
ERROR_COUNT=$(grep -c "ERROR" "$LOG_FILE" || true)
WARN_COUNT=$(grep -c "WARN" "$LOG_FILE" || true)

echo "Errors found: $ERROR_COUNT"
echo "Warnings found: $WARN_COUNT"
echo ""

# Show decision summary
echo "Decision Summary:"
echo "- Emergency sells: $(grep -c "EMERGENCY SELL" "$LOG_FILE" || true)"
echo "- Partial sells: $(grep -c "PARTIAL SELL" "$LOG_FILE" || true)"
echo "- Monitor only: $(grep -c "MONITORING" "$LOG_FILE" || true)"
echo "- Skipped: $(grep -c "SKIP" "$LOG_FILE" || true)"

echo ""

# Cleanup
echo -e "${YELLOW}Stopping ETH Kartal...${NC}"
kill $KARTAL_PID 2>/dev/null || true
wait $KARTAL_PID 2>/dev/null || true

echo -e "${GREEN}✅ Test complete${NC}"
echo ""
echo "To view full logs:"
echo "  less $LOG_FILE"
echo ""
echo "To run with real alerts from mempool processor:"
echo "  1. Start mempool processor with ZMQ enabled"
echo "  2. Run: $PROJECT_ROOT/target/debug/kartal --wallet-address $TEST_WALLET --test-mode"